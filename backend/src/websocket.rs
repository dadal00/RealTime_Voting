use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

use crate::state::AppState;

pub async fn websocket_handler(
    websocket: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    websocket.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: Arc<AppState>) {
    let user_count = state.concurrent_users.fetch_add(1, SeqCst) + 1;
    state
        .metrics
        .concurrent_users
        .set(user_count.try_into().unwrap());
    state.total_users.fetch_add(1, SeqCst);
    state.metrics.total_users.inc();
    debug!("New WebSocket connection. User count: {}", user_count);

    let mut rx = state.broadcast_tx.subscribe();
    let (ws_sender, mut ws_receiver) = socket.split();

    let ws_sender = Arc::new(Mutex::new(ws_sender));

    let initial = json!({
        "red": state.counters.red.load(SeqCst),
        "green": state.counters.green.load(SeqCst),
        "blue": state.counters.blue.load(SeqCst),
        "purple": state.counters.purple.load(SeqCst),
        "total": state.counters.total.load(SeqCst),
    });

    match serde_json::to_string(&initial) {
        Ok(json) => {
            let mut sender = ws_sender.lock().await;
            if let Err(e) = sender.send(Message::Text(json)).await {
                error!("Failed to send initial state: {}", e);
                state.metrics.concurrent_users.set(
                    (state.concurrent_users.fetch_sub(1, SeqCst) - 1)
                        .try_into()
                        .unwrap(),
                );
                return;
            }
        }
        Err(e) => {
            error!("Failed to serialize initial state: {}", e);
            state.metrics.concurrent_users.set(
                (state.concurrent_users.fetch_sub(1, SeqCst) - 1)
                    .try_into()
                    .unwrap(),
            );
            return;
        }
    }

    let handle_messages = {
        let ws_sender = Arc::clone(&ws_sender);
        let state_clone = state.clone();

        async move {
            while let Some(result) = ws_receiver.next().await {
                match result {
                    Ok(Message::Text(message)) => {
                        let color = match serde_json::from_str::<serde_json::Value>(&message) {
                            Ok(json) => {
                                if let Some(color) = json.get("color").and_then(|v| v.as_str()) {
                                    color.to_lowercase()
                                } else {
                                    message.to_lowercase()
                                }
                            }
                            Err(_) => message.to_lowercase(),
                        };
                        debug!("Received increment request for: {}", color);

                        match color.as_str() {
                            "red" => {
                                state_clone
                                    .metrics
                                    .votes
                                    .with_label_values(&["red"])
                                    .set((state_clone.counters.red).fetch_add(1, SeqCst) + 1);
                            }
                            "green" => {
                                state_clone
                                    .metrics
                                    .votes
                                    .with_label_values(&["green"])
                                    .set((state_clone.counters.green).fetch_add(1, SeqCst) + 1);
                            }
                            "blue" => {
                                state_clone
                                    .metrics
                                    .votes
                                    .with_label_values(&["blue"])
                                    .set((state_clone.counters.blue).fetch_add(1, SeqCst) + 1);
                            }
                            "purple" => {
                                state_clone
                                    .metrics
                                    .votes
                                    .with_label_values(&["purple"])
                                    .set((state_clone.counters.purple).fetch_add(1, SeqCst) + 1);
                            }
                            _ => {
                                warn!("Invalid color received: {}", color);
                                let mut sender = ws_sender.lock().await;
                                if let Err(e) = sender
                                    .send(Message::Text(format!("Invalid color: {}", color)))
                                    .await
                                {
                                    error!("Error sending validation message: {}", e);
                                }
                                continue;
                            }
                        };

                        state_clone.counters.total.fetch_add(1, SeqCst);

                        let message = json!({
                            "red": state_clone.counters.red.load(SeqCst),
                            "green": state_clone.counters.green.load(SeqCst),
                            "blue": state_clone.counters.blue.load(SeqCst),
                            "purple": state_clone.counters.purple.load(SeqCst),
                            "total": state_clone.counters.total.load(SeqCst),
                        });

                        match serde_json::to_string(&message) {
                            Ok(json) => {
                                if let Err(e) = state_clone.broadcast_tx.send(json) {
                                    warn!("Failed to broadcast update: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to serialize update: {}", e);
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        debug!("WebSocket receive error: {}", e);
                        break;
                    }
                }
            }
        }
    };

    let handle_broadcasts = async move {
        let ws_sender = Arc::clone(&ws_sender);
        while let Ok(msg) = rx.recv().await {
            let mut sender = ws_sender.lock().await;
            if let Err(e) = sender.send(Message::Text(msg)).await {
                debug!("WebSocket send error: {}", e);
                break;
            }
        }
    };

    tokio::select! {
        _ = handle_messages => {},
        _ = handle_broadcasts => {},
    }

    let new_count = state.concurrent_users.fetch_sub(1, SeqCst) - 1;
    state
        .metrics
        .concurrent_users
        .set(new_count.try_into().unwrap());
    debug!("WebSocket connection closed. User count: {}", new_count);
}
