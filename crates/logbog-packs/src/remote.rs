use serde::{Deserialize, Serialize};

/// Metadata for a pack available in the remote registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePackInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub tags: Vec<String>,
}

/// Remote pack registry — lists available packs that can be installed.
///
/// Currently uses a builtin index. In the future, this will fetch
/// from a GitHub-based registry or logbog.dev/packs.
pub struct RemoteRegistry {
    packs: Vec<RemotePackInfo>,
}

impl RemoteRegistry {
    /// Create a new remote registry with the builtin pack index.
    pub fn new() -> Self {
        Self {
            packs: builtin_index(),
        }
    }

    /// List all available packs.
    pub fn list(&self) -> &[RemotePackInfo] {
        &self.packs
    }

    /// Search packs by name or tag.
    pub fn search(&self, query: &str) -> Vec<&RemotePackInfo> {
        let q = query.to_lowercase();
        self.packs
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
                    || p.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Get a specific pack by name.
    pub fn get(&self, name: &str) -> Option<&RemotePackInfo> {
        self.packs.iter().find(|p| p.name == name)
    }

    /// Check if a pack is available.
    pub fn is_available(&self, name: &str) -> bool {
        self.packs.iter().any(|p| p.name == name)
    }

    /// Number of available packs.
    pub fn count(&self) -> usize {
        self.packs.len()
    }
}

impl Default for RemoteRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builtin pack index — the 5 core packs plus planned ones.
fn builtin_index() -> Vec<RemotePackInfo> {
    vec![
        RemotePackInfo {
            name: "nginx".into(),
            version: "1.0.0".into(),
            description: "Nginx access and error log parser".into(),
            tags: vec!["web".into(), "reverse-proxy".into(), "http".into()],
        },
        RemotePackInfo {
            name: "php-fpm".into(),
            version: "1.0.0".into(),
            description: "PHP-FPM error and slow log parser".into(),
            tags: vec!["php".into(), "web".into(), "backend".into()],
        },
        RemotePackInfo {
            name: "mysql".into(),
            version: "1.0.0".into(),
            description: "MySQL/MariaDB error and slow query log parser".into(),
            tags: vec!["database".into(), "mysql".into(), "mariadb".into()],
        },
        RemotePackInfo {
            name: "systemd".into(),
            version: "1.0.0".into(),
            description: "Systemd journal reader".into(),
            tags: vec!["system".into(), "linux".into(), "journal".into()],
        },
        RemotePackInfo {
            name: "syslog".into(),
            version: "1.0.0".into(),
            description: "Generic syslog and auth log parser (RFC 3164 & 5424)".into(),
            tags: vec!["system".into(), "security".into(), "auth".into()],
        },
        RemotePackInfo {
            name: "apache".into(),
            version: "0.1.0".into(),
            description: "Apache httpd access and error log parser".into(),
            tags: vec!["web".into(), "http".into()],
        },
        RemotePackInfo {
            name: "postgresql".into(),
            version: "0.1.0".into(),
            description: "PostgreSQL log parser".into(),
            tags: vec!["database".into(), "postgresql".into()],
        },
        RemotePackInfo {
            name: "redis".into(),
            version: "0.1.0".into(),
            description: "Redis log parser".into(),
            tags: vec!["database".into(), "cache".into(), "redis".into()],
        },
        RemotePackInfo {
            name: "docker".into(),
            version: "0.1.0".into(),
            description: "Docker container log parser".into(),
            tags: vec!["container".into(), "docker".into()],
        },
        RemotePackInfo {
            name: "haproxy".into(),
            version: "0.1.0".into(),
            description: "HAProxy access and stats log parser".into(),
            tags: vec!["web".into(), "load-balancer".into()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_registry() {
        let registry = RemoteRegistry::new();
        assert!(registry.count() >= 5);
        assert!(registry.is_available("nginx"));
        assert!(registry.is_available("php-fpm"));
        assert!(!registry.is_available("nonexistent"));
    }

    #[test]
    fn test_remote_search() {
        let registry = RemoteRegistry::new();
        let results = registry.search("web");
        assert!(results.len() >= 2); // nginx, php-fpm, apache, haproxy
    }

    #[test]
    fn test_remote_search_by_tag() {
        let registry = RemoteRegistry::new();
        let results = registry.search("database");
        assert!(results.len() >= 2); // mysql, postgresql, redis
    }

    #[test]
    fn test_remote_get() {
        let registry = RemoteRegistry::new();
        let pack = registry.get("nginx").unwrap();
        assert_eq!(pack.version, "1.0.0");
        assert!(pack.tags.contains(&"web".to_string()));
    }
}
