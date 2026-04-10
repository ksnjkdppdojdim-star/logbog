use chrono::{DateTime, Duration, Utc};
use logbog_core::{Correlation, CorrelationType, LogEntry};
use std::collections::VecDeque;
use tracing::debug;
use ulid::Ulid;

use crate::extractor::{self, ExtractedIds};

/// Entry with its pre-extracted identifiers, kept in the correlation window.
struct WindowEntry {
    entry: LogEntry,
    ids: ExtractedIds,
    timestamp: DateTime<Utc>,
}

/// Temporal correlation engine.
///
/// Maintains a sliding window of recent log entries and finds
/// correlations between them based on shared identifiers
/// (IPs, PIDs, request IDs, etc.) within a configurable time window.
pub struct Correlator {
    /// Time window for correlation (entries older than this are evicted).
    window: Duration,
    /// Minimum confidence score to emit a correlation.
    min_confidence: f64,
    /// Sliding window of recent entries.
    buffer: VecDeque<WindowEntry>,
    /// Maximum buffer size to prevent unbounded memory usage.
    max_buffer: usize,
}

impl Correlator {
    pub fn new(window_seconds: i64, min_confidence: f64) -> Self {
        Self {
            window: Duration::seconds(window_seconds),
            min_confidence,
            buffer: VecDeque::new(),
            max_buffer: 10_000,
        }
    }

    /// Process a new log entry and return any correlations found.
    pub fn process(&mut self, entry: LogEntry) -> Vec<Correlation> {
        let now = entry.timestamp;
        self.evict_old(now);

        let ids = extractor::extract(&entry);
        if ids.is_empty() {
            self.push_entry(entry, ids);
            return Vec::new();
        }

        let mut correlations = Vec::new();

        // Find correlations with entries in the window
        let mut correlated_ids = Vec::new();
        for (idx, window_entry) in self.buffer.iter().enumerate() {
            // Skip entries from the same pack+source (self-correlation)
            if window_entry.entry.pack == entry.pack && window_entry.entry.source == entry.source {
                continue;
            }

            let matches = ids.overlaps(&window_entry.ids);
            if matches.is_empty() {
                continue;
            }

            let confidence = compute_confidence(&matches, &entry, &window_entry.entry, self.window);
            if confidence >= self.min_confidence {
                correlated_ids.push((idx, confidence, matches));
            }
        }

        // Build correlations from matches
        for (idx, confidence, matches) in correlated_ids {
            let other = &self.buffer[idx].entry;
            let match_desc = matches
                .iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let correlation_type = infer_type(&entry, other, &matches);

            let correlation = Correlation {
                id: Ulid::new(),
                entry_ids: vec![other.id, entry.id],
                confidence,
                correlation_type,
                description: format!(
                    "{}/{} <-> {}/{} via {match_desc}",
                    entry.pack, entry.source, other.pack, other.source
                ),
                timestamp: now,
            };

            debug!(
                confidence = confidence,
                desc = %correlation.description,
                "Correlation found"
            );

            correlations.push(correlation);
        }

        self.push_entry(entry, ids);
        correlations
    }

    /// Evict entries older than the time window.
    fn evict_old(&mut self, now: DateTime<Utc>) {
        let cutoff = now - self.window;
        while self.buffer.front().is_some_and(|e| e.timestamp < cutoff) {
            self.buffer.pop_front();
        }
    }

    fn push_entry(&mut self, entry: LogEntry, ids: ExtractedIds) {
        let timestamp = entry.timestamp;
        self.buffer.push_back(WindowEntry {
            entry,
            ids,
            timestamp,
        });
        // Enforce max buffer size
        while self.buffer.len() > self.max_buffer {
            self.buffer.pop_front();
        }
    }

    /// Number of entries currently in the window.
    pub fn window_size(&self) -> usize {
        self.buffer.len()
    }
}

/// Compute a confidence score based on the quality and quantity of matches.
fn compute_confidence(
    matches: &[extractor::MatchField],
    entry_a: &LogEntry,
    entry_b: &LogEntry,
    window: Duration,
) -> f64 {
    let mut score = 0.0;

    // More matching fields = higher confidence
    for m in matches {
        score += match m {
            extractor::MatchField::RequestId(_) => 0.9,
            extractor::MatchField::SessionId(_) => 0.8,
            extractor::MatchField::Ip(_) => 0.5,
            extractor::MatchField::Pid(_) => 0.6,
            extractor::MatchField::Uri(_) => 0.4,
            extractor::MatchField::User(_) => 0.3,
        };
    }

    // Closer in time = higher confidence
    let time_diff = (entry_a.timestamp - entry_b.timestamp)
        .num_milliseconds()
        .unsigned_abs();
    let window_ms = window.num_milliseconds().unsigned_abs();
    if window_ms > 0 {
        let time_factor = 1.0 - (time_diff as f64 / window_ms as f64);
        score *= 0.5 + (0.5 * time_factor);
    }

    // Different packs = more interesting correlation
    if entry_a.pack != entry_b.pack {
        score *= 1.2;
    }

    // Error-level entries are more important to correlate
    if entry_a.level >= logbog_core::LogLevel::Error
        || entry_b.level >= logbog_core::LogLevel::Error
    {
        score *= 1.1;
    }

    score.min(1.0)
}

