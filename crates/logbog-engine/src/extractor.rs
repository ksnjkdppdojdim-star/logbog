use logbog_core::LogEntry;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::LazyLock;

/// Identifiers extracted from a log entry for correlation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractedIds {
    pub ips: HashSet<String>,
    pub pids: HashSet<u64>,
    pub request_ids: HashSet<String>,
    pub session_ids: HashSet<String>,
    pub uris: HashSet<String>,
    pub users: HashSet<String>,
    pub units: HashSet<String>,
}

impl ExtractedIds {
    /// Check if there is any overlap with another set of extracted IDs.
    pub fn overlaps(&self, other: &ExtractedIds) -> Vec<MatchField> {
        let mut matches = Vec::new();

        for ip in self.ips.intersection(&other.ips) {
            matches.push(MatchField::Ip(ip.clone()));
        }
        for pid in self.pids.intersection(&other.pids) {
            matches.push(MatchField::Pid(*pid));
        }
        for rid in self.request_ids.intersection(&other.request_ids) {
            matches.push(MatchField::RequestId(rid.clone()));
        }
        for sid in self.session_ids.intersection(&other.session_ids) {
            matches.push(MatchField::SessionId(sid.clone()));
        }
        for uri in self.uris.intersection(&other.uris) {
            matches.push(MatchField::Uri(uri.clone()));
        }
        for user in self.users.intersection(&other.users) {
            matches.push(MatchField::User(user.clone()));
        }

        matches
    }

    pub fn is_empty(&self) -> bool {
        self.ips.is_empty()
            && self.pids.is_empty()
            && self.request_ids.is_empty()
            && self.session_ids.is_empty()
            && self.uris.is_empty()
            && self.users.is_empty()
            && self.units.is_empty()
    }
}

/// A specific field that matched between two entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchField {
    Ip(String),
    Pid(u64),
    RequestId(String),
    SessionId(String),
    Uri(String),
    User(String),
}

impl std::fmt::Display for MatchField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchField::Ip(v) => write!(f, "ip={v}"),
            MatchField::Pid(v) => write!(f, "pid={v}"),
            MatchField::RequestId(v) => write!(f, "request_id={v}"),
            MatchField::SessionId(v) => write!(f, "session_id={v}"),
            MatchField::Uri(v) => write!(f, "uri={v}"),
            MatchField::User(v) => write!(f, "user={v}"),
        }
    }
}

// Regex patterns for extracting identifiers from raw log lines
static RE_IPV4: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\b").unwrap());

static RE_REQUEST_ID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:request.?id|x.request.id|trace.?id|correlation.?id)[=: ]+([a-f0-9\-]{8,})")
        .unwrap()
});

static RE_SESSION_ID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:session.?id|sess.?id|jsessionid)[=: ]+([a-zA-Z0-9\-_]{8,})").unwrap()
});

/// Extract identifiers from a LogEntry for correlation purposes.
pub fn extract(entry: &LogEntry) -> ExtractedIds {
    let mut ids = ExtractedIds::default();

    // Extract from structured fields
    extract_from_fields(entry, &mut ids);

    // Extract from raw log line
    extract_from_raw(&entry.raw, &mut ids);

    // Extract from message
    extract_from_raw(&entry.message, &mut ids);

    ids
}

fn extract_from_fields(entry: &LogEntry, ids: &mut ExtractedIds) {
    for (key, value) in &entry.fields {
        let key_lower = key.to_lowercase();

        match value {
            serde_json::Value::String(s) => {
                if key_lower.contains("ip")
                    || key_lower == "client"
                    || key_lower == "remote_addr"
                    || key_lower == "host"
                {
                    if RE_IPV4.is_match(s) {
                        ids.ips.insert(s.clone());
                    }
                }
                if key_lower.contains("uri")
                    || key_lower == "path"
                    || key_lower == "script"
                    || key_lower == "url"
                {
                    ids.uris.insert(s.clone());
                }
                if key_lower.contains("user") || key_lower == "remote_user" {
                    if s != "-" && !s.is_empty() {
                        ids.users.insert(s.clone());
                    }
                }
                if key_lower.contains("request_id") || key_lower.contains("trace_id") {
                    ids.request_ids.insert(s.clone());
                }
                if key_lower.contains("session") {
                    ids.session_ids.insert(s.clone());
                }
                if key_lower == "unit" || key_lower == "service" {
                    ids.units.insert(s.clone());
                }
            }
            serde_json::Value::Number(n) => {
                if key_lower == "pid" || key_lower == "procid" {
                    if let Some(v) = n.as_u64() {
                        ids.pids.insert(v);
                    }
                }
            }
            _ => {}
        }
    }
}

