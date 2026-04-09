use logbog_core::RawLogLine;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tracing::info;

use crate::LogSender;

/// Configuration for the OTLP HTTP log receiver.
#[derive(Debug, Clone)]
pub struct OtlpReceiverConfig {
    pub http_port: u16,
    pub pack: String,
    pub source: String,
}

impl Default for OtlpReceiverConfig {
    fn default() -> Self {
        Self {
            http_port: 4318,
            pack: "otlp".into(),
            source: "otlp-http".into(),
        }
    }
}

/// Simplified OTLP HTTP log receiver.
///
/// Accepts newline-delimited JSON log records over HTTP POST.
/// This is a basic implementation — full OTLP protobuf support
/// will be added in a future version with tonic/prost.
pub struct OtlpReceiver {
    sender: LogSender,
    config: OtlpReceiverConfig,
}

impl OtlpReceiver {
    pub fn new(sender: LogSender, config: OtlpReceiverConfig) -> Self {
        Self { sender, config }
    }

    /// Start the HTTP receiver.
    pub async fn run(self) -> logbog_core::Result<()> {
        let bind_addr = format!("0.0.0.0:{}", self.config.http_port);
        let listener = TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| logbog_core::Error::Collector(format!("OTLP bind {bind_addr}: {e}")))?;

        info!(port = self.config.http_port, "OTLP HTTP receiver started");

        loop {
            let (stream, addr) = listener
                .accept()
                .await
                .map_err(|e| logbog_core::Error::Collector(format!("OTLP accept: {e}")))?;

            let sender = self.sender.clone();
            let pack = self.config.pack.clone();
            let source = self.config.source.clone();

            tokio::spawn(async move {
                let reader = BufReader::new(stream);
                let mut lines = reader.lines();

                // Read HTTP request body as newline-delimited JSON
                let mut in_body = false;
                while let Ok(Some(line)) = lines.next_line().await {
                    // Simple HTTP parsing: skip headers, read body
                    if line.is_empty() {
                        in_body = true;
                        continue;
                    }
                    if !in_body {
                        continue;
                    }
                    if line.is_empty() {
                        continue;
                    }

                    let raw = RawLogLine {
                        source: source.clone(),
                        content: line,
                        pack: pack.clone(),
                        file_path: None,
                    };
                    if sender.send(raw).await.is_err() {
                        break;
                    }
                }

                info!(addr = %addr, "OTLP client disconnected");
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otlp_config_default() {
        let config = OtlpReceiverConfig::default();
        assert_eq!(config.http_port, 4318);
        assert_eq!(config.pack, "otlp");
    }
}
