use crate::{ParsedLog, Parser};
use logbog_core::LogLevel;
use regex::Regex;
use std::collections::HashMap;

/// Parseur basé sur des expressions régulières avec des groupes nommés.
pub struct RegexParser {
    pub patterns: Vec<Regex>,
    pub timestamp_field: String,
    pub timestamp_format: Option<String>,
    pub level_field: String,
    pub message_field: String,
}

impl RegexParser {
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        let re = Regex::new(pattern)?;
        Ok(Self {
            patterns: vec![re],
            timestamp_field: "timestamp".into(),
            timestamp_format: None,
            level_field: "level".into(),
            message_field: "message".into(),
        })
    }

    pub fn with_timestamp_format(mut self, format: impl Into<String>) -> Self {
        self.timestamp_format = Some(format.into());
        self
    }
}

impl Parser for RegexParser {
    fn parse(&self, raw: &str) -> Option<ParsedLog> {
        for pattern in &self.patterns {
            if let Some(caps) = pattern.captures(raw) {
                let mut fields = HashMap::new();

                // Extraire tous les groupes nommés
                for name in pattern.capture_names().flatten() {
                    if let Some(m) = caps.name(name) {
                        let value = m.as_str();
                        // Tenter de parser comme nombre
                        let json_value = if let Ok(n) = value.parse::<i64>() {
                            serde_json::Value::Number(n.into())
                        } else if let Ok(f) = value.parse::<f64>() {
                            serde_json::json!(f)
                        } else {
                            serde_json::Value::String(value.to_string())
                        };
                        fields.insert(name.to_string(), json_value);
                    }
                }

                let timestamp = caps
                    .name(&self.timestamp_field)
                    .and_then(|m| parse_timestamp(m.as_str(), self.timestamp_format.as_deref()));

                let level = caps
                    .name(&self.level_field)
                    .map(|m| LogLevel::from_str_loose(m.as_str()));

                let message = caps
                    .name(&self.message_field)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| raw.to_string());

                return Some(ParsedLog {
                    timestamp,
                    level,
                    message,
                    fields,
                });
            }
        }
        None
    }

    fn format_name(&self) -> &str {
        "regex"
    }
}

/// Tente de parser un timestamp avec différents formats.
fn parse_timestamp(s: &str, format: Option<&str>) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::{DateTime, NaiveDateTime, Utc};

    if let Some(fmt) = format {
        // Essayer avec timezone
        if let Ok(dt) = DateTime::parse_from_str(s, fmt) {
            return Some(dt.with_timezone(&Utc));
        }
        // Essayer sans timezone (assume UTC)
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(dt.and_utc());
        }
    }

    // Fallback : formats courants
    let common_formats = [
        "%Y-%m-%dT%H:%M:%S%.fZ",
        "%Y-%m-%d %H:%M:%S",
        "%d/%b/%Y:%H:%M:%S %z",
        "%Y/%m/%d %H:%M:%S",
        "%b %d %H:%M:%S",
    ];

    for fmt in &common_formats {
        if let Ok(dt) = DateTime::parse_from_str(s, fmt) {
            return Some(dt.with_timezone(&Utc));
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(dt.and_utc());
        }
    }

    // RFC 3339
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_parser_nginx_error() {
        let parser = RegexParser::new(
            r"(?P<timestamp>\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}) \[(?P<level>\w+)\] (?P<pid>\d+)#(?P<tid>\d+): \*(?P<cid>\d+) (?P<message>.*)",
        )
        .unwrap()
        .with_timestamp_format("%Y/%m/%d %H:%M:%S");

        let line =
            "2026/04/09 10:15:30 [error] 1234#0: *5678 upstream timed out, client: 192.168.1.1";
        let parsed = parser.parse(line).unwrap();
        assert_eq!(parsed.level, Some(LogLevel::Error));
        assert!(parsed.message.contains("upstream timed out"));
        assert!(parsed.timestamp.is_some());
        assert_eq!(parsed.fields.get("pid"), Some(&serde_json::json!(1234)));
    }

    #[test]
    fn test_regex_parser_no_match() {
        let parser = RegexParser::new(r"(?P<level>ERROR|WARN) (?P<message>.*)").unwrap();
        assert!(parser.parse("this does not match").is_none());
    }

    #[test]
    fn test_regex_parser_numeric_fields() {
        let parser = RegexParser::new(r"status=(?P<status>\d+) time=(?P<time>\d+\.\d+)").unwrap();
        let parsed = parser.parse("status=200 time=0.042").unwrap();
        assert_eq!(parsed.fields.get("status"), Some(&serde_json::json!(200)));
        assert_eq!(parsed.fields.get("time"), Some(&serde_json::json!(0.042)));
    }

    #[test]
    fn test_parse_timestamp_rfc3339() {
        let ts = parse_timestamp("2026-04-09T10:00:00Z", None);
        assert!(ts.is_some());
    }

    #[test]
    fn test_parse_timestamp_custom_format() {
        let ts = parse_timestamp("2026/04/09 10:00:00", Some("%Y/%m/%d %H:%M:%S"));
        assert!(ts.is_some());
    }
}
