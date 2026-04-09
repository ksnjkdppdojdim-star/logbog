use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use logbog_core::RawLogLine;
use notify::{Config as NotifyConfig, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, info, warn};

use crate::bookmark::BookmarkManager;
use crate::LogSender;

/// Tracks a single watched file.
struct TrackedFile {
    pack: String,
    source: String,
    offset: u64,
    inode: Option<u64>,
}

/// Watches log files for changes and sends new lines through a channel.
///
/// Handles log rotation (rename/truncate detection) and bookmarking
/// (remembers the last read position across restarts).
pub struct FileWatcher {
    sender: LogSender,
    bookmarks: BookmarkManager,
    files: HashMap<PathBuf, TrackedFile>,
}

impl FileWatcher {
    pub fn new(sender: LogSender, bookmarks: BookmarkManager) -> Self {
        Self {
            sender,
            bookmarks,
            files: HashMap::new(),
        }
    }

    /// Register a file to watch. Resolves the path and loads any existing bookmark.
    pub fn add_file(&mut self, path: PathBuf, pack: String, source: String) {
        let canonical = path
            .canonicalize()
            .unwrap_or_else(|_| path.clone());

        let offset = self
            .bookmarks
            .get(&canonical)
            .map(|p| p.offset)
            .unwrap_or(0);

        let inode = get_inode(&canonical);

        debug!(
            path = %canonical.display(),
            pack = %pack,
            source = %source,
            offset = offset,
            "Tracking file"
        );

        self.files.insert(
            canonical,
            TrackedFile {
                pack,
                source,
                offset,
                inode,
            },
        );
    }

    /// Expand glob patterns from pack sources and register matching files.
    pub fn add_glob(&mut self, pattern: &str, pack: String, source: String) {
        match glob::glob(pattern) {
            Ok(paths) => {
                for entry in paths.flatten() {
                    if entry.is_file() {
                        self.add_file(entry, pack.clone(), source.clone());
                    }
                }
            }
            Err(e) => {
                warn!(pattern = %pattern, error = %e, "Invalid glob pattern");
                // Try as a literal path
                let path = PathBuf::from(pattern);
                if path.exists() && path.is_file() {
                    self.add_file(path, pack, source);
                }
            }
        }
    }

    /// Start watching files. Runs until the channel is closed.
    pub async fn run(mut self) -> logbog_core::Result<()> {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                event_tx.send(res).ok();
            },
            NotifyConfig::default(),
        )
        .map_err(|e| logbog_core::Error::Collector(format!("Failed to create watcher: {e}")))?;

        // Watch parent directories of all tracked files
        let mut watched_dirs = HashSet::new();
        for path in self.files.keys() {
            if let Some(parent) = path.parent() {
                if parent.exists() && watched_dirs.insert(parent.to_path_buf()) {
                    watcher
                        .watch(parent, RecursiveMode::NonRecursive)
                        .map_err(|e| {
                            logbog_core::Error::Collector(format!(
                                "Failed to watch {}: {e}",
                                parent.display()
                            ))
                        })?;
                    info!(dir = %parent.display(), "Watching directory");
                }
            }
        }

        // Initial catch-up: read from last known offset to current end
        let paths: Vec<PathBuf> = self.files.keys().cloned().collect();
        for path in paths {
            if let Err(e) = self.read_new_lines(&path).await {
                warn!(path = %path.display(), error = %e, "Failed initial catch-up");
            }
        }

        info!(files = self.files.len(), "File watcher started");

        // Event loop
        while let Some(result) = event_rx.recv().await {
            match result {
                Ok(event) => {
                    if matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_)
                    ) {
                        for path in &event.paths {
                            let canonical = path
                                .canonicalize()
                                .unwrap_or_else(|_| path.clone());
                            if self.files.contains_key(&canonical) {
                                if let Err(e) = self.read_new_lines(&canonical).await {
                                    warn!(
                                        path = %canonical.display(),
                                        error = %e,
                                        "Failed to read new lines"
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "File watcher error");
                }
            }
        }

        // Persist bookmarks on exit
        if let Err(e) = self.bookmarks.persist() {
            warn!(error = %e, "Failed to persist bookmarks");
        }

        Ok(())
    }

    /// Read new lines from a tracked file since the last known offset.
    async fn read_new_lines(&mut self, path: &Path) -> logbog_core::Result<()> {
        let tracked = match self.files.get_mut(path) {
            Some(t) => t,
            None => return Ok(()),
        };

        if !path.exists() {
            return Ok(());
        }

        let metadata = std::fs::metadata(path)?;
        let current_inode = get_inode(path);
        let file_size = metadata.len();

        // Detect log rotation: inode changed or file shrank
        if tracked.inode.is_some() && current_inode != tracked.inode {
            info!(path = %path.display(), "Log rotation detected (inode changed)");
            tracked.offset = 0;
            tracked.inode = current_inode;
        } else if file_size < tracked.offset {
            info!(path = %path.display(), "Log rotation detected (file truncated)");
            tracked.offset = 0;
        }

        // Nothing new to read
        if file_size == tracked.offset {
            return Ok(());
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::Start(tracked.offset))?;

        let pack = tracked.pack.clone();
        let source = tracked.source.clone();
        let file_path_str = path.to_string_lossy().to_string();

        let mut line = String::new();
        let mut lines_read = 0u64;

        while reader.read_line(&mut line)? > 0 {
            // Only process complete lines (ending with newline)
            if line.ends_with('\n') {
                let content = line.trim_end().to_string();
                if !content.is_empty() {
                    let raw = RawLogLine {
                        source: source.clone(),
                        content,
                        pack: pack.clone(),
                        file_path: Some(file_path_str.clone()),
                    };
                    if self.sender.send(raw).await.is_err() {
                        return Err(logbog_core::Error::Collector(
                            "Log channel closed".into(),
                        ));
                    }
                    lines_read += 1;
                }
            }
            line.clear();
        }

        // Update offset
        let new_offset = reader.stream_position()?;
        tracked.offset = new_offset;
        tracked.inode = current_inode;

        // Save bookmark
        self.bookmarks.set(
            path,
            logbog_core::SourcePosition {
                path: file_path_str,
                offset: new_offset,
                inode: current_inode,
            },
        );

        if lines_read > 0 {
            debug!(
                path = %path.display(),
                lines = lines_read,
                offset = new_offset,
                "Read new lines"
            );
        }

        Ok(())
    }
}

