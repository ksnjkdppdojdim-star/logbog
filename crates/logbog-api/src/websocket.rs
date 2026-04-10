use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use tokio::sync::broadcast;
use tracing::info;

use crate::state::AppState;

/// WebSocket handler for live tail streaming.
pub async fn ws_tail_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let rx = state.live_tx.subscribe();
    ws.on_upgrade(move |socket| handle_ws(socket, rx))
}

async fn handle_ws(mut socket: WebSocket, mut rx: broadcast::Receiver<String>) {
    info!("WebSocket client connected for live tail");

    loop {
        tokio::select! {
            // Forward log lines from broadcast channel to WebSocket
            result = rx.recv() => {
                match result {
                    Ok(line) => {
                        if socket.send(Message::Text(line.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        let msg = format!("{{\"warning\": \"skipped {n} messages\"}}");
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            // Check for client disconnect
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {} // Ignore other messages
                }
            }
        }
    }

    info!("WebSocket client disconnected");
}
