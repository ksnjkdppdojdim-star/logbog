use crate::output;
use clap::Subcommand;
use logbog_packs::PackManifest;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum PackAction {
    /// Validate a pack's manifest
    Validate {
        /// Path to the pack directory or pack.toml
        path: String,
    },

    /// Show pack information
    Info {
        /// Path to the pack directory or pack.toml
        path: String,
    },
}

pub fn run(action: PackAction) -> anyhow::Result<()> {
    match action {
        PackAction::Validate { path } => validate(&path),
        PackAction::Info { path } => info(&path),
    }
}

fn resolve_pack_toml(path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_dir() { p.join("pack.toml") } else { p }
}

fn validate(path: &str) -> anyhow::Result<()> {
    let pack_path = resolve_pack_toml(path);
    output::info(&format!("Validating {}...", pack_path.display()));

    match PackManifest::load(&pack_path) {
        Ok(manifest) => {
            output::success(&format!(
                "Pack '{}' v{} is valid.",
                manifest.pack.name, manifest.pack.version
            ));
            output::item("Sources", &manifest.sources.len().to_string());
            output::item("Alerts", &manifest.alerts.len().to_string());
            output::item("Correlations", &manifest.correlations.len().to_string());
        }
        Err(e) => {
            output::error(&format!("Validation failed: {e}"));
        }
    }

    Ok(())
}

fn info(path: &str) -> anyhow::Result<()> {
    let pack_path = resolve_pack_toml(path);
    let manifest = PackManifest::load(&pack_path)?;

    output::header(&format!("Pack: {}", manifest.pack.name));
    output::item("Version", &manifest.pack.version);
    output::item("Description", &manifest.pack.description);
    output::item("Author", &manifest.pack.author);
    output::item("License", &manifest.pack.license);
    output::item("Tags", &manifest.pack.tags.join(", "));

    output::header("Sources");
    for source in &manifest.sources {
        output::item(
            &source.name,
            &format!("{} ({})", source.paths.join(", "), source.format),
        );
    }

    if !manifest.alerts.is_empty() {
        output::header("Alerts");
        for alert in &manifest.alerts {
            output::item(
                &alert.name,
                &format!("[{}] {}", alert.severity, alert.description),
            );
        }
    }

    if !manifest.correlations.is_empty() {
        output::header("Correlations");
        for corr in &manifest.correlations {
            output::item(
                &corr.target_pack,
                &format!(
                    "match={} window={}",
                    corr.match_fields.join(","),
                    corr.window
                ),
            );
        }
    }

    println!();
    Ok(())
}