fn extract_from_raw(text: &str, ids: &mut ExtractedIds) {
    if text.is_empty() {
        return;
    }

    // Extract IPs
    for cap in RE_IPV4.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let ip = m.as_str();
            // Skip common non-client IPs
            if ip != "127.0.0.1" && ip != "0.0.0.0" {
                ids.ips.insert(ip.to_string());
            }
        }
    }

    // Extract request IDs
    for cap in RE_REQUEST_ID.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            ids.request_ids.insert(m.as_str().to_string());
        }
    }

    // Extract session IDs
    for cap in RE_SESSION_ID.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            ids.session_ids.insert(m.as_str().to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logbog_core::{LogEntry, LogLevel};
    use std::collections::HashMap;

    fn make_entry(fields: Vec<(&str, serde_json::Value)>, raw: &str) -> LogEntry {
        let mut entry = LogEntry::new("test", "test");
        entry.raw = raw.to_string();
        entry.message = raw.to_string();
        entry.level = LogLevel::Info;
        entry.fields = fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        entry
    }

    #[test]
    fn test_extract_ip_from_fields() {
        let entry = make_entry(vec![("client_ip", serde_json::json!("192.168.1.100"))], "");
        let ids = extract(&entry);
        assert!(ids.ips.contains("192.168.1.100"));
    }

    #[test]
    fn test_extract_ip_from_raw() {
        let entry = make_entry(
            vec![],
            "client: 10.0.0.5, server: example.com, upstream: 192.168.1.50",
        );
        let ids = extract(&entry);
        assert!(ids.ips.contains("10.0.0.5"));
        assert!(ids.ips.contains("192.168.1.50"));
        assert!(!ids.ips.contains("127.0.0.1")); // filtered
    }

    #[test]
    fn test_extract_pid_from_fields() {
        let entry = make_entry(vec![("pid", serde_json::json!(1234))], "");
        let ids = extract(&entry);
        assert!(ids.pids.contains(&1234));
    }

    #[test]
    fn test_extract_uri_from_fields() {
        let entry = make_entry(vec![("uri", serde_json::json!("/api/users"))], "");
        let ids = extract(&entry);
        assert!(ids.uris.contains("/api/users"));
    }

    #[test]
    fn test_extract_request_id_from_raw() {
        let entry = make_entry(vec![], "request_id=abc123def456 processing request");
        let ids = extract(&entry);
        assert!(ids.request_ids.contains("abc123def456"));
    }

    #[test]
    fn test_overlaps() {
        let mut ids1 = ExtractedIds::default();
        ids1.ips.insert("192.168.1.1".into());
        ids1.pids.insert(1234);

        let mut ids2 = ExtractedIds::default();
        ids2.ips.insert("192.168.1.1".into());
        ids2.pids.insert(5678);

        let matches = ids1.overlaps(&ids2);
        assert_eq!(matches.len(), 1);
        assert!(matches!(matches[0], MatchField::Ip(_)));
    }

    #[test]
    fn test_no_overlap() {
        let mut ids1 = ExtractedIds::default();
        ids1.ips.insert("192.168.1.1".into());

        let mut ids2 = ExtractedIds::default();
        ids2.ips.insert("10.0.0.1".into());

        let matches = ids1.overlaps(&ids2);
        assert!(matches.is_empty());
    }
}
