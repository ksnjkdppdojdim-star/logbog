use crate::output;
use logbog_collector::bookmark::BookmarkManager;
use logbog_collector::file_watcher::FileWatcher;
use logbog_core::Config;
use logbog_packs::{PackEngine, PackRegistry};
use logbog_storage::LogStore;
use std::path::Path;

pub fn run(config_path: &str, foreground: bool) -> anyhow::Result<()> {
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

    // Load all packs into the engine
    let mut engine = PackEngine::new();
    for manifest in registry.list() {
        match engine.load_pack(manifest) {
            Ok(()) => output::success(&format!(
                "Pack '{}' loaded ({} sources)",
                manifest.pack.name,
                manifest.sources.len()
            )),
            Err(e) => output::warn(&format!(
                "Pack '{}' failed to load: {e}",
                manifest.pack.name
            )),
        }
    }

    output::info(&format!(
        "Engine ready: {} packs, {} sources",
        engine.pack_count(),
        engine.source_count()
    ));

    if !foreground {
        output::warn("Daemon mode not yet implemented. Running in foreground.");
    }

    // Build the tokio runtime and run the pipeline
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_pipeline(config, engine))?;

    Ok(())
}

async fn run_pipeline(config: Config, engine: PackEngine) -> anyhow::Result<()> {
    // Open storage
    let db_path = config.server.data_dir.join("logbog.duckdb");
    let store = LogStore::open(&db_path)?;
    output::success(&format!("Storage opened: {}", db_path.display()));

    // Create collection channel
    let (sender, mut receiver) = logbog_collector::channel(config.collector.batch_size);

    // Set up file watcher
    let bookmarks_path = config.server.data_dir.join("bookmarks.json");
    let bookmarks = BookmarkManager::new(bookmarks_path);
    let mut watcher = FileWatcher::new(sender.clone(), bookmarks);

    // Register all files from pack sources
    let watch_paths = engine.all_watch_paths();
    let mut file_count = 0;
    for (path_pattern, pack, source) in &watch_paths {
        // Skip non-file sources (like "journalctl")
        if !path_pattern.starts_with('/') {
            continue;
        }
        watcher.add_glob(path_pattern, pack.clone(), source.clone());
        file_count += 1;
    }

    if file_count > 0 {
        output::info(&format!("Watching {file_count} file pattern(s)"));
    }

    // Start syslog receiver if enabled
    if config.collector.syslog.enabled {
        let syslog_sender = sender.clone();
        let syslog_config = logbog_collector::syslog_receiver::SyslogReceiverConfig {
            udp_port: config.collector.syslog.udp_port,
            tcp_port: config.collector.syslog.tcp_port,
            pack: "syslog".into(),
            source: "syslog-receiver".into(),
        };
        let receiver =
            logbog_collector::syslog_receiver::SyslogReceiver::new(syslog_sender, syslog_config);
        tokio::spawn(async move {
            if let Err(e) = receiver.run().await {
                tracing::warn!(error = %e, "Syslog receiver stopped");
            }
        });
        output::info(&format!(
            "Syslog receiver: UDP/TCP port {}",
            config.collector.syslog.udp_port
        ));
    }

    // Start journal reader if systemd pack is installed
    let has_systemd = engine.pack_names().contains(&"systemd");
    if has_systemd {
        let journal_sender = sender.clone();
        let reader =
            logbog_collector::journal::JournalReader::new(journal_sender, vec![], "systemd".into());
        tokio::spawn(async move {
            if let Err(e) = reader.run().await {
                tracing::warn!(error = %e, "Journal reader stopped");
            }
        });
        output::info("Journal reader: all units");
    }

    // Drop the extra sender so the channel closes when all sources are done
    drop(sender);

    // Start file watcher in a background task
    tokio::spawn(async move {
        if let Err(e) = watcher.run().await {
            tracing::warn!(error = %e, "File watcher stopped");
        }
    });

    output::header("Pipeline running");
    output::info(&format!(
        "Dashboard: http://{}:{}",
        config.server.host, config.server.port
    ));
    output::info("Press Ctrl+C to stop");

    // Ingestion loop: receive raw lines, parse them, batch insert into storage
    let batch_size = config.collector.batch_size;
    let flush_interval = std::time::Duration::from_millis(config.collector.flush_interval_ms);
    let retention_days = config.storage.retention_days;

    let mut batch = Vec::with_capacity(batch_size);
    let mut total_ingested: u64 = 0;
    let mut last_flush = tokio::time::Instant::now();
    let mut last_retention = tokio::time::Instant::now();

    loop {
        // Try to receive with timeout for periodic flushing
        let timeout = tokio::time::timeout(flush_interval, receiver.recv()).await;

        match timeout {
            Ok(Some(raw_line)) => {
                // Parse the line using the appropriate pack/source parser
                if let Some(entry) =
                    engine.parse_line(&raw_line.content, &raw_line.pack, &raw_line.source)
                {
                    batch.push(entry);
                }

                // Flush when batch is full
                if batch.len() >= batch_size {
                    let count = store.insert_batch(&batch)?;
                    total_ingested += count as u64;
                    batch.clear();
                    last_flush = tokio::time::Instant::now();
                    tracing::debug!(count = count, total = total_ingested, "Flushed batch");
                }
            }
            Ok(None) => {
                // Channel closed - all sources done
                break;
            }
            Err(_) => {
                // Timeout - flush partial batch
                if !batch.is_empty() {
                    let count = store.insert_batch(&batch)?;
                    total_ingested += count as u64;
                    batch.clear();
                    last_flush = tokio::time::Instant::now();
                }
            }
        }

        // Periodic retention check (every hour)
        if last_retention.elapsed() > std::time::Duration::from_secs(3600) {
            if let Err(e) = store.apply_retention(retention_days) {
                tracing::warn!(error = %e, "Retention policy failed");
            }
            last_retention = tokio::time::Instant::now();
        }

        // Avoid busy loop warning
        let _ = last_flush;
    }

    // Final flush
    if !batch.is_empty() {
        let count = store.insert_batch(&batch)?;
        total_ingested += count as u64;
    }

    output::info(&format!("Total logs ingested: {total_ingested}"));
    Ok(())
}
