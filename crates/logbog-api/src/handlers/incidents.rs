use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;

use crate::state::AppState;
use logbog_core::Correlation;

#[derive(Serialize)]
pub struct IncidentListResponse {
    pub incidents: Vec<Correlation>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct TimelineResponse {
    pub incident_id: String,
    pub entries: Vec<TimelineEntry>,
}

#[derive(Serialize)]
pub struct TimelineEntry {
    pub timestamp: String,
    pub pack: String,
    pub source: String,
    pub level: String,
    pub message: String,
}

/// GET /api/v1/incidents — list recent correlations/incidents
pub async fn list_incidents(State(_state): State<AppState>) -> Json<IncidentListResponse> {
    // TODO: Store correlations in DuckDB and query them
    // For now, return empty — correlations are computed in real-time
    Json(IncidentListResponse {
        incidents: Vec::new(),
        total: 0,
    })
}

/// GET /api/v1/incidents/:id/timeline — incident timeline
pub async fn incident_timeline(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Json<TimelineResponse> {
    // TODO: Load correlated entries from storage
    Json(TimelineResponse {
        incident_id: id,
        entries: Vec::new(),
    })
}
