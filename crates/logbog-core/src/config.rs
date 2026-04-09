use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Configuration principale de LogBog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub collector: CollectorConfig,
    pub correlation: CorrelationConfig,
    pub intelligence: IntelligenceConfig,
    pub alerting: AlertingConfig,
    pub compliance: ComplianceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub engine: StorageEngine,
    pub retention_days: u32,
    pub max_size_gb: u32,
    pub s3: Option<S3Config>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageEngine {
    DuckDb,
    Parquet,
    Federated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    pub otlp: OtlpConfig,
    pub syslog: SyslogConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpConfig {
    pub enabled: bool,
    pub grpc_port: u16,
    pub http_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyslogConfig {
    pub enabled: bool,
    pub udp_port: u16,
    pub tcp_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationConfig {
    pub enabled: bool,
    pub window_seconds: u32,
    pub min_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceConfig {
    pub anomaly_detection: bool,
    pub error_clustering: bool,
    pub learning_window_days: u32,
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    pub enabled: bool,
    pub channels: Vec<AlertChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AlertChannel {
    Webhook {
        url: String,
    },
    Email {
        smtp_host: String,
        smtp_port: u16,
        from: String,
        to: Vec<String>,
    },
    Slack {
        webhook_url: String,
    },
    Telegram {
        bot_token: String,
        chat_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceConfig {
    pub pii_detection: bool,
    pub pii_masking: PiiMaskingMode,
    pub audit_log: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PiiMaskingMode {
    Redact,
    Hash,
    Pseudonymize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".into(),
                port: 6060,
                data_dir: PathBuf::from("/var/lib/logbog"),
            },
            storage: StorageConfig {
                engine: StorageEngine::DuckDb,
                retention_days: 30,
                max_size_gb: 10,
                s3: None,
            },
            collector: CollectorConfig {
                batch_size: 1000,
                flush_interval_ms: 500,
                otlp: OtlpConfig {
                    enabled: false,
                    grpc_port: 4317,
                    http_port: 4318,
                },
                syslog: SyslogConfig {
                    enabled: false,
                    udp_port: 5514,
                    tcp_port: 5514,
                },
            },
            correlation: CorrelationConfig {
                enabled: true,
                window_seconds: 2,
                min_confidence: 0.7,
            },
            intelligence: IntelligenceConfig {
                anomaly_detection: true,
                error_clustering: true,
                learning_window_days: 7,
                llm: LlmConfig {
                    enabled: false,
                    provider: "ollama".into(),
                    model: "llama3".into(),
                    endpoint: "http://localhost:11434".into(),
                },
            },
            alerting: AlertingConfig {
                enabled: true,
                channels: Vec::new(),
            },
            compliance: ComplianceConfig {
                pii_detection: false,
                pii_masking: PiiMaskingMode::Redact,
                audit_log: false,
            },
        }
    }
}

impl Config {
    /// Charge la config depuis un fichier TOML.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(Error::ConfigNotFound(path.display().to_string()));
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Sauvegarde la config dans un fichier TOML.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Retourne le chemin par défaut du fichier de config.
    pub fn default_path() -> PathBuf {
        PathBuf::from("logbog.toml")
    }

    /// Cherche le fichier de config dans les chemins standards.
    pub fn find() -> Option<PathBuf> {
        let mut candidates = vec![
            PathBuf::from("logbog.toml"),
            PathBuf::from("/etc/logbog/logbog.toml"),
        ];
        if let Some(config_dir) = dirs::config_dir() {
            candidates.push(config_dir.join("logbog/logbog.toml"));
        }
        candidates.into_iter().find(|p: &PathBuf| p.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.port, 6060);
        assert_eq!(config.storage.retention_days, 30);
        assert!(config.correlation.enabled);
        assert!(!config.intelligence.llm.enabled);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.server.port, config.server.port);
        assert_eq!(parsed.storage.retention_days, config.storage.retention_days);
    }

    #[test]
    fn test_config_save_and_load() {
        let config = Config::default();
        let mut tmp = NamedTempFile::new().unwrap();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        tmp.write_all(toml_str.as_bytes()).unwrap();

        let loaded = Config::load(tmp.path()).unwrap();
        assert_eq!(loaded.server.port, 6060);
    }

    #[test]
    fn test_config_load_not_found() {
        let result = Config::load(Path::new("/nonexistent/logbog.toml"));
        assert!(result.is_err());
    }
}
