use axum::{
    extract::{Path, State, ws::WebSocketUpgrade},
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde_json::json;
use std::{
    env,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    signal,
    sync::{broadcast, Mutex},
};
use tower_http::cors::CorsLayer;

struct AppState {
    counters: Mutex<Counters>,
    user_count: AtomicUsize,
    broadcast_tx: broadcast::Sender<String>,
}

#[derive(Default)]
struct Counters {
    red: i64,
    green: i64,
    blue: i64,
    purple: i64,
    total: i64,
}

#[tokio::main]
async fn main() {
    let rust_port = env::var("RUST_PORT").unwrap_or_else(|_| "8080".to_string());
    let svelte_url = env::var("SVELTE_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

    let (broadcast_tx, _) = broadcast::channel(100);
    let state = Arc::new(AppState {
        counters: Mutex::new(Counters::default()),
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
            if let Ok(json) = serde_json::to_string(&message) {
                let _ = state_clone.broadcast_tx.send(json);
            }
        }
    });

    let cors = CorsLayer::new()
        .allow_origin(svelte_url.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/api/increment/:color", post(increment_handler))
        .route("/api/ws", get(websocket_handler))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", rust_port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn increment_handler(
    Path(color): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut counters = state.counters.lock().await;
    
    match color.to_lowercase().as_str() {
        "red" => counters.red += 1,
        "green" => counters.green += 1,
        "blue" => counters.blue += 1,
        "purple" => counters.purple += 1,
        _ => return (StatusCode::BAD_REQUEST, "Invalid color"),
    };
    counters.total += 1;

    let message = json!({
        "red": counters.red,
        "green": counters.green,
        "blue": counters.blue,
        "purple": counters.purple,
        "total": counters.total,
    });

    if let Ok(json) = serde_json::to_string(&message) {
        let _ = state.broadcast_tx.send(json);
    }

    (StatusCode::OK, "OK")
}

async fn websocket_handler(
    websocket: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    websocket.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    state.user_count.fetch_add(1, Ordering::SeqCst);
    let mut rx = state.broadcast_tx.subscribe();

    let counters = state.counters.lock().await;
    let initial = json!({
        "red": counters.red,
        "green": counters.green,
        "blue": counters.blue,
        "purple": counters.purple,
        "total": counters.total,
    });
    drop(counters);

    let mut socket = socket;
    if socket.send(axum::extract::ws::Message::Text(initial.to_string())).await.is_ok() {
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    if let Ok(msg) = msg {
                        if socket.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                },
                result = socket.recv() => {
                    match result {
                        Some(Ok(_)) => {},
                        _ => break,
                    }
                }
            }
        }
    }

    state.user_count.fetch_sub(1, Ordering::SeqCst);
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
