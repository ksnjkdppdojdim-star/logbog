use crate::output;
use logbog_core::{Config, ServiceStatus};
use std::path::Path;

pub fn run(config_path: &str) -> anyhow::Result<()> {
    let path = Path::new(config_path);

    if !path.exists() {
        output::error("LogBog is not initialized. Run 'logbog init' first.");
        return Ok(());
    }

    let config = Config::load(path)?;
    let status = get_status(&config);

    output::header("LogBog Status");

    let running_str = if status.running {
        "RUNNING".to_string()
    } else {
        "STOPPED".to_string()
    };
    output::item("Service", &running_str);
    output::item("Version", &status.version);

    if let Some(uptime) = status.uptime_seconds {
        output::item("Uptime", &format_duration(uptime));
    }

    output::header("Packs");
    if status.packs_installed.is_empty() {
        output::info("No packs installed. Run 'logbog install <pack>' to add one.");
    } else {
        for pack in &status.packs_installed {
            let active = if status.packs_active.contains(pack) {
                "(active)"
            } else {
                "(inactive)"
            };
            output::item(pack, active);
        }
    }

    output::header("Storage");
    output::item("Engine", &format!("{:?}", config.storage.engine));
    output::item("Data dir", &config.server.data_dir.display().to_string());
    output::item(
        "Retention",
        &format!("{} days", config.storage.retention_days),
    );

    // Try to open DuckDB for live stats
    let db_path = config.server.data_dir.join("logbog.duckdb");
    if db_path.exists() {
        if let Ok(store) = logbog_storage::LogStore::open(&db_path)
            && let Ok(stats) = store.stats() {
                output::item("Logs stored", &format_number(stats.total_logs));
                output::item("Storage used", &format_bytes(stats.db_size_bytes));
                for (source, count) in &stats.sources {
                    output::item(&format!("  {source}"), &format_number(*count));
                }
            }
    } else {
        output::item("Logs ingested", &format_number(status.logs_ingested));
        output::item("Storage used", &format_bytes(status.storage_bytes));
    }

    output::header("Features");
    output::item(
        "Correlation",
        if config.correlation.enabled {
            "enabled"
        } else {
            "disabled"
        },
    );
    output::item(
        "Anomaly detection",
        if config.intelligence.anomaly_detection {
            "enabled"
        } else {
            "disabled"
        },
    );
    output::item(
        "LLM integration",
        if config.intelligence.llm.enabled {
            &config.intelligence.llm.provider
        } else {
            "disabled"
        },
    );
    output::item(
        "PII detection",
        if config.compliance.pii_detection {
            "enabled"
        } else {
            "disabled"
        },
    );

    println!();
    Ok(())
}

fn get_status(config: &Config) -> ServiceStatus {
    let mut status = ServiceStatus::default();

    // Scanner les packs installés
    let packs_dir = config.server.data_dir.join("packs");
    if let Ok(entries) = std::fs::read_dir(&packs_dir) {
        for entry in entries.flatten() {
            if entry.path().join("pack.toml").exists()
                && let Some(name) = entry.file_name().to_str()
            {
                status.packs_installed.push(name.to_string());
            }
        }
    }

    status
}

fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
