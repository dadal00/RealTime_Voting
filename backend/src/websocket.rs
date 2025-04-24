use axum::{
    extract::{
        ws::{close_code, CloseFrame, Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::{
    atomic::Ordering::{Acquire, Relaxed},
    Arc,
};
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

use crate::config::MAX_BYTES;
use crate::state::AppState;

pub async fn websocket_handler(
    websocket: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    websocket.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: Arc<AppState>) {
    state.metrics.concurrent_users.inc();
    let count = state.total_users.fetch_add(1, Relaxed);
    state.metrics.total_users.inc();
    debug!(
        "New WebSocket connection. User count: {}",
        state.concurrent_users.fetch_add(1, Relaxed) + 1
    );

    let mut rx = state.broadcast_tx.subscribe();
    let (ws_sender, mut ws_receiver) = socket.split();

    let ws_sender = Arc::new(Mutex::new(ws_sender));

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
        "red": state.counters.red.load(Acquire),
        "green": state.counters.green.load(Acquire),
        "blue": state.counters.blue.load(Acquire),
        "purple": state.counters.purple.load(Acquire),
        "total": state.counters.total.load(Acquire),
    });
    match serde_json::to_string(&initial) {
        Ok(json) => {
            let mut sender = ws_sender.lock().await;
            if let Err(e) = sender.send(Message::Text(json)).await {
                error!("Failed to send initial state: {}", e);
                state.concurrent_users.fetch_sub(1, Relaxed);
                state.metrics.concurrent_users.dec();
                return;
            }
        }
        Err(e) => {
            error!("Failed to serialize initial state: {}", e);
            state.concurrent_users.fetch_sub(1, Relaxed);
            state.metrics.concurrent_users.dec();
            return;
        }
    }

    let handle_messages = {
        let ws_sender = Arc::clone(&ws_sender);
        let state_clone = Arc::clone(&state);

        async move {
            while let Some(result) = ws_receiver.next().await {
                match result {
                    Ok(Message::Text(message)) => {
                        if message.len() > MAX_BYTES.into() {
                            error!("Payload abnormal: larger than max bytes");
                            let mut sender = ws_sender.lock().await;
                            let _ = sender
                                .send(Message::Close(Some(CloseFrame {
                                    code: close_code::INVALID,
                                    reason: "Payload too large".into(),
                                })))
                                .await;
                            return;
                        }

                        debug!("Received payload for: {}", message);

                        let updated_color;
                        match message.as_str() {
                            "red" => {
                                updated_color = state_clone.counters.red.fetch_add(1, Relaxed) + 1;
                                state_clone.metrics.votes.with_label_values(&["red"]).inc();
                            }
                            "green" => {
                                updated_color =
                                    state_clone.counters.green.fetch_add(1, Relaxed) + 1;
                                state_clone
                                    .metrics
                                    .votes
                                    .with_label_values(&["green"])
                                    .inc();
                            }
                            "blue" => {
                                updated_color = state_clone.counters.blue.fetch_add(1, Relaxed) + 1;
                                state_clone.metrics.votes.with_label_values(&["blue"]).inc();
                            }
                            "purple" => {
                                updated_color =
                                    state_clone.counters.purple.fetch_add(1, Relaxed) + 1;
                                state_clone
                                    .metrics
                                    .votes
                                    .with_label_values(&["purple"])
                                    .inc();
                            }
                            _ => {
                                error!("Invalid color received: {}", message);
                                let mut sender = ws_sender.lock().await;
                                let _ = sender
                                    .send(Message::Close(Some(CloseFrame {
                                        code: close_code::INVALID,
                                        reason: "Payload too large".into(),
                                    })))
                                    .await;
                                return;
                            }
                        };

                        let message = json!({
                            message.as_str(): updated_color,
                            "total": state_clone.counters.total.fetch_add(1, Relaxed) + 1,
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
                        error!("Websocket error: {}", e);
                        let mut sender = ws_sender.lock().await;
                        let _ = sender
                            .send(Message::Close(Some(CloseFrame {
                                code: close_code::INVALID,
                                reason: "Payload too large".into(),
                            })))
                            .await;
                        return;
                    }
                }
            }
        }
    };
    let state_clone = Arc::clone(&state);
    let handle_broadcasts = async move {
        let ws_sender = Arc::clone(&ws_sender);

        while let Ok(msg) = rx.recv().await {
            let mut sender = ws_sender.lock().await;
            if let Err(e) = sender.send(Message::Text(msg)).await {
                error!("WebSocket send error: {}", e);
                let _ = sender
                    .send(Message::Close(Some(CloseFrame {
                        code: close_code::INVALID,
                        reason: "Payload too large".into(),
                    })))
                    .await;
                return;
            }
        }
    };

    tokio::select! {
        _ = handle_messages => {},
        _ = handle_broadcasts => {},
    }

    state_clone.metrics.concurrent_users.dec();
    debug!(
        "WebSocket connection closed. User count: {}",
        state_clone.concurrent_users.fetch_sub(1, Relaxed) - 1
    );
}
