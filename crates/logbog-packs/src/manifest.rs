use regex::Regex;
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
        // Pack name validation
        if self.pack.name.is_empty() {
            return Err(logbog_core::Error::PackInvalid("pack name is empty".into()));
        }
        if !self
            .pack
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(logbog_core::Error::PackInvalid(format!(
                "pack name '{}' contains invalid characters (use alphanumeric, - or _)",
                self.pack.name
            )));
        }

        // Version validation (semver-like)
        if self.pack.version.is_empty() {
            return Err(logbog_core::Error::PackInvalid(
                "pack version is empty".into(),
            ));
        }
        if !is_valid_semver(&self.pack.version) {
            return Err(logbog_core::Error::PackInvalid(format!(
                "pack version '{}' is not valid semver (expected X.Y.Z)",
                self.pack.version
            )));
        }

        // Sources validation
        if self.sources.is_empty() {
            return Err(logbog_core::Error::PackInvalid(
                "pack has no sources defined".into(),
            ));
        }
        for source in &self.sources {
            self.validate_source(source)?;
        }

        // Alert severity validation
        for alert in &self.alerts {
            let valid_severities = ["info", "warning", "critical"];
            if !valid_severities.contains(&alert.severity.as_str()) {
                return Err(logbog_core::Error::PackInvalid(format!(
                    "alert '{}' has invalid severity '{}' (use info, warning, or critical)",
                    alert.name, alert.severity
                )));
            }
        }

        Ok(())
    }

    /// Valide une source individuelle.
    fn validate_source(&self, source: &PackSource) -> logbog_core::Result<()> {
        if source.name.is_empty() {
            return Err(logbog_core::Error::PackInvalid(
                "source has empty name".into(),
            ));
        }
        if source.paths.is_empty() {
            return Err(logbog_core::Error::PackInvalid(format!(
                "source '{}' has no paths",
                source.name
            )));
        }

        // Format validation
        let valid_formats = [
            "regex",
            "grok",
            "json",
            "logfmt",
            "syslog",
            "syslog-auto",
            "syslog-3164",
            "syslog-5424",
        ];
        if !valid_formats.contains(&source.format.as_str()) {
            return Err(logbog_core::Error::PackInvalid(format!(
                "source '{}' uses unknown format '{}' (valid: {})",
                source.name,
                source.format,
                valid_formats.join(", ")
            )));
        }

        // Regex/grok sources must have a pattern
        if (source.format == "regex" || source.format == "grok") && source.pattern.is_none() {
            return Err(logbog_core::Error::PackInvalid(format!(
                "source '{}' uses {} format but has no pattern",
                source.name, source.format
            )));
        }

        // Multiline sources must have multiline_start
        if source.multiline && source.multiline_start.is_none() {
            return Err(logbog_core::Error::PackInvalid(format!(
                "source '{}' has multiline=true but no multiline_start pattern",
                source.name
            )));
        }

        // Validate regex pattern if provided
        if let Some(ref pattern) = source.pattern
            && Regex::new(pattern).is_err()
        {
            return Err(logbog_core::Error::PackInvalid(format!(
                "source '{}' has invalid regex pattern",
                source.name
            )));
        }

        // Validate multiline_start regex if provided
        if let Some(ref pattern) = source.multiline_start
            && Regex::new(pattern).is_err()
        {
            return Err(logbog_core::Error::PackInvalid(format!(
                "source '{}' has invalid multiline_start regex",
                source.name
            )));
        }

        // Schema field type validation
        if let Some(ref schema) = self.schema {
            let valid_types = ["string", "int", "float", "bool", "ip", "timestamp"];
            for field in &schema.fields {
                if !valid_types.contains(&field.field_type.as_str()) {
                    return Err(logbog_core::Error::PackInvalid(format!(
                        "field '{}' has invalid type '{}' (valid: {})",
                        field.name,
                        field.field_type,
                        valid_types.join(", ")
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Basic semver validation (X.Y.Z with optional pre-release).
fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('-').next().unwrap_or("").split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u64>().is_ok())
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

    #[test]
    fn test_validate_invalid_name_chars() {
        let toml_str = r#"
[pack]
name = "my pack!!"
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
    fn test_validate_invalid_version() {
        let toml_str = r#"
[pack]
name = "test"
version = "not-a-version"
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
    fn test_validate_unknown_format() {
        let toml_str = r#"
[pack]
name = "test"
version = "1.0.0"
description = "test"
[[sources]]
name = "test"
paths = ["/test"]
format = "xml"
"#;
        let manifest: PackManifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_validate_regex_without_pattern() {
        let toml_str = r#"
[pack]
name = "test"
version = "1.0.0"
description = "test"
[[sources]]
name = "test"
paths = ["/test"]
format = "regex"
"#;
        let manifest: PackManifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_validate_multiline_without_start() {
        let toml_str = r#"
[pack]
name = "test"
version = "1.0.0"
description = "test"
[[sources]]
name = "test"
paths = ["/test"]
format = "json"
multiline = true
"#;
        let manifest: PackManifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_field_type() {
        let toml_str = r#"
[pack]
name = "test"
version = "1.0.0"
description = "test"
[[sources]]
name = "test"
paths = ["/test"]
format = "json"
[schema]
fields = [
    { name = "field1", type = "xml_element" },
]
"#;
        let manifest: PackManifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_is_valid_semver() {
        assert!(super::is_valid_semver("1.0.0"));
        assert!(super::is_valid_semver("0.1.0"));
        assert!(super::is_valid_semver("10.20.30"));
        assert!(super::is_valid_semver("1.0.0-beta"));
        assert!(!super::is_valid_semver("1.0"));
        assert!(!super::is_valid_semver("abc"));
        assert!(!super::is_valid_semver(""));
    }
}
