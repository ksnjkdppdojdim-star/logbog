use crate::{ParsedLog, Parser};
use logbog_core::LogLevel;
use std::collections::HashMap;

/// Parseur pour les logs au format JSON.
pub struct JsonParser {
    /// Mapping des noms de champs JSON vers les champs LogBog.
    /// Ex: {"severity" => "level", "msg" => "message"}
    pub field_mappings: HashMap<String, String>,
    pub timestamp_field: String,
    pub level_field: String,
    pub message_field: String,
}

impl Default for JsonParser {
    fn default() -> Self {
        Self {
            field_mappings: HashMap::new(),
            timestamp_field: "timestamp".into(),
            level_field: "level".into(),
            message_field: "message".into(),
        }
    }
}

impl Parser for JsonParser {
    fn parse(&self, raw: &str) -> Option<ParsedLog> {
        let value: serde_json::Value = serde_json::from_str(raw).ok()?;
        let obj = value.as_object()?;

        let timestamp = obj
            .get(&self.timestamp_field)
            .and_then(|v| v.as_str())
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            });

        let level = obj
            .get(&self.level_field)
            .and_then(|v| v.as_str())
            .map(LogLevel::from_str_loose);

        let message = obj
            .get(&self.message_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut fields: HashMap<String, serde_json::Value> = obj
            .iter()
            .filter(|(k, _)| {
                *k != &self.timestamp_field && *k != &self.level_field && *k != &self.message_field
            })
            .map(|(k, v)| {
                let key = self
                    .field_mappings
                    .get(k)
                    .cloned()
                    .unwrap_or_else(|| k.clone());
                (key, v.clone())
            })
            .collect();

        // Ajouter le message aussi dans les fields pour la recherche
        if !message.is_empty() {
            fields.insert("message".into(), serde_json::Value::String(message.clone()));
        }

        Some(ParsedLog {
            timestamp,
            level,
            message,
            fields,
        })
    }

    fn format_name(&self) -> &str {
        "json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_log() {
        let parser = JsonParser::default();
        let line = r#"{"timestamp":"2026-04-09T10:00:00Z","level":"error","message":"connection refused","host":"web-01","port":5432}"#;
        let parsed = parser.parse(line).unwrap();
        assert_eq!(parsed.level, Some(LogLevel::Error));
        assert_eq!(parsed.message, "connection refused");
        assert!(parsed.timestamp.is_some());
        assert_eq!(
            parsed.fields.get("host"),
            Some(&serde_json::Value::String("web-01".into()))
        );
        assert_eq!(parsed.fields.get("port"), Some(&serde_json::json!(5432)));
    }

    #[test]
    fn test_parse_invalid_json() {
        let parser = JsonParser::default();
        assert!(parser.parse("not json at all").is_none());
    }

    #[test]
    fn test_parse_json_minimal() {
        let parser = JsonParser::default();
        let line = r#"{"message":"hello"}"#;
        let parsed = parser.parse(line).unwrap();
        assert_eq!(parsed.message, "hello");
        assert!(parsed.timestamp.is_none());
        assert!(parsed.level.is_none());
    }
}
