use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ulid::Ulid;

/// Niveau de sévérité d'un log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Fatal => "fatal",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "trace" => Self::Trace,
            "debug" => Self::Debug,
            "info" | "notice" => Self::Info,
            "warn" | "warning" => Self::Warn,
            "error" | "err" | "crit" | "critical" => Self::Error,
            "fatal" | "emerg" | "emergency" | "alert" | "panic" => Self::Fatal,
            _ => Self::Info,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Entrée de log normalisée — le type central de LogBog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Identifiant unique (ULID = ordonné par temps).
    pub id: Ulid,
    /// Horodatage de l'événement.
    pub timestamp: DateTime<Utc>,
    /// Source du log (ex: "nginx", "php-fpm").
    pub source: String,
    /// Hôte d'origine.
    pub host: String,
    /// Niveau de sévérité.
    pub level: LogLevel,
    /// Message principal.
    pub message: String,
    /// Champs structurés extraits par le parseur.
    pub fields: HashMap<String, serde_json::Value>,
    /// Ligne brute originale.
    pub raw: String,
    /// Nom du pack qui a produit cette entrée.
    pub pack: String,
}

impl LogEntry {
    pub fn new(source: impl Into<String>, pack: impl Into<String>) -> Self {
        Self {
            id: Ulid::new(),
            timestamp: Utc::now(),
            source: source.into(),
            host: hostname(),
            level: LogLevel::Info,
            message: String::new(),
            fields: HashMap::new(),
            raw: String::new(),
            pack: pack.into(),
        }
    }
}

/// Ligne de log brute avant parsing.
#[derive(Debug, Clone)]
pub struct RawLogLine {
    /// Source du log.
    pub source: String,
    /// Contenu brut de la ligne.
    pub content: String,
    /// Nom du pack associé.
    pub pack: String,
    /// Chemin du fichier source (si applicable).
    pub file_path: Option<String>,
}

/// Position dans une source de log (pour le bookmarking).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcePosition {
    /// Chemin du fichier.
    pub path: String,
    /// Offset en bytes.
    pub offset: u64,
    /// Inode du fichier (pour détecter la rotation).
    pub inode: Option<u64>,
}

/// Corrélation entre plusieurs entrées de log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correlation {
    pub id: Ulid,
    pub entry_ids: Vec<Ulid>,
    pub confidence: f64,
    pub correlation_type: CorrelationType,
    pub description: String,
    pub timestamp: DateTime<Utc>,
}

/// Type de corrélation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrelationType {
    /// Même requête HTTP à travers les services.
    RequestChain,
    /// Même processus / PID.
    ProcessGroup,
    /// Proximité temporelle.
    Temporal,
    /// Relation causale (ex: OOM → crash).
    Causal,
}

/// Statut du service LogBog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub running: bool,
    pub uptime_seconds: Option<u64>,
    pub packs_installed: Vec<String>,
    pub packs_active: Vec<String>,
    pub logs_ingested: u64,
    pub storage_bytes: u64,
    pub version: String,
}

impl Default for ServiceStatus {
    fn default() -> Self {
        Self {
            running: false,
            uptime_seconds: None,
            packs_installed: Vec::new(),
            packs_active: Vec::new(),
            logs_ingested: 0,
            storage_bytes: 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Récupère le hostname de la machine.
fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| {
            std::fs::read_to_string("/etc/hostname")
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str_loose() {
        assert_eq!(LogLevel::from_str_loose("ERROR"), LogLevel::Error);
        assert_eq!(LogLevel::from_str_loose("warning"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str_loose("crit"), LogLevel::Error);
        assert_eq!(LogLevel::from_str_loose("emerg"), LogLevel::Fatal);
        assert_eq!(LogLevel::from_str_loose("notice"), LogLevel::Info);
        assert_eq!(LogLevel::from_str_loose("garbage"), LogLevel::Info);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Error.to_string(), "error");
        assert_eq!(LogLevel::Warn.to_string(), "warn");
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Fatal);
    }

    #[test]
    fn test_log_entry_new() {
        let entry = LogEntry::new("nginx", "nginx");
        assert_eq!(entry.source, "nginx");
        assert_eq!(entry.pack, "nginx");
        assert_eq!(entry.level, LogLevel::Info);
        assert!(entry.fields.is_empty());
    }

    #[test]
    fn test_service_status_default() {
        let status = ServiceStatus::default();
        assert!(!status.running);
        assert!(status.packs_installed.is_empty());
        assert_eq!(status.logs_ingested, 0);
    }

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry::new("nginx", "nginx");
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: LogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source, "nginx");
        assert_eq!(deserialized.pack, "nginx");
    }
}
