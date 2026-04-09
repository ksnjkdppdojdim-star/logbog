use logbog_core::RawLogLine;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{info, warn};

use crate::LogSender;

/// Reads logs from the systemd journal via `journalctl --output=json`.
pub struct JournalReader {
    sender: LogSender,
    units: Vec<String>,
    pack: String,
}

impl JournalReader {
    /// Create a new journal reader.
    ///
    /// If `units` is empty, reads all journal entries.
    /// Otherwise, filters by the specified systemd units.
    pub fn new(sender: LogSender, units: Vec<String>, pack: String) -> Self {
        Self {
            sender,
            units,
            pack,
        }
    }

    /// Start reading the journal. Runs until the process exits or the channel closes.
    pub async fn run(self) -> logbog_core::Result<()> {
        let mut cmd = Command::new("journalctl");
        cmd.arg("--output=json").arg("--follow").arg("--no-pager");

        // Filter by units if specified
        for unit in &self.units {
            cmd.arg("--unit").arg(unit);
        }

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());

        let mut child = cmd.spawn().map_err(|e| {
            logbog_core::Error::Collector(format!("Failed to spawn journalctl: {e}"))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            logbog_core::Error::Collector("Failed to capture journalctl stdout".into())
        })?;

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        info!(
            units = ?self.units,
            "Journal reader started"
        );

        while let Ok(Some(line)) = lines.next_line().await {
            if line.is_empty() {
                continue;
            }

            let raw = RawLogLine {
                source: "journal".to_string(),
                content: line,
                pack: self.pack.clone(),
                file_path: None,
            };

            if self.sender.send(raw).await.is_err() {
                break;
            }
        }

        // Wait for the child process to exit
        match child.wait().await {
            Ok(status) => {
                if !status.success() {
                    warn!(code = ?status.code(), "journalctl exited with error");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to wait for journalctl");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journal_reader_creation() {
        let (tx, _rx) = tokio::sync::mpsc::channel(10);
        let reader = JournalReader::new(
            tx,
            vec!["nginx.service".into(), "php-fpm.service".into()],
            "systemd".into(),
        );
        assert_eq!(reader.units.len(), 2);
        assert_eq!(reader.pack, "systemd");
    }
}
