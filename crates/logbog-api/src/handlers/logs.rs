use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use logbog_core::LogEntry;
use logbog_storage::LogQuery;

#[derive(Debug, Deserialize)]
pub struct LogListParams {
    pub source: Option<String>,
    pub pack: Option<String>,
    pub level: Option<String>,
    pub search: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Serialize)]
pub struct LogListResponse {
    pub logs: Vec<LogEntry>,
    pub total: u64,
}

#[derive(Deserialize)]
pub struct SearchRequest {
    pub sql: Option<String>,
    pub source: Option<String>,
    pub pack: Option<String>,
    pub level: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct CountResponse {
    pub total: u64,
    pub by_source: Vec<(String, u64)>,
    pub by_pack: Vec<(String, u64)>,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub total_logs: u64,
    pub sources: Vec<(String, u64)>,
    pub db_size_bytes: u64,
    pub oldest_entry: Option<String>,
    pub newest_entry: Option<String>,
}

/// GET /api/v1/logs — list logs with filters
pub async fn list_logs(
    State(state): State<AppState>,
    Query(params): Query<LogListParams>,
) -> Json<LogListResponse> {
    let store = state.store.lock().unwrap();

    let filter = LogQuery {
        source: params.source,
        pack: params.pack,
        level_min: params
            .level
            .map(|l| logbog_core::LogLevel::from_str_loose(&l)),
        search: params.search,
        since: params.since.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        }),
        until: params.until.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        }),
        limit: params.limit.unwrap_or(50),
        offset: params.offset.unwrap_or(0),
    };

    let total = store.count().unwrap_or(0);
    let logs = store.query(&filter).unwrap_or_default();

    Json(LogListResponse { logs, total })
}

/// POST /api/v1/logs/search — search logs (SQL or structured)
pub async fn search_logs(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Json<LogListResponse> {
    let store = state.store.lock().unwrap();

    let logs = if let Some(ref sql) = req.sql {
        store.query_sql(sql).unwrap_or_default()
    } else {
        let filter = LogQuery {
            source: req.source,
            pack: req.pack,
            level_min: req.level.map(|l| logbog_core::LogLevel::from_str_loose(&l)),
            search: req.search,
            limit: req.limit.unwrap_or(50),
            ..Default::default()
        };
        store.query(&filter).unwrap_or_default()
    };

    let total = logs.len() as u64;
    Json(LogListResponse { logs, total })
}

/// GET /api/v1/logs/count — count logs by source and pack
pub async fn count_logs(State(state): State<AppState>) -> Json<CountResponse> {
    let store = state.store.lock().unwrap();

    Json(CountResponse {
        total: store.count().unwrap_or(0),
        by_source: store.count_by_source().unwrap_or_default(),
        by_pack: store.count_by_pack().unwrap_or_default(),
    })
}

/// GET /api/v1/logs/stats — storage statistics
pub async fn log_stats(State(state): State<AppState>) -> Json<StatsResponse> {
    let store = state.store.lock().unwrap();

    let stats = store.stats().unwrap_or(logbog_storage::StorageStats {
        total_logs: 0,
        sources: Vec::new(),
        db_size_bytes: 0,
        oldest_entry: None,
        newest_entry: None,
    });

    Json(StatsResponse {
        total_logs: stats.total_logs,
        sources: stats.sources,
        db_size_bytes: stats.db_size_bytes,
        oldest_entry: stats.oldest_entry.map(|dt| dt.to_rfc3339()),
        newest_entry: stats.newest_entry.map(|dt| dt.to_rfc3339()),
    })
}
