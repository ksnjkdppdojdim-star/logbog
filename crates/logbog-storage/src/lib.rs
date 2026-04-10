//! Couche de stockage pour LogBog — implémentation DuckDB.
//!
//! Stocke les logs dans une base DuckDB locale, avec support
//! pour les requêtes SQL, le batch insert et la rétention.

use chrono::{DateTime, Utc};
use duckdb::{Connection, params};
use logbog_core::{LogEntry, LogLevel};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Statistiques de stockage.
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_logs: u64,
    pub sources: Vec<(String, u64)>,
    pub db_size_bytes: u64,
    pub oldest_entry: Option<DateTime<Utc>>,
    pub newest_entry: Option<DateTime<Utc>>,
}

/// Filtre structuré pour les requêtes de logs.
#[derive(Debug, Clone, Default)]
pub struct LogQuery {
    pub source: Option<String>,
    pub pack: Option<String>,
    pub level_min: Option<LogLevel>,
    pub search: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: usize,
    pub offset: usize,
}

/// Stockage des logs basé sur DuckDB.
pub struct LogStore {
    conn: Connection,
    db_path: PathBuf,
}

impl LogStore {
    /// Ouvre (ou crée) une base DuckDB au chemin spécifié.
    pub fn open(path: &Path) -> logbog_core::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)
            .map_err(|e| logbog_core::Error::Storage(format!("Failed to open DuckDB: {e}")))?;
        let store = Self {
            conn,
            db_path: path.to_path_buf(),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Crée une base DuckDB en mémoire (pour les tests).
    pub fn open_in_memory() -> logbog_core::Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| {
            logbog_core::Error::Storage(format!("Failed to open in-memory DB: {e}"))
        })?;
        let store = Self {
            conn,
            db_path: PathBuf::from(":memory:"),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Initialise le schéma de la base.
    fn init_schema(&self) -> logbog_core::Result<()> {
        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS logs (
                id VARCHAR PRIMARY KEY,
                timestamp VARCHAR NOT NULL,
                source VARCHAR NOT NULL,
                host VARCHAR NOT NULL,
                level VARCHAR NOT NULL,
                message VARCHAR,
                fields VARCHAR,
                raw VARCHAR,
                pack VARCHAR NOT NULL,
                ingested_at VARCHAR DEFAULT current_timestamp
            );
        ",
            )
            .map_err(|e| logbog_core::Error::Storage(format!("Schema init failed: {e}")))?;

        info!("Storage schema initialized");
        Ok(())
    }

    /// Insère une entrée de log.
    pub fn insert(&self, entry: &LogEntry) -> logbog_core::Result<()> {
        let fields_json = serde_json::to_string(&entry.fields).unwrap_or_else(|_| "{}".to_string());
        let ts = entry.timestamp.to_rfc3339();

        self.conn
            .execute(
                "INSERT INTO logs (id, timestamp, source, host, level, message, fields, raw, pack) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    entry.id.to_string(),
                    ts,
                    entry.source,
                    entry.host,
                    entry.level.as_str(),
                    entry.message,
                    fields_json,
                    entry.raw,
                    entry.pack,
                ],
            )
            .map_err(|e| logbog_core::Error::Storage(format!("Insert failed: {e}")))?;

        Ok(())
    }

    /// Insère un batch d'entrées dans une transaction.
    pub fn insert_batch(&self, entries: &[LogEntry]) -> logbog_core::Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        self.conn
            .execute_batch("BEGIN TRANSACTION")
            .map_err(|e| logbog_core::Error::Storage(format!("Begin transaction: {e}")))?;

        let mut stmt = self
            .conn
            .prepare(
                "INSERT INTO logs (id, timestamp, source, host, level, message, fields, raw, pack) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .map_err(|e| logbog_core::Error::Storage(format!("Prepare insert: {e}")))?;

        let mut count = 0;
        for entry in entries {
            let fields_json =
                serde_json::to_string(&entry.fields).unwrap_or_else(|_| "{}".to_string());
            let ts = entry.timestamp.to_rfc3339();

            stmt.execute(params![
                entry.id.to_string(),
                ts,
                entry.source,
                entry.host,
                entry.level.as_str(),
                entry.message,
                fields_json,
                entry.raw,
                entry.pack,
            ])
            .map_err(|e| logbog_core::Error::Storage(format!("Batch insert row: {e}")))?;
            count += 1;
        }

        drop(stmt);

        self.conn
            .execute_batch("COMMIT")
            .map_err(|e| logbog_core::Error::Storage(format!("Commit transaction: {e}")))?;

        debug!(count = count, "Batch inserted logs");
        Ok(count)
    }

    /// Exécute une requête SQL brute et retourne les entrées.
    pub fn query_sql(&self, sql: &str) -> logbog_core::Result<Vec<LogEntry>> {
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| logbog_core::Error::Storage(format!("Query prepare: {e}")))?;

        let rows = stmt
            .query_map([], |row| Ok(row_to_log_entry(row)))
            .map_err(|e| logbog_core::Error::Storage(format!("Query execute: {e}")))?;

        let mut entries = Vec::new();
        for row in rows {
            match row {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to read row");
                }
            }
        }

        Ok(entries)
    }

    /// Requête structurée avec filtres.
    pub fn query(&self, filter: &LogQuery) -> logbog_core::Result<Vec<LogEntry>> {
        let mut sql = String::from(
            "SELECT id, timestamp, source, host, level, message, fields, raw, pack FROM logs WHERE 1=1",
        );
        let mut param_values: Vec<String> = Vec::new();

        if let Some(ref source) = filter.source {
            param_values.push(source.clone());
            sql.push_str(&format!(" AND source = ${}", param_values.len()));
        }
        if let Some(ref pack) = filter.pack {
            param_values.push(pack.clone());
            sql.push_str(&format!(" AND pack = ${}", param_values.len()));
        }
        if let Some(ref level) = filter.level_min {
            param_values.push(level.as_str().to_string());
            // Filter by level - use string comparison for simplicity
            let levels: Vec<&str> = match level {
                LogLevel::Trace => vec!["trace", "debug", "info", "warn", "error", "fatal"],
                LogLevel::Debug => vec!["debug", "info", "warn", "error", "fatal"],
                LogLevel::Info => vec!["info", "warn", "error", "fatal"],
                LogLevel::Warn => vec!["warn", "error", "fatal"],
                LogLevel::Error => vec!["error", "fatal"],
                LogLevel::Fatal => vec!["fatal"],
            };
            let level_list = levels
                .iter()
                .map(|l| format!("'{l}'"))
                .collect::<Vec<_>>()
                .join(",");
            // Remove the last param_values entry since we inline the values
            param_values.pop();
            sql.push_str(&format!(" AND level IN ({level_list})"));
        }
        if let Some(ref search) = filter.search {
            param_values.push(format!("%{search}%"));
            sql.push_str(&format!(" AND message LIKE ${}", param_values.len()));
        }
        if let Some(ref since) = filter.since {
            param_values.push(since.to_rfc3339());
            sql.push_str(&format!(" AND timestamp >= ${}", param_values.len()));
        }
        if let Some(ref until) = filter.until {
            param_values.push(until.to_rfc3339());
            sql.push_str(&format!(" AND timestamp <= ${}", param_values.len()));
        }

        sql.push_str(" ORDER BY timestamp DESC");

        let limit = if filter.limit == 0 { 100 } else { filter.limit };
        sql.push_str(&format!(" LIMIT {limit}"));
        if filter.offset > 0 {
            sql.push_str(&format!(" OFFSET {}", filter.offset));
        }

        // Build parameterized query
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| logbog_core::Error::Storage(format!("Query prepare: {e}")))?;

        let params_refs: Vec<&dyn duckdb::ToSql> = param_values
            .iter()
            .map(|v| v as &dyn duckdb::ToSql)
            .collect();

        let rows = stmt
            .query_map(params_refs.as_slice(), |row| Ok(row_to_log_entry(row)))
            .map_err(|e| logbog_core::Error::Storage(format!("Query execute: {e}")))?;

        let mut entries = Vec::new();
        for row in rows {
            match row {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to read row");
                }
            }
        }

        Ok(entries)
    }

    /// Nombre total de logs stockés.
    pub fn count(&self) -> logbog_core::Result<u64> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM logs")
            .map_err(|e| logbog_core::Error::Storage(e.to_string()))?;
        let count: u64 = stmt
            .query_row([], |row| row.get(0))
            .map_err(|e| logbog_core::Error::Storage(e.to_string()))?;
        Ok(count)
    }

    /// Nombre de logs par source.
    pub fn count_by_source(&self) -> logbog_core::Result<Vec<(String, u64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT source, COUNT(*) as cnt FROM logs GROUP BY source ORDER BY cnt DESC")
            .map_err(|e| logbog_core::Error::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, u64>(1).unwrap_or(0),
                ))
            })
            .map_err(|e| logbog_core::Error::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows.flatten() {
            results.push(row);
        }
        Ok(results)
    }

    /// Nombre de logs par pack.
    pub fn count_by_pack(&self) -> logbog_core::Result<Vec<(String, u64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT pack, COUNT(*) as cnt FROM logs GROUP BY pack ORDER BY cnt DESC")
            .map_err(|e| logbog_core::Error::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, u64>(1).unwrap_or(0),
                ))
            })
            .map_err(|e| logbog_core::Error::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows.flatten() {
            results.push(row);
        }
        Ok(results)
    }

    /// Taille de la base sur disque (en bytes).
    pub fn storage_size(&self) -> u64 {
        if self.db_path.to_str() == Some(":memory:") {
            return 0;
        }
        std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    /// Statistiques complètes du stockage.
    pub fn stats(&self) -> logbog_core::Result<StorageStats> {
        let total_logs = self.count()?;
        let sources = self.count_by_source()?;
        let db_size_bytes = self.storage_size();

        let oldest = self
            .conn
            .prepare("SELECT MIN(timestamp) FROM logs")
            .ok()
            .and_then(|mut s| {
                s.query_row([], |row| row.get::<_, Option<String>>(0))
                    .ok()
                    .flatten()
            })
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let newest = self
            .conn
            .prepare("SELECT MAX(timestamp) FROM logs")
            .ok()
            .and_then(|mut s| {
                s.query_row([], |row| row.get::<_, Option<String>>(0))
                    .ok()
                    .flatten()
            })
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(StorageStats {
            total_logs,
            sources,
            db_size_bytes,
            oldest_entry: oldest,
            newest_entry: newest,
        })
    }

    /// Applique la rétention : supprime les logs plus vieux que `days` jours.
    /// Retourne le nombre de lignes supprimées.
    pub fn apply_retention(&self, days: u32) -> logbog_core::Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(i64::from(days));
        let cutoff_str = cutoff.to_rfc3339();

        // Count first
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM logs WHERE timestamp < ?")
            .map_err(|e| logbog_core::Error::Storage(e.to_string()))?;
        let count: u64 = stmt
            .query_row(params![cutoff_str], |row| row.get(0))
            .map_err(|e| logbog_core::Error::Storage(e.to_string()))?;

        if count > 0 {
            self.conn
                .execute("DELETE FROM logs WHERE timestamp < ?", params![cutoff_str])
                .map_err(|e| logbog_core::Error::Storage(e.to_string()))?;
            info!(deleted = count, days = days, "Applied retention policy");
        }

        Ok(count)
    }

    /// Retourne le chemin de la base de données.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

