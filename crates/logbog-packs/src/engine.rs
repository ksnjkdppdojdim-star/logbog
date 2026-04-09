use std::collections::HashMap;

use logbog_core::{LogEntry, RawLogLine};
use logbog_parser::json::JsonParser;
use logbog_parser::logfmt::LogfmtParser;
use logbog_parser::regex_parser::RegexParser;
use logbog_parser::syslog::{SyslogParser, SyslogRfc};
use logbog_parser::{to_log_entry, Parser};

use crate::manifest::{PackManifest, PackSource};

/// A configured parser for a specific source within a pack.
pub struct SourceParser {
    pub pack_name: String,
    pub source_name: String,
    pub parser: Box<dyn Parser>,
    pub paths: Vec<String>,
    pub multiline: bool,
    pub multiline_start: Option<regex::Regex>,
}

/// Engine that wires pack manifests to their parsers.
///
/// Given a pack name and source name, it can parse raw log lines into
/// structured `LogEntry` records using the correct parser configuration.
pub struct PackEngine {
    /// pack_name -> list of source parsers
    sources: HashMap<String, Vec<SourceParser>>,
}

impl PackEngine {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    /// Load a pack manifest and create parsers for all its sources.
    pub fn load_pack(&mut self, manifest: &PackManifest) -> logbog_core::Result<()> {
        let pack_name = &manifest.pack.name;
        let mut source_parsers = Vec::new();

        for source in &manifest.sources {
            let parser = create_parser(source)?;

            let multiline_re = if source.multiline {
                source
                    .multiline_start
                    .as_ref()
                    .map(|p| {
                        regex::Regex::new(p).map_err(|e| {
                            logbog_core::Error::PackInvalid(format!(
                                "source '{}' has invalid multiline_start regex: {e}",
                                source.name
                            ))
                        })
                    })
                    .transpose()?
            } else {
                None
            };

            source_parsers.push(SourceParser {
                pack_name: pack_name.clone(),
                source_name: source.name.clone(),
                parser,
                paths: source.paths.clone(),
                multiline: source.multiline,
                multiline_start: multiline_re,
            });
        }

        self.sources.insert(pack_name.clone(), source_parsers);
        Ok(())
    }

    /// Parse a raw log line using the specified pack and source.
    pub fn parse_line(&self, raw: &str, pack: &str, source: &str) -> Option<LogEntry> {
        let sources = self.sources.get(pack)?;
        let sp = sources.iter().find(|s| s.source_name == source)?;
        let parsed = sp.parser.parse(raw)?;
        let raw_line = RawLogLine {
            source: source.to_string(),
            content: raw.to_string(),
            pack: pack.to_string(),
            file_path: None,
        };
        Some(to_log_entry(parsed, &raw_line))
    }

    /// Parse a raw log line, trying all sources for the given pack.
    pub fn parse_line_any_source(&self, raw: &str, pack: &str) -> Option<(String, LogEntry)> {
        let sources = self.sources.get(pack)?;
        for sp in sources {
            if let Some(parsed) = sp.parser.parse(raw) {
                let raw_line = RawLogLine {
                    source: sp.source_name.clone(),
                    content: raw.to_string(),
                    pack: pack.to_string(),
                    file_path: None,
                };
                return Some((sp.source_name.clone(), to_log_entry(parsed, &raw_line)));
            }
        }
        None
    }

    /// Get all file paths that should be watched, with their pack and source names.
    /// Returns `(path_pattern, pack_name, source_name)` tuples.
    pub fn all_watch_paths(&self) -> Vec<(String, String, String)> {
        let mut paths = Vec::new();
        for (pack_name, sources) in &self.sources {
            for sp in sources {
                for path in &sp.paths {
                    paths.push((path.clone(), pack_name.clone(), sp.source_name.clone()));
                }
            }
        }
        paths
    }

    /// Get a source parser by pack and source name.
    pub fn get_source(&self, pack: &str, source: &str) -> Option<&SourceParser> {
        self.sources
            .get(pack)?
            .iter()
            .find(|s| s.source_name == source)
    }

    /// Check if a source supports multiline parsing.
    pub fn is_multiline(&self, pack: &str, source: &str) -> bool {
        self.get_source(pack, source)
            .is_some_and(|s| s.multiline)
    }

    /// Number of loaded packs.
    pub fn pack_count(&self) -> usize {
        self.sources.len()
    }

    /// Total number of sources across all packs.
    pub fn source_count(&self) -> usize {
        self.sources.values().map(|v| v.len()).sum()
    }

