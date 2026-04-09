use crate::{ParsedLog, Parser};
use logbog_core::LogLevel;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Parseur syslog supportant RFC 3164 et RFC 5424.
pub struct SyslogParser {
    pub rfc: SyslogRfc,
}

#[derive(Debug, Clone, Copy)]
pub enum SyslogRfc {
    Rfc3164,
    Rfc5424,
    Auto,
}

impl Default for SyslogParser {
    fn default() -> Self {
        Self {
            rfc: SyslogRfc::Auto,
        }
    }
}

// RFC 3164: <PRI>Mon DD HH:MM:SS hostname app[pid]: message
static RE_3164: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:<(?P<pri>\d+)>)?(?P<timestamp>\w{3}\s+\d+\s+\d{2}:\d{2}:\d{2})\s+(?P<hostname>\S+)\s+(?P<app>\S+?)(?:\[(?P<pid>\d+)\])?:\s*(?P<message>.*)",
    )
    .unwrap()
});

// RFC 5424: <PRI>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID STRUCTURED-DATA MSG
static RE_5424: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"<(?P<pri>\d+)>(?P<version>\d+)\s+(?P<timestamp>\S+)\s+(?P<hostname>\S+)\s+(?P<app>\S+)\s+(?P<procid>\S+)\s+(?P<msgid>\S+)\s+(?P<sd>(?:\[.*?\])+|-)\s*(?P<message>.*)",
    )
    .unwrap()
});

impl Parser for SyslogParser {
    fn parse(&self, raw: &str) -> Option<ParsedLog> {
        match self.rfc {
            SyslogRfc::Rfc3164 => parse_rfc3164(raw),
            SyslogRfc::Rfc5424 => parse_rfc5424(raw),
            SyslogRfc::Auto => parse_rfc5424(raw).or_else(|| parse_rfc3164(raw)),
        }
    }

    fn format_name(&self) -> &str {
        match self.rfc {
            SyslogRfc::Rfc3164 => "syslog-3164",
            SyslogRfc::Rfc5424 => "syslog-5424",
            SyslogRfc::Auto => "syslog-auto",
        }
    }
}

fn parse_rfc3164(raw: &str) -> Option<ParsedLog> {
    let caps = RE_3164.captures(raw)?;
    let mut fields = HashMap::new();

    if let Some(hostname) = caps.name("hostname") {
        fields.insert(
            "hostname".into(),
            serde_json::Value::String(hostname.as_str().into()),
        );
    }
    if let Some(app) = caps.name("app") {
        fields.insert("app".into(), serde_json::Value::String(app.as_str().into()));
    }
    if let Some(n) = caps
        .name("pid")
        .and_then(|p| p.as_str().parse::<i64>().ok())
    {
        fields.insert("pid".into(), serde_json::json!(n));
    }

    let level = caps
        .name("pri")
        .and_then(|m| m.as_str().parse::<u8>().ok())
        .map(priority_to_level)
        .unwrap_or(LogLevel::Info);

    let message = caps
        .name("message")
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();

    Some(ParsedLog {
        timestamp: None, // syslog 3164 timestamps lack year, defer to collection time
        level: Some(level),
        message,
        fields,
    })
}

fn parse_rfc5424(raw: &str) -> Option<ParsedLog> {
    let caps = RE_5424.captures(raw)?;
    let mut fields = HashMap::new();

    if let Some(hostname) = caps.name("hostname") {
        fields.insert(
            "hostname".into(),
            serde_json::Value::String(hostname.as_str().into()),
        );
    }
    if let Some(app) = caps.name("app") {
        fields.insert("app".into(), serde_json::Value::String(app.as_str().into()));
    }
    if let Some(procid) = caps.name("procid") {
        fields.insert(
            "procid".into(),
            serde_json::Value::String(procid.as_str().into()),
        );
    }
    if let Some(msgid) = caps.name("msgid") {
        fields.insert(
            "msgid".into(),
            serde_json::Value::String(msgid.as_str().into()),
        );
    }

    let timestamp = caps.name("timestamp").and_then(|m| {
        chrono::DateTime::parse_from_rfc3339(m.as_str())
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });

    let level = caps
        .name("pri")
        .and_then(|m| m.as_str().parse::<u8>().ok())
        .map(priority_to_level)
        .unwrap_or(LogLevel::Info);

    let message = caps
        .name("message")
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();

    Some(ParsedLog {
        timestamp,
        level: Some(level),
        message,
        fields,
    })
}

/// Convertit une priorité syslog en LogLevel.
/// Priority = facility * 8 + severity
fn priority_to_level(pri: u8) -> LogLevel {
    let severity = pri % 8;
    match severity {
        0 => LogLevel::Fatal, // Emergency
        1 => LogLevel::Fatal, // Alert
        2 => LogLevel::Error, // Critical
        3 => LogLevel::Error, // Error
        4 => LogLevel::Warn,  // Warning
        5 => LogLevel::Info,  // Notice
        6 => LogLevel::Info,  // Informational
        7 => LogLevel::Debug, // Debug
        _ => LogLevel::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rfc3164() {
        let parser = SyslogParser {
            rfc: SyslogRfc::Rfc3164,
        };
        let line = "<34>Apr  9 10:15:30 web-01 sshd[12345]: Failed password for root from 192.168.1.100 port 22 ssh2";
        let parsed = parser.parse(line).unwrap();
        assert_eq!(parsed.level, Some(LogLevel::Error)); // pri 34 = facility 4, severity 2 (critical)
        assert!(parsed.message.contains("Failed password"));
        assert_eq!(
            parsed.fields.get("app"),
            Some(&serde_json::Value::String("sshd".into()))
        );
        assert_eq!(parsed.fields.get("pid"), Some(&serde_json::json!(12345)));
    }

    #[test]
    fn test_parse_rfc5424() {
        let parser = SyslogParser {
            rfc: SyslogRfc::Rfc5424,
        };
        let line =
            "<165>1 2026-04-09T10:15:30.000Z web-01 myapp 1234 ID47 - Connection established";
        let parsed = parser.parse(line).unwrap();
        assert_eq!(parsed.level, Some(LogLevel::Info)); // pri 165 = facility 20, severity 5 (notice)
        assert!(parsed.timestamp.is_some());
        assert_eq!(parsed.message, "Connection established");
    }

    #[test]
    fn test_parse_auto() {
        let parser = SyslogParser::default();
        let rfc3164 = "<13>Apr  9 10:15:30 web-01 kernel: Out of memory";
        let parsed = parser.parse(rfc3164).unwrap();
        assert!(parsed.message.contains("Out of memory"));
    }

    #[test]
    fn test_priority_to_level() {
        assert_eq!(priority_to_level(0), LogLevel::Fatal); // emergency
        assert_eq!(priority_to_level(3), LogLevel::Error); // error
        assert_eq!(priority_to_level(4), LogLevel::Warn); // warning
        assert_eq!(priority_to_level(6), LogLevel::Info); // info
        assert_eq!(priority_to_level(7), LogLevel::Debug); // debug
    }
}
