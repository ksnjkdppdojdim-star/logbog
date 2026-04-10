use axum::{Json, extract::State};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub version: String,
    pub logs_stored: u64,
    pub storage_bytes: u64,
    pub correlation_window_size: usize,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
}

/// GET /api/v1/status — service status
pub async fn service_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let store = state.store.lock().unwrap();
    let correlator = state.correlator.lock().unwrap();

    let total = store.count().unwrap_or(0);
    let size = store.storage_size();

    Json(StatusResponse {
        status: "running".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        logs_stored: total,
        storage_bytes: size,
        correlation_window_size: correlator.window_size(),
    })
}

/// GET /api/v1/health — health check
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}
