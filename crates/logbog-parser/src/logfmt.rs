use crate::{ParsedLog, Parser};
use logbog_core::LogLevel;
use std::collections::HashMap;

/// Parseur pour le format logfmt (key=value pairs).
/// Exemple : `ts=2026-04-09T10:00:00Z level=error msg="connection refused" host=web-01`
pub struct LogfmtParser {
    pub timestamp_field: String,
    pub level_field: String,
    pub message_field: String,
}

impl Default for LogfmtParser {
    fn default() -> Self {
        Self {
            timestamp_field: "ts".into(),
            level_field: "level".into(),
            message_field: "msg".into(),
        }
    }
}

impl Parser for LogfmtParser {
    fn parse(&self, raw: &str) -> Option<ParsedLog> {
        let pairs = parse_logfmt(raw);
        if pairs.is_empty() {
            return None;
        }

        let timestamp = pairs.get(&self.timestamp_field).and_then(|v| {
            chrono::DateTime::parse_from_rfc3339(v)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        let level = pairs
            .get(&self.level_field)
            .map(|v| LogLevel::from_str_loose(v));

        let message = pairs.get(&self.message_field).cloned().unwrap_or_default();

        let fields: HashMap<String, serde_json::Value> = pairs
            .into_iter()
            .filter(|(k, _)| {
                k != &self.timestamp_field && k != &self.level_field && k != &self.message_field
            })
            .map(|(k, v)| {
                let json_value = if let Ok(n) = v.parse::<i64>() {
                    serde_json::Value::Number(n.into())
                } else if let Ok(f) = v.parse::<f64>() {
                    serde_json::json!(f)
                } else if v == "true" {
                    serde_json::Value::Bool(true)
                } else if v == "false" {
                    serde_json::Value::Bool(false)
                } else {
                    serde_json::Value::String(v)
                };
                (k, json_value)
            })
            .collect();

        Some(ParsedLog {
            timestamp,
            level,
            message,
            fields,
        })
    }

    fn format_name(&self) -> &str {
        "logfmt"
    }
}

/// Parse une chaîne logfmt en paires clé-valeur.
fn parse_logfmt(input: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut chars = input.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        // Read key
        let key: String = chars.by_ref().take_while(|c| *c != '=').collect();
        if key.is_empty() {
            break;
        }

        // Read value
        let value = if chars.peek() == Some(&'"') {
            chars.next(); // skip opening quote
            let mut val = String::new();
            loop {
                match chars.next() {
                    Some('"') | None => break,
                    Some('\\') => {
                        if let Some(next) = chars.next() {
                            val.push(next);
                        }
                    }
                    Some(c) => val.push(c),
                }
            }
            val
        } else {
            let mut val = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                val.push(c);
                chars.next();
            }
            val
        };

        result.insert(key, value);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_logfmt_basic() {
        let parser = LogfmtParser::default();
        let line =
            r#"ts=2026-04-09T10:00:00Z level=error msg="connection refused" host=web-01 port=5432"#;
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
    fn test_parse_logfmt_empty() {
        let parser = LogfmtParser::default();
        assert!(parser.parse("").is_none());
    }

    #[test]
    fn test_parse_logfmt_booleans() {
        let parser = LogfmtParser::default();
        let line = "level=info msg=hello active=true count=42";
        let parsed = parser.parse(line).unwrap();
        assert_eq!(
            parsed.fields.get("active"),
            Some(&serde_json::Value::Bool(true))
        );
    }
}
