use axum::{
    extract::{ws::WebSocketUpgrade, Path, State},
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde_json::json;
use std::{
    env,
    sync::{
        atomic::{AtomicI64, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use thiserror::Error;
use tokio::{signal, sync::broadcast};
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Error, Debug)]
enum AppError {
    #[error("Environment error: {0}")]
    Environment(#[from] std::env::VarError),

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("Invalid header value: {0}")]
    HeaderValue(#[from] axum::http::header::InvalidHeaderValue),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid color: {0}")]
    InvalidColor(String),
}

type Result<T> = std::result::Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AppError::InvalidColor(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            _ => {
                error!("Server error: {}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
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

    let rust_port = env::var("RUST_PORT")
        .inspect_err(|_| {
            info!("RUST_PORT not set, using default");
        })
        .unwrap_or_else(|_| "8080".to_string());
    let svelte_url = env::var("SVELTE_URL")
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

    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            interval.tick().await;
            let count = state_clone.user_count.load(Ordering::SeqCst);
            let message = json!({
                "type": "users",
                "count": count,
            });
            match serde_json::to_string(&message) {
                Ok(json) => {
                    if count > 0 {
                        if let Err(e) = state_clone.broadcast_tx.send(json) {
                            warn!("Failed to broadcast user count: {}", e);
                        }
                    }
                }
                Err(e) => error!("Failed to serialize user count message: {}", e),
            }
        }
    });

    let cors = CorsLayer::new()
        .allow_origin(svelte_url.parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/api/increment/:color", post(increment_handler))
        .route("/api/ws", get(websocket_handler))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", rust_port);
    info!("Binding to {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
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

async fn increment_handler(
    Path(color): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    info!(
        "New Increment Request. Color: {}",
        color.to_lowercase().as_str()
    );
    match color.to_lowercase().as_str() {
        "red" => state.counters.red.fetch_add(1, Ordering::SeqCst),
        "green" => state.counters.green.fetch_add(1, Ordering::SeqCst),
        "blue" => state.counters.blue.fetch_add(1, Ordering::SeqCst),
        "purple" => state.counters.purple.fetch_add(1, Ordering::SeqCst),
        _ => {
            warn!("{} {}", color.to_lowercase(), "Invalid color provided");
            return Err(AppError::InvalidColor(color));
        }
    };

    state.counters.total.fetch_add(1, Ordering::SeqCst);

    let message = json!({
        "red": state.counters.red.load(Ordering::SeqCst),
        "green": state.counters.green.load(Ordering::SeqCst),
        "blue": state.counters.blue.load(Ordering::SeqCst),
        "purple": state.counters.purple.load(Ordering::SeqCst),
        "total": state.counters.total.load(Ordering::SeqCst),
    });

    let json = serde_json::to_string(&message)?;

    if let Err(e) = state.broadcast_tx.send(json) {
        warn!("Failed to broadcast counter update: {}", e);
    }

    Ok((StatusCode::OK, "OK"))
}

async fn websocket_handler(
    websocket: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    websocket.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    let prev_count = state.user_count.fetch_add(1, Ordering::SeqCst);
    info!("New WebSocket connection. User count: {}", prev_count + 1);

    let mut rx = state.broadcast_tx.subscribe();

    let initial = json!({
        "red": state.counters.red.load(Ordering::SeqCst),
        "green": state.counters.green.load(Ordering::SeqCst),
        "blue": state.counters.blue.load(Ordering::SeqCst),
        "purple": state.counters.purple.load(Ordering::SeqCst),
        "total": state.counters.total.load(Ordering::SeqCst),
    });

    let mut socket = socket;
    match serde_json::to_string(&initial) {
        Ok(json) => {
            if let Err(e) = socket.send(axum::extract::ws::Message::Text(json)).await {
                error!("Failed to send initial state: {}", e);
                state.user_count.fetch_sub(1, Ordering::SeqCst);
                return;
            }

            loop {
                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Ok(msg) => {
                                if let Err(e) = socket.send(axum::extract::ws::Message::Text(msg)).await {
                                    debug!("Failed to send message: {}", e);
                                    break;
                                }
                            },
                            Err(e) => {
                                debug!("Broadcast channel error: {}", e);
                                break;
                            }
                        }
                    },
                    result = socket.recv() => {
                        match result {
                            Some(Ok(_)) => {},
                            Some(Err(e)) => {
                                debug!("WebSocket error: {}", e);
                                break;
                            },
                            None => {
                                debug!("WebSocket closed by client");
                                break;
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to serialize initial state: {}", e);
        }
    }

    let new_count = state.user_count.fetch_sub(1, Ordering::SeqCst) - 1;
    info!("WebSocket connection closed. User count: {}", new_count);
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C, shutting down");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
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