/// Infer the correlation type from context.
fn infer_type(a: &LogEntry, b: &LogEntry, matches: &[extractor::MatchField]) -> CorrelationType {
    // If linked by request ID, it's a request chain
    if matches
        .iter()
        .any(|m| matches!(m, extractor::MatchField::RequestId(_)))
    {
        return CorrelationType::RequestChain;
    }

    // If linked by PID, it's a process group
    if matches
        .iter()
        .any(|m| matches!(m, extractor::MatchField::Pid(_)))
    {
        return CorrelationType::ProcessGroup;
    }

    // If one is an error and the other explains it, it's causal
    if (a.level >= logbog_core::LogLevel::Error) != (b.level >= logbog_core::LogLevel::Error) {
        return CorrelationType::Causal;
    }

    // If linked by IP across different services, it's a request chain
    if a.pack != b.pack
        && matches
            .iter()
            .any(|m| matches!(m, extractor::MatchField::Ip(_)))
    {
        return CorrelationType::RequestChain;
    }

    CorrelationType::Temporal
}

#[cfg(test)]
mod tests {
    use super::*;
    use logbog_core::LogLevel;
    use std::collections::HashMap;

    fn make_entry(
        pack: &str,
        source: &str,
        level: LogLevel,
        message: &str,
        fields: Vec<(&str, serde_json::Value)>,
    ) -> LogEntry {
        let mut entry = LogEntry::new(source, pack);
        entry.level = level;
        entry.message = message.to_string();
        entry.raw = message.to_string();
        entry.fields = fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        entry
    }

    #[test]
    fn test_correlate_by_ip_across_packs() {
        let mut correlator = Correlator::new(2, 0.3);

        let nginx_entry = make_entry(
            "nginx",
            "access",
            LogLevel::Error,
            "GET /api/slow 502",
            vec![
                ("client_ip", serde_json::json!("192.168.1.100")),
                ("status", serde_json::json!(502)),
                ("uri", serde_json::json!("/api/slow")),
            ],
        );

        let php_entry = make_entry(
            "php-fpm",
            "error",
            LogLevel::Error,
            "Fatal error in /api/slow",
            vec![
                ("client_ip", serde_json::json!("192.168.1.100")),
                ("script", serde_json::json!("/api/slow")),
            ],
        );

        let c1 = correlator.process(nginx_entry);
        assert!(c1.is_empty()); // first entry, nothing to correlate

        let c2 = correlator.process(php_entry);
        assert!(!c2.is_empty(), "Should find correlation by IP");
        assert_eq!(c2[0].entry_ids.len(), 2);
        assert!(c2[0].confidence >= 0.3);
    }

    #[test]
    fn test_no_self_correlation() {
        let mut correlator = Correlator::new(2, 0.3);

        let e1 = make_entry(
            "nginx",
            "access",
            LogLevel::Info,
            "GET / 200",
            vec![("client_ip", serde_json::json!("192.168.1.1"))],
        );
        let e2 = make_entry(
            "nginx",
            "access",
            LogLevel::Info,
            "GET /about 200",
            vec![("client_ip", serde_json::json!("192.168.1.1"))],
        );

        correlator.process(e1);
        let corrs = correlator.process(e2);
        assert!(
            corrs.is_empty(),
            "Same pack+source should not self-correlate"
        );
    }

    #[test]
    fn test_correlate_by_request_id() {
        let mut correlator = Correlator::new(5, 0.3);

        let e1 = make_entry(
            "nginx",
            "access",
            LogLevel::Info,
            "request_id=abc123def456",
            vec![("request_id", serde_json::json!("abc123def456"))],
        );
        let e2 = make_entry(
            "php-fpm",
            "error",
            LogLevel::Error,
            "request_id=abc123def456 fatal error",
            vec![("request_id", serde_json::json!("abc123def456"))],
        );

        correlator.process(e1);
        let corrs = correlator.process(e2);
        assert!(!corrs.is_empty());
        assert!(matches!(
            corrs[0].correlation_type,
            CorrelationType::RequestChain
        ));
    }

    #[test]
    fn test_window_eviction() {
        let mut correlator = Correlator::new(2, 0.3);

        let mut old_entry = make_entry(
            "nginx",
            "access",
            LogLevel::Info,
            "old request",
            vec![("client_ip", serde_json::json!("192.168.1.1"))],
        );
        old_entry.timestamp = Utc::now() - Duration::seconds(10);

        let new_entry = make_entry(
            "php-fpm",
            "error",
            LogLevel::Error,
            "new error",
            vec![("client_ip", serde_json::json!("192.168.1.1"))],
        );

        correlator.process(old_entry);
        let corrs = correlator.process(new_entry);
        assert!(corrs.is_empty(), "Old entry should be evicted from window");
    }

    #[test]
    fn test_502_chain_nginx_php_mysql() {
        let mut correlator = Correlator::new(5, 0.3);

        let nginx = make_entry(
            "nginx",
            "access",
            LogLevel::Error,
            "GET /api/data 502",
            vec![
                ("client_ip", serde_json::json!("10.0.0.5")),
                ("uri", serde_json::json!("/api/data")),
                ("status", serde_json::json!(502)),
            ],
        );
        let php = make_entry(
            "php-fpm",
            "error",
            LogLevel::Error,
            "PHP Fatal error: MySQL connection refused",
            vec![("client_ip", serde_json::json!("10.0.0.5"))],
        );
        let mysql = make_entry(
            "mysql",
            "error",
            LogLevel::Error,
            "Too many connections",
            vec![],
        );

        let c1 = correlator.process(nginx);
        assert!(c1.is_empty());

        let c2 = correlator.process(php);
        assert!(!c2.is_empty(), "nginx <-> php-fpm should correlate via IP");

        // MySQL won't correlate without shared identifiers
        let c3 = correlator.process(mysql);
        // MySQL error has no shared IP/PID, so may or may not correlate
        let _ = c3;

        assert!(correlator.window_size() == 3);
    }
}
