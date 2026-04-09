//! Integration tests: each pack correctly parses its reference log files.

use logbog_packs::engine::PackEngine;
use logbog_packs::manifest::PackManifest;
use std::path::Path;

fn load_fixture(name: &str) -> String {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let path = workspace_root.join("tests/fixtures").join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {e}", path.display()))
}

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

fn mysql_manifest() -> PackManifest {
    toml::from_str(
        r#"
[pack]
name = "mysql"
version = "1.0.0"
description = "MySQL error log parser"

[[sources]]
name = "error"
paths = ["/var/log/mysql/error.log"]
format = "regex"
pattern = '(?P<timestamp>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d+Z)\s+(?P<tid>\d+)\s+\[(?P<level>\w+)\]\s+(?P<message>.*)'
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
"#,
    )
    .unwrap()
}

fn systemd_manifest() -> PackManifest {
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

fn phpfpm_manifest() -> PackManifest {
    toml::from_str(
        r#"
[pack]
name = "php-fpm"
version = "1.0.0"
description = "PHP-FPM error log"

[[sources]]
name = "error"
paths = ["/var/log/php-fpm/error.log"]
format = "regex"
pattern = '\[(?P<timestamp>[^\]]+)\] (?P<level>\w+): (?P<message>.*)'
"#,
    )
    .unwrap()
}

/// Helper: parse every line in a fixture and return (parsed_count, total_count, failed_lines)
fn parse_fixture(
    engine: &PackEngine,
    fixture_name: &str,
    pack: &str,
    source: &str,
) -> (usize, usize, Vec<String>) {
    let content = load_fixture(fixture_name);
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    let total = lines.len();
    let mut parsed = 0;
    let mut failed = Vec::new();

    for line in &lines {
        if engine.parse_line(line, pack, source).is_some() {
            parsed += 1;
        } else {
            failed.push(line.to_string());
        }
    }

    (parsed, total, failed)
}

#[test]
fn test_nginx_access_log_100_percent() {
    let mut engine = PackEngine::new();
    engine.load_pack(&nginx_manifest()).unwrap();

    let (parsed, total, failed) = parse_fixture(&engine, "nginx_access.log", "nginx", "access");
    assert!(total > 0, "Fixture is empty");
    assert_eq!(
        parsed, total,
        "Failed {}/{total} lines: {failed:?}",
        total - parsed
    );
}

#[test]
fn test_nginx_error_log_100_percent() {
    let mut engine = PackEngine::new();
    engine.load_pack(&nginx_manifest()).unwrap();

    let (parsed, total, failed) = parse_fixture(&engine, "nginx_error.log", "nginx", "error");
    assert!(total > 0);
    assert_eq!(
        parsed, total,
        "Failed {}/{total} lines: {failed:?}",
        total - parsed
    );
}

#[test]
fn test_nginx_access_fields_extraction() {
    let mut engine = PackEngine::new();
    engine.load_pack(&nginx_manifest()).unwrap();

    let line = r#"93.184.216.34 - - [09/Apr/2026:10:15:30 +0000] "GET /api/users HTTP/1.1" 200 1234 "https://example.com" "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36""#;
    let entry = engine.parse_line(line, "nginx", "access").unwrap();

    assert_eq!(
        entry.fields.get("client_ip"),
        Some(&serde_json::json!("93.184.216.34"))
    );
    assert_eq!(
        entry.fields.get("method"),
        Some(&serde_json::json!("GET"))
    );
    assert_eq!(
        entry.fields.get("uri"),
        Some(&serde_json::json!("/api/users"))
    );
    assert_eq!(entry.fields.get("status"), Some(&serde_json::json!(200)));
    assert_eq!(
        entry.fields.get("body_bytes"),
        Some(&serde_json::json!(1234))
    );
    assert!(entry.timestamp.timestamp() > 0);
}

#[test]
fn test_mysql_error_log_100_percent() {
    let mut engine = PackEngine::new();
    engine.load_pack(&mysql_manifest()).unwrap();

    let (parsed, total, failed) = parse_fixture(&engine, "mysql_error.log", "mysql", "error");
    assert!(total > 0);
    assert_eq!(
        parsed, total,
        "Failed {}/{total} lines: {failed:?}",
        total - parsed
    );
}

#[test]
fn test_syslog_messages_high_parse_rate() {
    let mut engine = PackEngine::new();
    engine.load_pack(&syslog_manifest()).unwrap();

    let content = load_fixture("syslog_messages.log");
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    let total = lines.len();

    let mut parsed = 0;
    for line in &lines {
        if engine.parse_line(line, "syslog", "syslog").is_some() {
            parsed += 1;
        }
    }

    // Syslog should parse most lines (both RFC 3164 and 5424)
    let parse_rate = (parsed as f64) / (total as f64);
    assert!(
        parse_rate >= 0.8,
        "Syslog parse rate too low: {parsed}/{total} ({:.0}%)",
        parse_rate * 100.0
    );
}

#[test]
fn test_systemd_journal_100_percent() {
    let mut engine = PackEngine::new();
    engine.load_pack(&systemd_manifest()).unwrap();

    let (parsed, total, failed) =
        parse_fixture(&engine, "systemd_journal.jsonl", "systemd", "journal");
    assert!(total > 0);
    assert_eq!(
        parsed, total,
        "Failed {}/{total} lines: {failed:?}",
        total - parsed
    );
}

#[test]
fn test_phpfpm_error_log_100_percent() {
    let mut engine = PackEngine::new();
    engine.load_pack(&phpfpm_manifest()).unwrap();

    let (parsed, total, failed) =
        parse_fixture(&engine, "php_fpm_error.log", "php-fpm", "error");
    assert!(total > 0);
    assert_eq!(
        parsed, total,
        "Failed {}/{total} lines: {failed:?}",
        total - parsed
    );
}

#[test]
fn test_all_packs_load_simultaneously() {
    let mut engine = PackEngine::new();
    engine.load_pack(&nginx_manifest()).unwrap();
    engine.load_pack(&mysql_manifest()).unwrap();
    engine.load_pack(&syslog_manifest()).unwrap();
    engine.load_pack(&systemd_manifest()).unwrap();
    engine.load_pack(&phpfpm_manifest()).unwrap();

    assert_eq!(engine.pack_count(), 5);
    // nginx(2) + mysql(1) + syslog(1) + systemd(1) + php-fpm(1) = 6
    assert_eq!(engine.source_count(), 6);
}
