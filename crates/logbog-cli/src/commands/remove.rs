use crate::output;
use logbog_core::Config;
use logbog_packs::PackRegistry;
use std::path::Path;

pub fn run(config_path: &str, pack_name: &str) -> anyhow::Result<()> {
    let path = Path::new(config_path);
    if !path.exists() {
        output::error("LogBog is not initialized. Run 'logbog init' first.");
        return Ok(());
    }

    let config = Config::load(path)?;
    let packs_dir = config.server.data_dir.join("packs");
    let mut registry = PackRegistry::new(&packs_dir);
    registry.scan()?;

    if !registry.is_installed(pack_name) {
        output::error(&format!("Pack '{pack_name}' is not installed."));
        return Ok(());
    }

    let pack_dir = packs_dir.join(pack_name);
    std::fs::remove_dir_all(&pack_dir)?;

    output::success(&format!("Pack '{pack_name}' removed successfully."));

    // Re-scan to confirm
    registry.scan()?;
    output::info(&format!("Total packs installed: {}", registry.count()));

    Ok(())
}
