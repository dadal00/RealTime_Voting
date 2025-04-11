use crate::{
    config::Config,
    metrics::{metrics_handler, Metrics},
    state::Counters,
    websocket::websocket_handler,
};
use axum::{
    http::{header::CONTENT_TYPE, HeaderName, Method},
    routing::get,
    Router,
};
use error::AppError;
use serde_json::json;
use signals::shutdown_signal;
use state::AppState;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc,
    },
    time::Duration,
};
use tokio::{net::TcpListener, sync::broadcast, time::interval};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

mod config;
mod error;
mod metrics;
mod signals;
mod state;
mod websocket;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    fmt()
        .with_env_filter(
            EnvFilter::from_default_env(), // backend (target) = info (logging level)
        )
        .init();

    info!("Starting server...");

    let config = Config::load()?;

    info!(port = %config.rust_port, frontend = %config.svelte_url, "Server configuration");

    let svelte_url = config.svelte_url.clone();

    let (broadcast_tx, _) = broadcast::channel(100);
    let state = Arc::new(AppState {
        metrics: Metrics::default(),
        counters: Counters::default(),
        concurrent_users: AtomicUsize::new(0),
        total_users: AtomicUsize::new(0),
        broadcast_tx,
    });

    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_millis(500));
        loop {
            interval.tick().await;
            let count = state_clone.total_users.load(SeqCst);
            let current_users = state_clone.concurrent_users.load(SeqCst);
            let message = json!({
                "type": "users",
                "count": count,
            });
            match serde_json::to_string(&message) {
                Ok(json) => {
                    if current_users > 0 {
                        if let Err(e) = state_clone.broadcast_tx.send(json) {
                            warn!("Failed to broadcast total users: {}", e);
                        }
                    }
                }
                Err(e) => error!("Failed to serialize total users message: {}", e),
            }
        }
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
        .route("/metrics", get(metrics_handler))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.rust_port);
    info!("Binding to {}", addr);

    let listener = TcpListener::bind(&addr).await?;
    info!("Server running on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}
