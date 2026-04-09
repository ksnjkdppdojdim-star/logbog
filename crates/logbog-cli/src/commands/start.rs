use crate::output;
use logbog_core::Config;
use logbog_packs::PackRegistry;
use std::path::Path;

pub fn run(config_path: &str, _foreground: bool) -> anyhow::Result<()> {
    let path = Path::new(config_path);
    if !path.exists() {
        output::error("LogBog is not initialized. Run 'logbog init' first.");
        return Ok(());
    }

    let config = Config::load(path)?;
    let packs_dir = config.server.data_dir.join("packs");
    let mut registry = PackRegistry::new(&packs_dir);
    registry.scan()?;

    output::banner();

    if registry.count() == 0 {
        output::warn("No packs installed. Install packs first:");
        println!("  logbog install nginx php-fpm mysql");
        return Ok(());
    }

    output::info(&format!("Loaded {} pack(s)", registry.count()));

    for pack in registry.list() {
        output::item(&pack.pack.name, &format!("{} sources", pack.sources.len()));
    }

    output::info(&format!(
        "Dashboard: http://{}:{}",
        config.server.host, config.server.port
    ));

    // Phase 2 : ici on lancera le pipeline d'ingestion
    output::warn("Collection pipeline not yet implemented (Phase 2)");
    output::info("LogBog initialized and ready. Collection will start in Phase 2.");

    Ok(())
}
