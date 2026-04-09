use crate::manifest::PackManifest;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Registre local des packs installés.
pub struct PackRegistry {
    /// Répertoire racine des packs.
    packs_dir: PathBuf,
    /// Packs chargés en mémoire.
    packs: HashMap<String, PackManifest>,
}

impl PackRegistry {
    /// Crée un nouveau registre dans le répertoire spécifié.
    pub fn new(packs_dir: &Path) -> Self {
        Self {
            packs_dir: packs_dir.to_path_buf(),
            packs: HashMap::new(),
        }
    }

    /// Scanne le répertoire des packs et charge les manifests.
    pub fn scan(&mut self) -> logbog_core::Result<()> {
        self.packs.clear();

        if !self.packs_dir.exists() {
            std::fs::create_dir_all(&self.packs_dir)?;
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.packs_dir)? {
            let entry = entry?;
            let pack_toml = entry.path().join("pack.toml");
            if pack_toml.exists() {
                match PackManifest::load(&pack_toml) {
                    Ok(manifest) => {
                        info!(pack = %manifest.pack.name, "Loaded pack");
                        self.packs.insert(manifest.pack.name.clone(), manifest);
                    }
                    Err(e) => {
                        warn!(
                            path = %pack_toml.display(),
                            error = %e,
                            "Failed to load pack"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Retourne un pack par nom.
    pub fn get(&self, name: &str) -> Option<&PackManifest> {
        self.packs.get(name)
    }

    /// Liste tous les packs installés.
    pub fn list(&self) -> Vec<&PackManifest> {
        self.packs.values().collect()
    }

    /// Vérifie si un pack est installé.
    pub fn is_installed(&self, name: &str) -> bool {
        self.packs.contains_key(name)
    }

    /// Nombre de packs installés.
    pub fn count(&self) -> usize {
        self.packs.len()
    }

    /// Retourne le chemin du répertoire des packs.
    pub fn packs_dir(&self) -> &Path {
        &self.packs_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_registry_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let mut registry = PackRegistry::new(tmp.path());
        registry.scan().unwrap();
        assert_eq!(registry.count(), 0);
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_registry_scan_pack() {
        let tmp = tempfile::tempdir().unwrap();
        let pack_dir = tmp.path().join("nginx");
        fs::create_dir_all(&pack_dir).unwrap();
        fs::write(
            pack_dir.join("pack.toml"),
            r#"
[pack]
name = "nginx"
version = "1.0.0"
description = "Nginx logs"
[[sources]]
name = "access"
paths = ["/var/log/nginx/access.log"]
format = "regex"
pattern = '(?P<client_ip>\S+) - (?P<remote_user>\S+) \[(?P<timestamp>[^\]]+)\] "(?P<method>\S+) (?P<uri>\S+) HTTP/\S+" (?P<status>\d+) (?P<body_bytes>\d+)'
"#,
        )
        .unwrap();

        let mut registry = PackRegistry::new(tmp.path());
        registry.scan().unwrap();
        assert_eq!(registry.count(), 1);
        assert!(registry.is_installed("nginx"));
        assert!(!registry.is_installed("apache"));

        let pack = registry.get("nginx").unwrap();
        assert_eq!(pack.pack.version, "1.0.0");
    }

    #[test]
    fn test_registry_creates_dir_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let packs_dir = tmp.path().join("nonexistent/packs");
        let mut registry = PackRegistry::new(&packs_dir);
        registry.scan().unwrap();
        assert!(packs_dir.exists());
    }
}
