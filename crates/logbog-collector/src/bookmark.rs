use logbog_core::SourcePosition;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::warn;

/// Manages file read positions across restarts.
///
/// Bookmarks are stored as a JSON file so LogBog can resume
/// reading from where it left off after a restart.
pub struct BookmarkManager {
    state_file: PathBuf,
    positions: HashMap<String, SourcePosition>,
}

impl BookmarkManager {
    /// Create a new bookmark manager with the given state file path.
    pub fn new(state_file: PathBuf) -> Self {
        let mut mgr = Self {
            state_file,
            positions: HashMap::new(),
        };
        mgr.load();
        mgr
    }

    /// Get the saved position for a file.
    pub fn get(&self, path: &Path) -> Option<&SourcePosition> {
        let key = path.to_string_lossy().to_string();
        self.positions.get(&key)
    }

    /// Set the position for a file.
    pub fn set(&mut self, path: &Path, position: SourcePosition) {
        let key = path.to_string_lossy().to_string();
        self.positions.insert(key, position);
    }

    /// Remove bookmark for a file.
    pub fn remove(&mut self, path: &Path) {
        let key = path.to_string_lossy().to_string();
        self.positions.remove(&key);
    }

    /// Load bookmarks from the state file.
    fn load(&mut self) {
        if !self.state_file.exists() {
            return;
        }
        match std::fs::read_to_string(&self.state_file) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(positions) => {
                    self.positions = positions;
                }
                Err(e) => {
                    warn!(
                        path = %self.state_file.display(),
                        error = %e,
                        "Failed to parse bookmarks file, starting fresh"
                    );
                }
            },
            Err(e) => {
                warn!(
                    path = %self.state_file.display(),
                    error = %e,
                    "Failed to read bookmarks file"
                );
            }
        }
    }

    /// Persist bookmarks to the state file.
    pub fn persist(&self) -> logbog_core::Result<()> {
        if let Some(parent) = self.state_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.positions)?;
        std::fs::write(&self.state_file, json)?;
        Ok(())
    }

    /// Number of tracked files.
    pub fn count(&self) -> usize {
        self.positions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_set_get() {
        let tmp = tempfile::tempdir().unwrap();
        let state_file = tmp.path().join("bookmarks.json");
        let mut mgr = BookmarkManager::new(state_file);

        let path = Path::new("/var/log/nginx/access.log");
        assert!(mgr.get(path).is_none());

        mgr.set(
            path,
            SourcePosition {
                path: path.to_string_lossy().to_string(),
                offset: 12345,
                inode: Some(67890),
            },
        );

        let pos = mgr.get(path).unwrap();
        assert_eq!(pos.offset, 12345);
        assert_eq!(pos.inode, Some(67890));
    }

    #[test]
    fn test_bookmark_persist_and_reload() {
        let tmp = tempfile::tempdir().unwrap();
        let state_file = tmp.path().join("bookmarks.json");

        // Write
        {
            let mut mgr = BookmarkManager::new(state_file.clone());
            mgr.set(
                Path::new("/var/log/test.log"),
                SourcePosition {
                    path: "/var/log/test.log".into(),
                    offset: 999,
                    inode: Some(42),
                },
            );
            mgr.persist().unwrap();
        }

        // Reload
        {
            let mgr = BookmarkManager::new(state_file);
            let pos = mgr.get(Path::new("/var/log/test.log")).unwrap();
            assert_eq!(pos.offset, 999);
            assert_eq!(pos.inode, Some(42));
        }
    }

    #[test]
    fn test_bookmark_remove() {
        let tmp = tempfile::tempdir().unwrap();
        let state_file = tmp.path().join("bookmarks.json");
        let mut mgr = BookmarkManager::new(state_file);

        let path = Path::new("/var/log/test.log");
        mgr.set(
            path,
            SourcePosition {
                path: path.to_string_lossy().to_string(),
                offset: 100,
                inode: None,
            },
        );
        assert_eq!(mgr.count(), 1);

        mgr.remove(path);
        assert_eq!(mgr.count(), 0);
        assert!(mgr.get(path).is_none());
    }

    #[test]
    fn test_bookmark_missing_state_file() {
        let tmp = tempfile::tempdir().unwrap();
        let state_file = tmp.path().join("nonexistent/bookmarks.json");
        let mgr = BookmarkManager::new(state_file);
        assert_eq!(mgr.count(), 0);
    }
}
