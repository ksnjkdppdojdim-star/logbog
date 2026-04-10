use logbog_core::{Correlation, CorrelationType, LogEntry, LogLevel};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// A causal chain — a sequence of correlated log entries forming an incident.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalChain {
    pub id: Ulid,
    pub name: String,
    pub entries: Vec<ChainEntry>,
    pub severity: LogLevel,
    pub root_cause: Option<String>,
    pub correlation_ids: Vec<Ulid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEntry {
    pub entry_id: Ulid,
    pub pack: String,
    pub source: String,
    pub level: LogLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub role: ChainRole,
}

/// Role of an entry in the causal chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainRole {
    /// The user-facing symptom (e.g., 502 error).
    Symptom,
    /// An intermediate processing step.
    Intermediate,
    /// The underlying root cause.
    RootCause,
}

/// Known chain templates for common incident patterns.
#[derive(Debug, Clone)]
pub struct ChainTemplate {
    pub name: &'static str,
    pub description: &'static str,
    pub pattern: Vec<ChainStep>,
}

#[derive(Debug, Clone)]
pub struct ChainStep {
    pub pack: &'static str,
    pub level_min: LogLevel,
    pub message_contains: &'static str,
}

/// Built-in chain templates.
pub fn builtin_templates() -> Vec<ChainTemplate> {
    vec![
        ChainTemplate {
            name: "502_chain",
            description: "502 Bad Gateway: nginx -> php-fpm -> backend error",
            pattern: vec![
                ChainStep {
                    pack: "nginx",
                    level_min: LogLevel::Error,
                    message_contains: "502",
                },
                ChainStep {
                    pack: "php-fpm",
                    level_min: LogLevel::Error,
                    message_contains: "",
                },
            ],
        },
        ChainTemplate {
            name: "oom_chain",
            description: "OOM Killer: memory exhaustion -> process kill -> service restart",
            pattern: vec![
                ChainStep {
                    pack: "systemd",
                    level_min: LogLevel::Error,
                    message_contains: "Out of memory",
                },
                ChainStep {
                    pack: "systemd",
                    level_min: LogLevel::Info,
                    message_contains: "restart",
                },
            ],
        },
        ChainTemplate {
            name: "mysql_overload",
            description: "MySQL overload: too many connections -> query timeouts -> 502",
            pattern: vec![
                ChainStep {
                    pack: "mysql",
                    level_min: LogLevel::Error,
                    message_contains: "Too many connections",
                },
                ChainStep {
                    pack: "php-fpm",
                    level_min: LogLevel::Error,
                    message_contains: "",
                },
                ChainStep {
                    pack: "nginx",
                    level_min: LogLevel::Error,
                    message_contains: "502",
                },
            ],
        },
        ChainTemplate {
            name: "auth_bruteforce",
            description: "SSH brute force: multiple failed passwords from same IP",
            pattern: vec![ChainStep {
                pack: "syslog",
                level_min: LogLevel::Warn,
                message_contains: "Failed password",
            }],
        },
    ]
}

/// Try to match a set of correlations + entries against known chain templates.
pub fn detect_chain(correlations: &[Correlation], entries: &[&LogEntry]) -> Option<CausalChain> {
    let templates = builtin_templates();

    for template in &templates {
        if let Some(chain) = try_match_template(template, correlations, entries) {
            return Some(chain);
        }
    }

    // If no template matches but we have correlations with errors, build a generic chain
    if correlations.len() >= 2 || entries.iter().any(|e| e.level >= LogLevel::Error) {
        return build_generic_chain(correlations, entries);
    }

    None
}

