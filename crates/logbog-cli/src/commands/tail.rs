use crate::output;
use logbog_core::Config;
use logbog_storage::{LogQuery, LogStore};
use std::path::Path;

pub fn run(config_path: &str, source: Option<&str>, pack: Option<&str>, n: usize) -> anyhow::Result<()> {
    let path = Path::new(config_path);
    if !path.exists() {
        output::error("LogBog is not initialized. Run 'logbog init' first.");
        return Ok(());
    }

    let config = Config::load(path)?;
    let db_path = config.server.data_dir.join("logbog.duckdb");

    if !db_path.exists() {
        output::warn("No data yet. Start LogBog first with 'logbog start'.");
        return Ok(());
    }

    let store = LogStore::open(&db_path)?;

    let filter = LogQuery {
        source: source.map(String::from),
        pack: pack.map(String::from),
        limit: n,
        ..Default::default()
    };

    let entries = store.query(&filter)?;

    // Print in chronological order (query returns newest first)
    for entry in entries.iter().rev() {
        print_tail_entry(entry);
    }

    Ok(())
}

fn print_tail_entry(entry: &logbog_core::LogEntry) {
    use colored::Colorize;

    let level_colored = match entry.level {
        logbog_core::LogLevel::Fatal | logbog_core::LogLevel::Error => {
            entry.level.as_str().red().bold().to_string()
        }
        logbog_core::LogLevel::Warn => entry.level.as_str().yellow().to_string(),
        logbog_core::LogLevel::Info => entry.level.as_str().green().to_string(),
        _ => entry.level.as_str().dimmed().to_string(),
    };

    println!(
        "{} {} {}/{} {}",
        entry.timestamp.format("%H:%M:%S").to_string().dimmed(),
        level_colored,
        entry.pack.cyan(),
        entry.source,
        entry.message,
    );
}
