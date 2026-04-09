use crate::output;
use logbog_core::Config;
use std::path::{Path, PathBuf};

pub fn run(data_dir: &str, port: u16) -> anyhow::Result<()> {
    let config_path = Config::default_path();

    if config_path.exists() {
        output::warn(&format!(
            "Configuration file '{}' already exists. Use --force to overwrite.",
            config_path.display()
        ));
        return Ok(());
    }

    output::banner();
    output::info("Initializing LogBog...");

    // Créer la config avec les valeurs personnalisées
    let mut config = Config::default();
    config.server.data_dir = PathBuf::from(data_dir);
    config.server.port = port;

    // Sauvegarder la config
    config.save(&config_path)?;
    output::success(&format!("Configuration created: {}", config_path.display()));

    // Créer le répertoire de données
    let data_path = Path::new(data_dir);
    std::fs::create_dir_all(data_path)?;
    output::success(&format!("Data directory created: {data_dir}"));

    // Créer le répertoire des packs
    let packs_dir = data_path.join("packs");
    std::fs::create_dir_all(&packs_dir)?;
    output::success(&format!("Packs directory created: {}", packs_dir.display()));

    // Détecter les services installés
    output::header("Detected services");
    detect_services();

    output::header("Next steps");
    output::info("Install log packs for your services:");
    println!("  logbog install nginx php-fpm mysql");
    output::info("Then start LogBog:");
    println!("  logbog start");

    Ok(())
}

/// Détecte les services courants installés sur le serveur.
fn detect_services() {
    let checks = [
        ("nginx", &["/etc/nginx", "/var/log/nginx"] as &[&str]),
        (
            "apache",
            &["/etc/apache2", "/etc/httpd", "/var/log/apache2"],
        ),
        ("php-fpm", &["/etc/php", "/var/log/php-fpm"]),
        ("mysql", &["/etc/mysql", "/var/log/mysql"]),
        ("postgresql", &["/etc/postgresql", "/var/log/postgresql"]),
        ("redis", &["/etc/redis", "/var/log/redis"]),
        ("mongodb", &["/etc/mongod.conf"]),
        ("docker", &["/var/run/docker.sock"]),
    ];

    let mut found = false;
    for (name, paths) in &checks {
        if paths.iter().any(|p| Path::new(p).exists()) {
            output::success(&format!("{name} detected"));
            found = true;
        }
    }

    if !found {
        output::info("No common services detected. You can install packs manually.");
    }
}
