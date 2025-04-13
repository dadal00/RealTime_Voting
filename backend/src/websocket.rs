use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::{atomic::Ordering::SeqCst, Arc};
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

    let count = state.total_users.load(SeqCst);
    let message = json!({
        "type": "users",
        "count": count,
    });
    match serde_json::to_string(&message) {
        Ok(json) => {
            if let Err(e) = state.broadcast_tx.send(json) {
                warn!("Failed to broadcast total users: {}", e);
            }
        }
        Err(e) => error!("Failed to serialize total users message: {}", e),
    }

    let initial = json!({
        "type": "initial",
        "count": count,
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
                        debug!("Received payload for: {}", message);

                        let updated_color;
                        match message.as_str() {
                            "red" => {
                                updated_color = (state_clone.counters.red).fetch_add(1, SeqCst) + 1;
                                state_clone
                                    .metrics
                                    .votes
                                    .with_label_values(&["red"])
                                    .set(updated_color.try_into().unwrap());
                            }
                            "green" => {
                                updated_color =
                                    (state_clone.counters.green).fetch_add(1, SeqCst) + 1;
                                state_clone
                                    .metrics
                                    .votes
                                    .with_label_values(&["green"])
                                    .set(updated_color.try_into().unwrap());
                            }
                            "blue" => {
                                updated_color =
                                    (state_clone.counters.blue).fetch_add(1, SeqCst) + 1;
                                state_clone
                                    .metrics
                                    .votes
                                    .with_label_values(&["blue"])
                                    .set(updated_color.try_into().unwrap());
                            }
                            "purple" => {
                                updated_color =
                                    (state_clone.counters.purple).fetch_add(1, SeqCst) + 1;
                                state_clone
                                    .metrics
                                    .votes
                                    .with_label_values(&["purple"])
                                    .set(updated_color.try_into().unwrap());
                            }
                            _ => {
                                warn!("Invalid color received: {}", message);
                                let mut sender = ws_sender.lock().await;
                                if let Err(e) = sender
                                    .send(Message::Text(format!("Invalid color: {}", message)))
                                    .await
                                {
                                    error!("Error sending validation message: {}", e);
                                }
                                continue;
                            }
                        };

                        let total = state_clone.counters.total.fetch_add(1, SeqCst) + 1;

                        let message = json!({
                            message.as_str(): updated_color,
                            "total": total,
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
                warn!("WebSocket send error: {}", e);
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