fn try_match_template(
    template: &ChainTemplate,
    _correlations: &[Correlation],
    entries: &[&LogEntry],
) -> Option<CausalChain> {
    let mut matched_entries = Vec::new();

    for step in &template.pattern {
        let found = entries.iter().find(|e| {
            e.pack == step.pack
                && e.level >= step.level_min
                && (step.message_contains.is_empty()
                    || e.message
                        .to_lowercase()
                        .contains(&step.message_contains.to_lowercase()))
        });

        match found {
            Some(entry) => matched_entries.push(*entry),
            None => return None, // Step not matched, template doesn't apply
        }
    }

    if matched_entries.is_empty() {
        return None;
    }

    // Sort by timestamp
    matched_entries.sort_by_key(|e| e.timestamp);

    let severity = matched_entries
        .iter()
        .map(|e| e.level)
        .max()
        .unwrap_or(LogLevel::Info);

    let chain_entries: Vec<ChainEntry> = matched_entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let role = if i == 0 {
                ChainRole::RootCause
            } else if i == matched_entries.len() - 1 {
                ChainRole::Symptom
            } else {
                ChainRole::Intermediate
            };

            ChainEntry {
                entry_id: e.id,
                pack: e.pack.clone(),
                source: e.source.clone(),
                level: e.level,
                message: e.message.clone(),
                timestamp: e.timestamp,
                role,
            }
        })
        .collect();

    let root_cause = chain_entries
        .first()
        .map(|e| format!("{}/{}: {}", e.pack, e.source, e.message));

    Some(CausalChain {
        id: Ulid::new(),
        name: template.name.to_string(),
        entries: chain_entries,
        severity,
        root_cause,
        correlation_ids: Vec::new(),
    })
}

fn build_generic_chain(correlations: &[Correlation], entries: &[&LogEntry]) -> Option<CausalChain> {
    if entries.is_empty() {
        return None;
    }

    let mut sorted: Vec<&&LogEntry> = entries.iter().collect();
    sorted.sort_by_key(|e| e.timestamp);

    let severity = entries
        .iter()
        .map(|e| e.level)
        .max()
        .unwrap_or(LogLevel::Info);

    let chain_entries: Vec<ChainEntry> = sorted
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let role = if e.level >= LogLevel::Error && i == 0 {
                ChainRole::RootCause
            } else if e.level >= LogLevel::Error {
                ChainRole::Symptom
            } else {
                ChainRole::Intermediate
            };

            ChainEntry {
                entry_id: e.id,
                pack: e.pack.clone(),
                source: e.source.clone(),
                level: e.level,
                message: e.message.clone(),
                timestamp: e.timestamp,
                role,
            }
        })
        .collect();

    Some(CausalChain {
        id: Ulid::new(),
        name: "generic_chain".to_string(),
        entries: chain_entries,
        severity,
        root_cause: None,
        correlation_ids: correlations.iter().map(|c| c.id).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(pack: &str, source: &str, level: LogLevel, msg: &str) -> LogEntry {
        let mut entry = LogEntry::new(source, pack);
        entry.level = level;
        entry.message = msg.to_string();
        entry
    }

    #[test]
    fn test_detect_502_chain() {
        let nginx = make_entry("nginx", "access", LogLevel::Error, "GET /api 502");
        let php = make_entry("php-fpm", "error", LogLevel::Error, "Segfault in worker");

        let entries: Vec<&LogEntry> = vec![&nginx, &php];
        let chain = detect_chain(&[], &entries);
        assert!(chain.is_some());
        let chain = chain.unwrap();
        assert_eq!(chain.name, "502_chain");
        assert_eq!(chain.entries.len(), 2);
    }

    #[test]
    fn test_detect_oom_chain() {
        let oom = make_entry(
            "systemd",
            "journal",
            LogLevel::Error,
            "Out of memory: killed process 1234",
        );
        let restart = make_entry(
            "systemd",
            "journal",
            LogLevel::Info,
            "mysql.service: Scheduled restart job",
        );

        let entries: Vec<&LogEntry> = vec![&oom, &restart];
        let chain = detect_chain(&[], &entries);
        assert!(chain.is_some());
        assert_eq!(chain.unwrap().name, "oom_chain");
    }

    #[test]
    fn test_builtin_templates_count() {
        assert!(builtin_templates().len() >= 4);
    }

    #[test]
    fn test_no_chain_for_normal_logs() {
        let e1 = make_entry("nginx", "access", LogLevel::Info, "GET / 200");
        let entries: Vec<&LogEntry> = vec![&e1];
        let chain = detect_chain(&[], &entries);
        assert!(chain.is_none());
    }
}
