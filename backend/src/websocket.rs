use axum::{
    extract::{
        ws::{close_code, CloseFrame, Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde_json::json;
use std::sync::{
    atomic::Ordering::{Acquire, Relaxed},
    Arc,
};
use tokio::sync::{broadcast::Receiver, Mutex};
use tracing::{debug, error, warn};

use crate::config::MAX_BYTES;
use crate::error::AppError;
use crate::state::AppState;

enum ClosingSignal {
    WebSocketErr,
    PayloadTooLarge,
    InvalidColor,
    WebSocketSendErr,
}

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

    let rx = state.broadcast_tx.subscribe();

    let (ws_sender, ws_receiver) = socket.split();
    let ws_sender_arc = Arc::new(Mutex::new(ws_sender));

    let handle_messages_sender = Arc::clone(&ws_sender_arc);
    let handle_messages_state = Arc::clone(&state);

    let handle_broadcasts_sender = Arc::clone(&ws_sender_arc);
    let metrics_state = Arc::clone(&state);

    match send_initial(&count, &state, &ws_sender_arc).await {
        Ok(()) => {}
        Err(e) => {
            error!("Sending initial state failed: {}", e);
            state.concurrent_users.fetch_sub(1, Relaxed);
            state.metrics.concurrent_users.dec();
            return;
        }
    }

    tokio::select! {
        _ = handle_messages(ws_receiver, handle_messages_sender, handle_messages_state) => {},
        _ = handle_broadcasts(rx, handle_broadcasts_sender) => {},
    }

    metrics_state.metrics.concurrent_users.dec();
    debug!(
        "WebSocket connection closed. User count: {}",
        metrics_state.concurrent_users.fetch_sub(1, Relaxed) - 1
    );
}

async fn handle_messages(
    mut ws_receiver: SplitStream<WebSocket>,
    ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    state: Arc<AppState>,
) {
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(Message::Text(message)) => {
                if message.len() > MAX_BYTES.into() {
                    close_connection(ClosingSignal::PayloadTooLarge, &ws_sender, None).await;
                    return;
                }

                debug!("Received payload for: {}", message);

                process_message(&message, &state, &ws_sender).await;
            }
            Ok(_) => {}
            Err(e) => {
                close_connection(
                    ClosingSignal::WebSocketErr,
                    &ws_sender,
                    Some(&e.to_string()),
                )
                .await;
                return;
            }
        }
    }
}

async fn handle_broadcasts(
    mut rx: Receiver<String>,
    ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
) {
    while let Ok(msg) = rx.recv().await {
        let mut sender = ws_sender.lock().await;
        if let Err(e) = sender.send(Message::Text(msg)).await {
            close_connection(
                ClosingSignal::WebSocketSendErr,
                &ws_sender,
                Some(&e.to_string()),
            )
            .await;
            return;
        }
    }
}

async fn process_message(
    message: &str,
    state: &Arc<AppState>,
    ws_sender: &Arc<Mutex<SplitSink<WebSocket, Message>>>,
) {
    let state_clone = Arc::clone(state);

    let updated_color = match message {
        "red" => {
            state_clone.metrics.votes.with_label_values(&["red"]).inc();
            state_clone.counters.red.fetch_add(1, Relaxed) + 1
        }
        "green" => {
            state_clone
                .metrics
                .votes
                .with_label_values(&["green"])
                .inc();
            state_clone.counters.green.fetch_add(1, Relaxed) + 1
        }
        "blue" => {
            state_clone.metrics.votes.with_label_values(&["blue"]).inc();
            state_clone.counters.blue.fetch_add(1, Relaxed) + 1
        }
        "purple" => {
            state_clone
                .metrics
                .votes
                .with_label_values(&["purple"])
                .inc();
            state_clone.counters.purple.fetch_add(1, Relaxed) + 1
        }
        _ => {
            close_connection(ClosingSignal::InvalidColor, ws_sender, Some(message)).await;
            return;
        }
    };

    broadcast_update(message, updated_color, state_clone).await;
}

async fn broadcast_update(message: &str, updated_color: usize, state: Arc<AppState>) {
    let update = json!({
        message: updated_color,
        "total": state.counters.total.fetch_add(1, Relaxed) + 1,
    });

    match serde_json::to_string(&update) {
        Ok(json) => {
            if let Err(e) = state.broadcast_tx.send(json) {
                warn!("Failed to broadcast update: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to serialize update: {}", e);
        }
    }
}

async fn close_connection(
    signal: ClosingSignal,
    ws_sender: &Arc<Mutex<SplitSink<WebSocket, Message>>>,
    error_info: Option<&str>,
) {
    let message = match signal {
        ClosingSignal::WebSocketErr => {
            error!(
                "Websocket error: {}",
                error_info.unwrap_or("unknown websocket error")
            );
            "Websocket Error"
        }
        ClosingSignal::PayloadTooLarge => {
            error!("Payload abnormal: larger than max bytes");
            "Abnormal Payload"
        }
        ClosingSignal::InvalidColor => {
            error!(
                "Invalid color received: {}",
                error_info.unwrap_or("unknown color")
            );
            "Invalid Color"
        }
        ClosingSignal::WebSocketSendErr => {
            error!(
                "Websocket sending error: {}",
                error_info.unwrap_or("unknown color")
            );
            "Websocket Sending Error"
        }
    };
    let mut sender = ws_sender.lock().await;
    let _ = sender
        .send(Message::Close(Some(CloseFrame {
            code: close_code::INVALID,
            reason: message.into(),
        })))
        .await;
}

async fn send_initial(
    count: &usize,
    state: &Arc<AppState>,
    ws_sender: &Arc<Mutex<SplitSink<WebSocket, Message>>>,
) -> Result<(), AppError> {
    let message = json!({
        "type": "users",
        "count": count,
    });
    let json = serde_json::to_string(&message)?;
    state.broadcast_tx.send(json)?;

    let initial = json!({
        "type": "initial",
        "count": count,
        "red": state.counters.red.load(Acquire),
        "green": state.counters.green.load(Acquire),
        "blue": state.counters.blue.load(Acquire),
        "purple": state.counters.purple.load(Acquire),
        "total": state.counters.total.load(Acquire),
    });
    let json = serde_json::to_string(&initial)?;

    let mut sender = ws_sender.lock().await;
    sender.send(Message::Text(json)).await?;
    Ok(())
}