/// Get the inode of a file (Unix only).
fn get_inode(path: &Path) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(path).ok().map(|m| m.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tokio::sync::mpsc;

    #[test]
    fn test_get_inode() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let inode = get_inode(tmp.path());
        #[cfg(unix)]
        assert!(inode.is_some());
        #[cfg(not(unix))]
        assert!(inode.is_none());
    }

    #[tokio::test]
    async fn test_add_file_and_read() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let log_file = tmp_dir.path().join("test.log");

        // Write initial content
        {
            let mut f = File::create(&log_file).unwrap();
            writeln!(f, "line 1").unwrap();
            writeln!(f, "line 2").unwrap();
        }

        let (tx, mut rx) = mpsc::channel(100);
        let bookmarks = BookmarkManager::new(tmp_dir.path().join("bookmarks.json"));

        let mut watcher = FileWatcher::new(tx, bookmarks);
        watcher.add_file(log_file.clone(), "test".into(), "main".into());

        // Read the existing lines
        watcher.read_new_lines(&log_file.canonicalize().unwrap()).await.unwrap();

        let line1 = rx.try_recv().unwrap();
        assert_eq!(line1.content, "line 1");
        assert_eq!(line1.pack, "test");
        assert_eq!(line1.source, "main");

        let line2 = rx.try_recv().unwrap();
        assert_eq!(line2.content, "line 2");

        // No more lines
        assert!(rx.try_recv().is_err());

        // Append more content
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&log_file)
                .unwrap();
            writeln!(f, "line 3").unwrap();
        }

        watcher.read_new_lines(&log_file.canonicalize().unwrap()).await.unwrap();

        let line3 = rx.try_recv().unwrap();
        assert_eq!(line3.content, "line 3");
    }

    #[tokio::test]
    async fn test_log_rotation_detection() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let log_file = tmp_dir.path().join("app.log");

        // Write initial content
        {
            let mut f = File::create(&log_file).unwrap();
            writeln!(f, "old line 1").unwrap();
            writeln!(f, "old line 2").unwrap();
        }

        let (tx, mut rx) = mpsc::channel(100);
        let bookmarks = BookmarkManager::new(tmp_dir.path().join("bookmarks.json"));

        let mut watcher = FileWatcher::new(tx, bookmarks);
        watcher.add_file(log_file.clone(), "test".into(), "main".into());

        let canonical = log_file.canonicalize().unwrap();

        // Read initial content
        watcher.read_new_lines(&canonical).await.unwrap();
        let _ = rx.try_recv().unwrap(); // old line 1
        let _ = rx.try_recv().unwrap(); // old line 2

        // Simulate truncation (log rotation)
        {
            let mut f = File::create(&log_file).unwrap(); // truncates
            writeln!(f, "new line 1").unwrap();
        }

        watcher.read_new_lines(&canonical).await.unwrap();

        let new_line = rx.try_recv().unwrap();
        assert_eq!(new_line.content, "new line 1");
    }
}