    /// List loaded pack names.
    pub fn pack_names(&self) -> Vec<&str> {
        self.sources.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for PackEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a Parser instance from a PackSource definition.
fn create_parser(source: &PackSource) -> logbog_core::Result<Box<dyn Parser>> {
    match source.format.as_str() {
        "regex" | "grok" => {
            let pattern = source.pattern.as_deref().ok_or_else(|| {
                logbog_core::Error::PackInvalid(format!(
                    "source '{}' uses regex format but has no pattern",
                    source.name
                ))
            })?;
            let mut parser = RegexParser::new(pattern).map_err(|e| {
                logbog_core::Error::PackInvalid(format!(
                    "source '{}' has invalid regex pattern: {e}",
                    source.name
                ))
            })?;
            if let Some(ref fmt) = source.timestamp_format {
                parser = parser.with_timestamp_format(fmt);
            }
            if let Some(ref field) = source.timestamp_field {
                parser.timestamp_field = field.clone();
            }
            Ok(Box::new(parser))
        }
        "json" => {
            let mut parser = JsonParser::default();
            if let Some(ref field) = source.timestamp_field {
                parser.timestamp_field = field.clone();
            }
            Ok(Box::new(parser))
        }
        "logfmt" => {
            let mut parser = LogfmtParser::default();
            if let Some(ref field) = source.timestamp_field {
                parser.timestamp_field = field.clone();
            }
            Ok(Box::new(parser))
        }
        "syslog" | "syslog-auto" => Ok(Box::new(SyslogParser {
            rfc: SyslogRfc::Auto,
        })),
        "syslog-3164" => Ok(Box::new(SyslogParser {
            rfc: SyslogRfc::Rfc3164,
        })),
        "syslog-5424" => Ok(Box::new(SyslogParser {
            rfc: SyslogRfc::Rfc5424,
        })),
        other => Err(logbog_core::Error::PackInvalid(format!(
            "source '{}' uses unknown format: '{other}'",
            source.name
        ))),
    }
}

/// Known/supported parser formats.
pub const SUPPORTED_FORMATS: &[&str] = &[
    "regex",
    "grok",
    "json",
    "logfmt",
    "syslog",
    "syslog-auto",
    "syslog-3164",
    "syslog-5424",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn nginx_manifest() -> PackManifest {
        toml::from_str(
            r#"
[pack]
name = "nginx"
version = "1.0.0"
description = "Nginx log parser"

[[sources]]
name = "access"
paths = ["/var/log/nginx/access.log"]
format = "regex"
pattern = '(?P<client_ip>\S+) - (?P<remote_user>\S+) \[(?P<timestamp>[^\]]+)\] "(?P<method>\S+) (?P<uri>\S+) HTTP/(?P<http_version>\S+)" (?P<status>\d+) (?P<body_bytes>\d+) "(?P<referrer>[^"]*)" "(?P<user_agent>[^"]*)"'
timestamp_format = "%d/%b/%Y:%H:%M:%S %z"

[[sources]]
name = "error"
paths = ["/var/log/nginx/error.log"]
format = "regex"
pattern = '(?P<timestamp>\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}) \[(?P<level>\w+)\] (?P<pid>\d+)#(?P<tid>\d+): \*(?P<cid>\d+) (?P<message>.*)'
timestamp_format = "%Y/%m/%d %H:%M:%S"
"#,
        )
        .unwrap()
    }

    fn syslog_manifest() -> PackManifest {
        toml::from_str(
            r#"
[pack]
name = "syslog"
version = "1.0.0"
description = "Syslog parser"

[[sources]]
name = "syslog"
paths = ["/var/log/syslog"]
format = "syslog"

[[sources]]
name = "auth"
paths = ["/var/log/auth.log"]
format = "syslog"
"#,
        )
        .unwrap()
    }

    fn json_manifest() -> PackManifest {
        toml::from_str(
            r#"
[pack]
name = "systemd"
version = "1.0.0"
description = "Systemd journal"

[[sources]]
name = "journal"
paths = ["journalctl"]
format = "json"
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_load_nginx_pack() {
        let mut engine = PackEngine::new();
        engine.load_pack(&nginx_manifest()).unwrap();
        assert_eq!(engine.pack_count(), 1);
        assert_eq!(engine.source_count(), 2);
    }

    #[test]
    fn test_parse_nginx_access_log() {
        let mut engine = PackEngine::new();
        engine.load_pack(&nginx_manifest()).unwrap();

        let line = r#"93.184.216.34 - - [09/Apr/2026:10:15:30 +0000] "GET /api/users HTTP/1.1" 200 1234 "https://example.com" "Mozilla/5.0""#;
        let entry = engine.parse_line(line, "nginx", "access").unwrap();
        assert_eq!(entry.source, "access");
        assert_eq!(entry.pack, "nginx");
        assert_eq!(
            entry.fields.get("status"),
            Some(&serde_json::json!(200))
        );
        assert_eq!(
            entry.fields.get("method"),
            Some(&serde_json::Value::String("GET".into()))
        );
        assert!(entry.timestamp.timestamp() > 0);
    }

    #[test]
    fn test_parse_nginx_error_log() {
        let mut engine = PackEngine::new();
        engine.load_pack(&nginx_manifest()).unwrap();

        let line = "2026/04/09 10:15:30 [error] 1234#0: *5678 upstream timed out, client: 192.168.1.1";
        let entry = engine.parse_line(line, "nginx", "error").unwrap();
        assert_eq!(entry.level, logbog_core::LogLevel::Error);
        assert!(entry.message.contains("upstream timed out"));
    }

    #[test]
    fn test_parse_syslog() {
        let mut engine = PackEngine::new();
        engine.load_pack(&syslog_manifest()).unwrap();

        let line =
            "<34>Apr  9 10:15:30 web-01 sshd[12345]: Failed password for root from 192.168.1.100";
        let entry = engine.parse_line(line, "syslog", "syslog").unwrap();
        assert!(entry.message.contains("Failed password"));
        assert_eq!(
            entry.fields.get("app"),
            Some(&serde_json::Value::String("sshd".into()))
        );
    }

    #[test]
    fn test_parse_json_log() {
        let mut engine = PackEngine::new();
        engine.load_pack(&json_manifest()).unwrap();

        let line =
            r#"{"timestamp":"2026-04-09T10:00:00Z","level":"error","message":"service crashed","unit":"nginx.service"}"#;
        let entry = engine.parse_line(line, "systemd", "journal").unwrap();
        assert_eq!(entry.level, logbog_core::LogLevel::Error);
        assert_eq!(entry.message, "service crashed");
    }

    #[test]
    fn test_parse_line_any_source() {
        let mut engine = PackEngine::new();
        engine.load_pack(&nginx_manifest()).unwrap();

        let line = "2026/04/09 10:15:30 [warn] 1234#0: *5678 something happened, client: 1.2.3.4";
        let (source, entry) = engine.parse_line_any_source(line, "nginx").unwrap();
        assert_eq!(source, "error");
        assert_eq!(entry.level, logbog_core::LogLevel::Warn);
    }

    #[test]
    fn test_all_watch_paths() {
        let mut engine = PackEngine::new();
        engine.load_pack(&nginx_manifest()).unwrap();
        engine.load_pack(&syslog_manifest()).unwrap();

        let paths = engine.all_watch_paths();
        assert_eq!(paths.len(), 4); // 2 nginx + 2 syslog
    }

    #[test]
    fn test_unknown_format() {
        let manifest: PackManifest = toml::from_str(
            r#"
[pack]
name = "bad"
version = "1.0.0"
description = "Bad pack"

[[sources]]
name = "main"
paths = ["/tmp/test.log"]
format = "xml"
"#,
        )
        .unwrap();

        let mut engine = PackEngine::new();
        let result = engine.load_pack(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_regex_without_pattern() {
        let manifest: PackManifest = toml::from_str(
            r#"
[pack]
name = "bad"
version = "1.0.0"
description = "Bad pack"

[[sources]]
name = "main"
paths = ["/tmp/test.log"]
format = "regex"
"#,
        )
        .unwrap();

        let mut engine = PackEngine::new();
        let result = engine.load_pack(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_regex_pattern() {
        let manifest: PackManifest = toml::from_str(
            r#"
[pack]
name = "bad"
version = "1.0.0"
description = "Bad pack"

[[sources]]
name = "main"
paths = ["/tmp/test.log"]
format = "regex"
pattern = '(?P<unclosed'
"#,
        )
        .unwrap();

        let mut engine = PackEngine::new();
        let result = engine.load_pack(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiline_pack() {
        let manifest: PackManifest = toml::from_str(
            r#"
[pack]
name = "php-fpm"
version = "1.0.0"
description = "PHP-FPM"

[[sources]]
name = "error"
paths = ["/var/log/php-fpm.log"]
format = "regex"
pattern = '\[(?P<timestamp>[^\]]+)\] (?P<level>\w+): (?P<message>.*)'
multiline = true
multiline_start = '^\['
"#,
        )
        .unwrap();

        let mut engine = PackEngine::new();
        engine.load_pack(&manifest).unwrap();
        assert!(engine.is_multiline("php-fpm", "error"));
        assert!(!engine.is_multiline("php-fpm", "nonexistent"));
    }
}
