use logbog_core::RawLogLine;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, UdpSocket};
use tracing::{info, warn};

use crate::LogSender;

/// Configuration for the syslog receiver.
#[derive(Debug, Clone)]
pub struct SyslogReceiverConfig {
    pub udp_port: u16,
    pub tcp_port: u16,
    pub pack: String,
    pub source: String,
}

impl Default for SyslogReceiverConfig {
    fn default() -> Self {
        Self {
            udp_port: 5514,
            tcp_port: 5514,
            pack: "syslog".into(),
            source: "syslog-receiver".into(),
        }
    }
}

/// Receives syslog messages over UDP and TCP.
pub struct SyslogReceiver {
    sender: LogSender,
    config: SyslogReceiverConfig,
}

impl SyslogReceiver {
    pub fn new(sender: LogSender, config: SyslogReceiverConfig) -> Self {
        Self { sender, config }
    }

    /// Start both UDP and TCP listeners.
    pub async fn run(self) -> logbog_core::Result<()> {
        let udp_sender = self.sender.clone();
        let udp_config = self.config.clone();

        let tcp_sender = self.sender;
        let tcp_config = self.config;

        let udp_handle = tokio::spawn(async move {
            if let Err(e) = run_udp(udp_sender, &udp_config).await {
                warn!(error = %e, "UDP syslog receiver stopped");
            }
        });

        let tcp_handle = tokio::spawn(async move {
            if let Err(e) = run_tcp(tcp_sender, &tcp_config).await {
                warn!(error = %e, "TCP syslog receiver stopped");
            }
        });

        // Wait for both to finish (they normally run forever)
        let _ = tokio::try_join!(udp_handle, tcp_handle);

        Ok(())
    }
}

async fn run_udp(sender: LogSender, config: &SyslogReceiverConfig) -> logbog_core::Result<()> {
    let bind_addr = format!("0.0.0.0:{}", config.udp_port);
    let socket = UdpSocket::bind(&bind_addr)
        .await
        .map_err(|e| logbog_core::Error::Collector(format!("UDP bind {bind_addr}: {e}")))?;

    info!(port = config.udp_port, "Syslog UDP receiver started");

    let mut buf = vec![0u8; 65535];

    loop {
        let (len, _addr) = socket
            .recv_from(&mut buf)
            .await
            .map_err(|e| logbog_core::Error::Collector(format!("UDP recv: {e}")))?;

        let message = String::from_utf8_lossy(&buf[..len]).to_string();
        for line in message.lines() {
            if line.is_empty() {
                continue;
            }
            let raw = RawLogLine {
                source: config.source.clone(),
                content: line.to_string(),
                pack: config.pack.clone(),
                file_path: None,
            };
            if sender.send(raw).await.is_err() {
                return Ok(());
            }
        }
    }
}

async fn run_tcp(sender: LogSender, config: &SyslogReceiverConfig) -> logbog_core::Result<()> {
    let bind_addr = format!("0.0.0.0:{}", config.tcp_port);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| logbog_core::Error::Collector(format!("TCP bind {bind_addr}: {e}")))?;

    info!(port = config.tcp_port, "Syslog TCP receiver started");

    loop {
        let (stream, addr) = listener
            .accept()
            .await
            .map_err(|e| logbog_core::Error::Collector(format!("TCP accept: {e}")))?;

        let sender = sender.clone();
        let pack = config.pack.clone();
        let source = config.source.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stream);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
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

            info!(addr = %addr, "TCP syslog client disconnected");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syslog_config_default() {
        let config = SyslogReceiverConfig::default();
        assert_eq!(config.udp_port, 5514);
        assert_eq!(config.tcp_port, 5514);
        assert_eq!(config.pack, "syslog");
    }

    #[tokio::test]
    async fn test_udp_syslog_receiver() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let config = SyslogReceiverConfig {
            udp_port: 0, // OS-assigned port
            tcp_port: 0,
            pack: "syslog".into(),
            source: "test".into(),
        };

        // Bind to random port for test
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = socket.local_addr().unwrap().port();
        drop(socket);

        let test_config = SyslogReceiverConfig {
            udp_port: port,
            ..config
        };

        let sender = tx.clone();
        let cfg = test_config.clone();
        let handle = tokio::spawn(async move {
            run_udp(sender, &cfg).await.ok();
        });

        // Give the receiver time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Send a test message
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client
            .send_to(
                b"<34>Apr  9 10:15:30 web-01 sshd[12345]: Test message",
                format!("127.0.0.1:{port}"),
            )
            .await
            .unwrap();

        // Wait for the message
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(msg.content.contains("Test message"));
        assert_eq!(msg.pack, "syslog");

        handle.abort();
    }
}
