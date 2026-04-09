use crate::output;
use logbog_core::Config;
use logbog_packs::PackRegistry;
use std::path::Path;

pub fn run(config_path: &str) -> anyhow::Result<()> {
    let path = Path::new(config_path);
    if !path.exists() {
        output::error("LogBog is not initialized. Run 'logbog init' first.");
        return Ok(());
    }

    let config = Config::load(path)?;
    let packs_dir = config.server.data_dir.join("packs");
    let mut registry = PackRegistry::new(&packs_dir);
    registry.scan()?;

    let packs = registry.list();

    if packs.is_empty() {
        output::info("No packs installed.");
        output::info("Install packs with: logbog install nginx php-fpm mysql");
        return Ok(());
    }

    output::header("Installed packs");
    for pack in &packs {
        let tags = pack.pack.tags.join(", ");
        let sources_count = pack.sources.len();
        let alerts_count = pack.alerts.len();

        println!(
            "  {} v{} - {} ({} sources, {} alerts) [{}]",
            pack.pack.name,
            pack.pack.version,
            pack.pack.description,
            sources_count,
            alerts_count,
            tags,
        );
    }

    println!();
    output::info(&format!("Total: {} pack(s)", packs.len()));

    Ok(())
}
