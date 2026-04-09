//! Moteur de parsing universel pour LogBog.
//!
//! Supporte les formats : regex, JSON, logfmt, syslog (RFC 3164 & 5424).

pub mod json;
pub mod logfmt;
pub mod regex_parser;
pub mod syslog;

use logbog_core::{LogEntry, LogLevel, RawLogLine};
use std::collections::HashMap;

/// Résultat d'un parsing.
#[derive(Debug, Clone)]
pub struct ParsedLog {
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub level: Option<LogLevel>,
    pub message: String,
    pub fields: HashMap<String, serde_json::Value>,
}

/// Trait commun à tous les parseurs.
pub trait Parser: Send + Sync {
    /// Parse une ligne brute en log structuré.
    fn parse(&self, raw: &str) -> Option<ParsedLog>;

    /// Nom du format de ce parseur.
    fn format_name(&self) -> &str;
}

/// Convertit un ParsedLog en LogEntry complet.
pub fn to_log_entry(parsed: ParsedLog, raw_line: &RawLogLine) -> LogEntry {
    let mut entry = LogEntry::new(&raw_line.source, &raw_line.pack);
    if let Some(ts) = parsed.timestamp {
        entry.timestamp = ts;
    }
    if let Some(level) = parsed.level {
        entry.level = level;
    }
    entry.message = parsed.message;
    entry.fields = parsed.fields;
    entry.raw = raw_line.content.clone();
    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_log_entry() {
        let parsed = ParsedLog {
            timestamp: None,
            level: Some(LogLevel::Error),
            message: "test error".into(),
            fields: HashMap::new(),
        };
        let raw = RawLogLine {
            source: "test".into(),
            content: "raw line".into(),
            pack: "test-pack".into(),
            file_path: None,
        };
        let entry = to_log_entry(parsed, &raw);
        assert_eq!(entry.level, LogLevel::Error);
        assert_eq!(entry.message, "test error");
        assert_eq!(entry.raw, "raw line");
        assert_eq!(entry.pack, "test-pack");
    }
}
