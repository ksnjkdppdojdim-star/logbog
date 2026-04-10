//! Serveur API REST pour LogBog.
//!
//! Fournit des endpoints pour interroger les logs, les corrélations,
//! les incidents et le statut du service.

pub mod handlers;
pub mod routes;
pub mod state;
pub mod websocket;

pub use routes::build_router;
pub use state::AppState;
