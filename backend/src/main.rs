use axum::{
    extract::{ws::Message, ws::WebSocket, ws::WebSocketUpgrade, State},
    http::{header::InvalidHeaderValue, header::CONTENT_TYPE, HeaderName, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Error as jsonError};
use signal::{
    ctrl_c,
    unix::{signal, SignalKind},
};
use std::{
    env::{var, VarError},
    io::Error as IOError,
    result::Result as stdResult,
    sync::{
        atomic::{AtomicI64, AtomicUsize, Ordering::SeqCst},
        Arc,
    },
    time::Duration,
};
use thiserror::Error;
use tokio::{net::TcpListener, signal, sync::broadcast, sync::Mutex};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::{debug, dispatcher::SetGlobalDefaultError, error, info, warn};
use tracing_subscriber::{filter::ParseError, fmt, EnvFilter};

#[derive(Error, Debug)]
enum AppError {
    #[error("Environment error: {0}")]
    Environment(#[from] VarError),

    #[error("Network error: {0}")]
    Network(#[from] IOError),

    #[error("Invalid header value: {0}")]
    HeaderValue(#[from] InvalidHeaderValue),

    #[error("JSON serialization error: {0}")]
    Json(#[from] jsonError),

    #[error("Tracing filter parse error: {0}")]
    TracingFilterParse(#[from] ParseError),

    #[error("Tracing subscriber error: {0}")]
    TracingSubscriber(#[from] SetGlobalDefaultError),
}

type Result<T> = stdResult<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = {
            error!("Server error: {}", self);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            )
        };

        (status, message).into_response()
    }
}

struct AppState {
    counters: Counters,
    user_count: AtomicUsize,
    broadcast_tx: broadcast::Sender<String>,
}

struct Counters {
    red: AtomicI64,
    green: AtomicI64,
    blue: AtomicI64,
    purple: AtomicI64,
    total: AtomicI64,
}

impl Default for Counters {
    fn default() -> Self {
        Self {
            red: AtomicI64::new(0),
            green: AtomicI64::new(0),
            blue: AtomicI64::new(0),
            purple: AtomicI64::new(0),
            total: AtomicI64::new(0),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("backend=info".parse().unwrap()), // backend (target) = info (logging level)
        )
        .init();

    info!("Starting server...");

    let rust_port = var("RUST_PORT")
        .inspect_err(|_| {
            info!("RUST_PORT not set, using default");
        })
        .unwrap_or_else(|_| "8080".to_string());
    let svelte_url = var("SVELTE_URL")
        .inspect_err(|_| {
            info!("SVELTE_URL not set, using default");
        })
        .unwrap_or_else(|_| "http://localhost:5173".to_string());

    info!(port = %rust_port, frontend = %svelte_url, "Server configuration");

    let (broadcast_tx, _) = broadcast::channel(100);
    let state = Arc::new(AppState {
        counters: Counters::default(),
        user_count: AtomicUsize::new(0),
        broadcast_tx,
    });

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin, _req| {
            origin.as_bytes() == svelte_url.as_bytes()
        }))
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE, HeaderName::from_static("traceparent")])
        .allow_credentials(true)
        .max_age(Duration::from_secs(60 * 60));

    let app = Router::new()
        .route("/api/ws", get(websocket_handler))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", rust_port);
    info!("Binding to {}", addr);

    let listener = TcpListener::bind(&addr).await?;
    info!("Server running on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| {
            error!("Server error: {}", e);
            AppError::Network(e)
        })?;

    info!("Server shutdown complete");
    Ok(())
}

async fn websocket_handler(
    websocket: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    websocket.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: Arc<AppState>) {
    let prev_count = state.user_count.fetch_add(1, SeqCst);
    info!("New WebSocket connection. User count: {}", prev_count + 1);

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
                state.user_count.fetch_sub(1, SeqCst);
                return;
            }
        }
        Err(e) => {
            error!("Failed to serialize initial state: {}", e);
            state.user_count.fetch_sub(1, SeqCst);
            return;
        }
    }

    let handle_messages = {
        let ws_sender = Arc::clone(&ws_sender);
        let state_clone = state.clone();

        async move {
            while let Some(result) = ws_receiver.next().await {
                match result {
                    Ok(Message::Text(color)) => {
                        let color = color.to_lowercase();
                        info!("Received increment request for: {}", color);

                        let counter = match color.as_str() {
                            "red" => &state_clone.counters.red,
                            "green" => &state_clone.counters.green,
                            "blue" => &state_clone.counters.blue,
                            "purple" => &state_clone.counters.purple,
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

                        counter.fetch_add(1, SeqCst);
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

    let new_count = state.user_count.fetch_sub(1, SeqCst) - 1;
    info!("WebSocket connection closed. User count: {}", new_count);
}

async fn shutdown_signal() {
    let ctrl_c = async {
        ctrl_c().await.expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C, shutting down");
    };

    #[cfg(unix)]
    let terminate = async {
        signal(SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
        info!("Received terminate signal, shutting down");
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
