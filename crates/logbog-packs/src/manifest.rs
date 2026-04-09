use serde::{Deserialize, Serialize};
use std::path::Path;

/// Manifest d'un Log Pack (pack.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    pub pack: PackMeta,
    #[serde(default)]
    pub sources: Vec<PackSource>,
    pub schema: Option<PackSchema>,
    #[serde(default)]
    pub alerts: Vec<PackAlert>,
    #[serde(default)]
    pub correlations: Vec<PackCorrelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default = "default_license")]
    pub license: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

fn default_license() -> String {
    "Apache-2.0".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackSource {
    pub name: String,
    pub paths: Vec<String>,
    pub format: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub timestamp_format: Option<String>,
    #[serde(default)]
    pub timestamp_field: Option<String>,
    #[serde(default)]
    pub multiline: bool,
    #[serde(default)]
    pub multiline_start: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackSchema {
    pub fields: Vec<PackField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub indexed: bool,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackAlert {
    pub name: String,
    pub description: String,
    pub condition: String,
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default)]
    pub cooldown: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

fn default_severity() -> String {
    "warning".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackCorrelation {
    pub target_pack: String,
    pub match_fields: Vec<String>,
    #[serde(default = "default_window")]
    pub window: String,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_window() -> String {
    "2s".into()
}

impl PackManifest {
    /// Charge un manifest depuis un fichier pack.toml.
    pub fn load(path: &Path) -> logbog_core::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: PackManifest = toml::from_str(&content)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Valide la cohérence du manifest.
    pub fn validate(&self) -> logbog_core::Result<()> {
        if self.pack.name.is_empty() {
            return Err(logbog_core::Error::PackInvalid("pack name is empty".into()));
        }
        if self.pack.version.is_empty() {
            return Err(logbog_core::Error::PackInvalid(
                "pack version is empty".into(),
            ));
        }
        if self.sources.is_empty() {
            return Err(logbog_core::Error::PackInvalid(
                "pack has no sources defined".into(),
            ));
        }
        for source in &self.sources {
            if source.paths.is_empty() {
                return Err(logbog_core::Error::PackInvalid(format!(
                    "source '{}' has no paths",
                    source.name
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_toml() -> &'static str {
        r#"
[pack]
name = "nginx"
version = "1.0.0"
description = "Nginx log parser"
author = "LogBog Team"
tags = ["web", "http"]

[[sources]]
name = "access"
paths = ["/var/log/nginx/access.log"]
format = "grok"
pattern = "test pattern"

[[sources]]
name = "error"
paths = ["/var/log/nginx/error.log"]
format = "regex"
pattern = "test pattern"

[schema]
fields = [
    { name = "status", type = "int", indexed = true, description = "HTTP status code" },
]

[[alerts]]
name = "5xx_spike"
description = "5xx error spike"
condition = "count(status >= 500) > 10 in 1m"
severity = "critical"
cooldown = "5m"

[[correlations]]
target_pack = "php-fpm"
match_fields = ["timestamp", "uri:script"]
window = "2s"
"#
    }

    #[test]
    fn test_parse_manifest() {
        let manifest: PackManifest = toml::from_str(sample_toml()).unwrap();
        assert_eq!(manifest.pack.name, "nginx");
        assert_eq!(manifest.sources.len(), 2);
        assert_eq!(manifest.alerts.len(), 1);
        assert_eq!(manifest.correlations.len(), 1);
    }

    #[test]
    fn test_validate_valid() {
        let manifest: PackManifest = toml::from_str(sample_toml()).unwrap();
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let toml_str = r#"
[pack]
name = ""
version = "1.0.0"
description = "test"
[[sources]]
name = "test"
paths = ["/test"]
format = "json"
"#;
        let manifest: PackManifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_validate_no_sources() {
        let toml_str = r#"
[pack]
name = "test"
version = "1.0.0"
description = "test"
"#;
        let manifest: PackManifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.validate().is_err());
    }
}
