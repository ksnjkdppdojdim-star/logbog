use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::state::AppState;
use crate::websocket;

/// Build the complete API router with all endpoints.
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        // Logs
        .route("/api/v1/logs", get(handlers::logs::list_logs))
        .route("/api/v1/logs/search", post(handlers::logs::search_logs))
        .route("/api/v1/logs/count", get(handlers::logs::count_logs))
        .route("/api/v1/logs/stats", get(handlers::logs::log_stats))
        // Incidents / Correlations
        .route(
            "/api/v1/incidents",
            get(handlers::incidents::list_incidents),
        )
        .route(
            "/api/v1/incidents/{id}/timeline",
            get(handlers::incidents::incident_timeline),
        )
        // Status
        .route("/api/v1/status", get(handlers::status::service_status))
        .route("/api/v1/health", get(handlers::status::health_check))
        // WebSocket live tail
        .route("/api/v1/ws/tail", get(websocket::ws_tail_handler));

    api.layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
