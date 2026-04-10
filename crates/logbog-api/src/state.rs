use logbog_engine::Correlator;
use logbog_storage::LogStore;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Shared application state for all API handlers.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<LogStore>>,
    pub correlator: Arc<Mutex<Correlator>>,
    /// Broadcast channel for live tail WebSocket streaming.
    pub live_tx: broadcast::Sender<String>,
}

impl AppState {
    pub fn new(store: LogStore, correlator: Correlator) -> Self {
        let (live_tx, _) = broadcast::channel(1024);
        Self {
            store: Arc::new(Mutex::new(store)),
            correlator: Arc::new(Mutex::new(correlator)),
            live_tx,
        }
    }
}
