use crate::output;
use logbog_core::Config;
use logbog_storage::{LogQuery, LogStore};
use std::path::Path;

pub fn run(
    config_path: &str,
    sql: Option<&str>,
    source: Option<&str>,
    pack: Option<&str>,
    level: Option<&str>,
    search: Option<&str>,
    limit: usize,
) -> anyhow::Result<()> {
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

    if let Some(sql) = sql {
        // Raw SQL mode
        let entries = store.query_sql(sql)?;
        for entry in &entries {
            print_entry(entry);
        }
        output::info(&format!("{} results", entries.len()));
    } else {
        // Structured query mode
        let filter = LogQuery {
            source: source.map(String::from),
            pack: pack.map(String::from),
            level_min: level.map(logbog_core::LogLevel::from_str_loose),
            search: search.map(String::from),
            limit,
            ..Default::default()
        };

        let entries = store.query(&filter)?;
        for entry in &entries {
            print_entry(entry);
        }
        output::info(&format!("{} results", entries.len()));
    }

    Ok(())
}

fn print_entry(entry: &logbog_core::LogEntry) {
    use colored::Colorize;

    let level_colored = match entry.level {
        logbog_core::LogLevel::Fatal | logbog_core::LogLevel::Error => {
            entry.level.as_str().red().bold().to_string()
        }
        logbog_core::LogLevel::Warn => entry.level.as_str().yellow().to_string(),
        logbog_core::LogLevel::Info => entry.level.as_str().green().to_string(),
        logbog_core::LogLevel::Debug => entry.level.as_str().blue().to_string(),
        logbog_core::LogLevel::Trace => entry.level.as_str().dimmed().to_string(),
    };

    println!(
        "{} [{}] {}/{}: {}",
        entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
        level_colored,
        entry.pack,
        entry.source,
        entry.message
    );
}