/// Convertit une ligne DuckDB en LogEntry.
fn row_to_log_entry(row: &duckdb::Row<'_>) -> LogEntry {
    let id_str: String = row.get(0).unwrap_or_default();
    let ts_str: String = row.get(1).unwrap_or_default();
    let source: String = row.get(2).unwrap_or_default();
    let host: String = row.get(3).unwrap_or_default();
    let level_str: String = row.get(4).unwrap_or_default();
    let message: String = row.get(5).unwrap_or_default();
    let fields_json: String = row.get(6).unwrap_or_else(|_| "{}".to_string());
    let raw: String = row.get(7).unwrap_or_default();
    let pack: String = row.get(8).unwrap_or_default();

    let id = id_str.parse().unwrap_or_else(|_| ulid::Ulid::new());
    let timestamp = DateTime::parse_from_rfc3339(&ts_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let level = LogLevel::from_str_loose(&level_str);
    let fields = serde_json::from_str(&fields_json).unwrap_or_default();

    LogEntry {
        id,
        timestamp,
        source,
        host,
        level,
        message,
        fields,
        raw,
        pack,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_entry(source: &str, pack: &str, level: LogLevel, message: &str) -> LogEntry {
        LogEntry {
            id: ulid::Ulid::new(),
            timestamp: Utc::now(),
            source: source.into(),
            host: "test-host".into(),
            level,
            message: message.into(),
            fields: HashMap::new(),
            raw: message.into(),
            pack: pack.into(),
        }
    }

    #[test]
    fn test_open_in_memory() {
        let store = LogStore::open_in_memory().unwrap();
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_insert_and_count() {
        let store = LogStore::open_in_memory().unwrap();

        let entry = make_entry("access", "nginx", LogLevel::Info, "GET /api/users 200");
        store.insert(&entry).unwrap();

        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn test_insert_batch() {
        let store = LogStore::open_in_memory().unwrap();

        let entries: Vec<LogEntry> = (0..100)
            .map(|i| make_entry("access", "nginx", LogLevel::Info, &format!("Request {i}")))
            .collect();

        let count = store.insert_batch(&entries).unwrap();
        assert_eq!(count, 100);
        assert_eq!(store.count().unwrap(), 100);
    }

    #[test]
    fn test_insert_batch_empty() {
        let store = LogStore::open_in_memory().unwrap();
        let count = store.insert_batch(&[]).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_query_by_source() {
        let store = LogStore::open_in_memory().unwrap();

        store
            .insert(&make_entry("access", "nginx", LogLevel::Info, "GET /"))
            .unwrap();
        store
            .insert(&make_entry(
                "error",
                "nginx",
                LogLevel::Error,
                "upstream timeout",
            ))
            .unwrap();
        store
            .insert(&make_entry("error", "php-fpm", LogLevel::Fatal, "segfault"))
            .unwrap();

        let results = store
            .query(&LogQuery {
                source: Some("error".into()),
                limit: 100,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_by_pack() {
        let store = LogStore::open_in_memory().unwrap();

        store
            .insert(&make_entry("access", "nginx", LogLevel::Info, "GET /"))
            .unwrap();
        store
            .insert(&make_entry("error", "php-fpm", LogLevel::Error, "error"))
            .unwrap();

        let results = store
            .query(&LogQuery {
                pack: Some("nginx".into()),
                limit: 100,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pack, "nginx");
    }

    #[test]
    fn test_query_by_level() {
        let store = LogStore::open_in_memory().unwrap();

        store
            .insert(&make_entry("app", "test", LogLevel::Info, "info msg"))
            .unwrap();
        store
            .insert(&make_entry("app", "test", LogLevel::Warn, "warn msg"))
            .unwrap();
        store
            .insert(&make_entry("app", "test", LogLevel::Error, "error msg"))
            .unwrap();

        let results = store
            .query(&LogQuery {
                level_min: Some(LogLevel::Warn),
                limit: 100,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_search() {
        let store = LogStore::open_in_memory().unwrap();

        store
            .insert(&make_entry(
                "app",
                "test",
                LogLevel::Error,
                "upstream timeout",
            ))
            .unwrap();
        store
            .insert(&make_entry("app", "test", LogLevel::Info, "connection OK"))
            .unwrap();

        let results = store
            .query(&LogQuery {
                search: Some("timeout".into()),
                limit: 100,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].message.contains("timeout"));
    }

    #[test]
    fn test_count_by_source() {
        let store = LogStore::open_in_memory().unwrap();

        store
            .insert(&make_entry("access", "nginx", LogLevel::Info, "a"))
            .unwrap();
        store
            .insert(&make_entry("access", "nginx", LogLevel::Info, "b"))
            .unwrap();
        store
            .insert(&make_entry("error", "nginx", LogLevel::Error, "c"))
            .unwrap();

        let counts = store.count_by_source().unwrap();
        assert_eq!(counts.len(), 2);
        assert_eq!(counts[0], ("access".into(), 2));
        assert_eq!(counts[1], ("error".into(), 1));
    }

    #[test]
    fn test_stats() {
        let store = LogStore::open_in_memory().unwrap();

        store
            .insert(&make_entry("access", "nginx", LogLevel::Info, "test"))
            .unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total_logs, 1);
        assert_eq!(stats.sources.len(), 1);
        assert!(stats.oldest_entry.is_some());
        assert!(stats.newest_entry.is_some());
    }

    #[test]
    fn test_retention() {
        let store = LogStore::open_in_memory().unwrap();

        // Insert an entry with old timestamp
        let mut old_entry = make_entry("app", "test", LogLevel::Info, "old");
        old_entry.timestamp = Utc::now() - chrono::Duration::days(60);
        store.insert(&old_entry).unwrap();

        // Insert a recent entry
        store
            .insert(&make_entry("app", "test", LogLevel::Info, "new"))
            .unwrap();

        assert_eq!(store.count().unwrap(), 2);

        let deleted = store.apply_retention(30).unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn test_query_sql() {
        let store = LogStore::open_in_memory().unwrap();

        store
            .insert(&make_entry("access", "nginx", LogLevel::Info, "GET /"))
            .unwrap();

        let results = store
            .query_sql("SELECT id, timestamp, source, host, level, message, fields, raw, pack FROM logs LIMIT 10")
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "access");
    }

    #[test]
    fn test_fields_roundtrip() {
        let store = LogStore::open_in_memory().unwrap();

        let mut entry = make_entry("access", "nginx", LogLevel::Info, "GET /api");
        entry.fields.insert("status".into(), serde_json::json!(200));
        entry
            .fields
            .insert("method".into(), serde_json::json!("GET"));
        store.insert(&entry).unwrap();

        let results = store
            .query_sql("SELECT id, timestamp, source, host, level, message, fields, raw, pack FROM logs LIMIT 1")
            .unwrap();
        assert_eq!(
            results[0].fields.get("status"),
            Some(&serde_json::json!(200))
        );
        assert_eq!(
            results[0].fields.get("method"),
            Some(&serde_json::json!("GET"))
        );
    }

    #[test]
    fn test_open_persistent() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("logbog.duckdb");

        {
            let store = LogStore::open(&db_path).unwrap();
            store
                .insert(&make_entry("app", "test", LogLevel::Info, "persist"))
                .unwrap();
            assert_eq!(store.count().unwrap(), 1);
        }

        // Reopen
        {
            let store = LogStore::open(&db_path).unwrap();
            assert_eq!(store.count().unwrap(), 1);
        }
    }
}
