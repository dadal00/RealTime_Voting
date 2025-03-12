use axum::{
    routing::{get, post},
    http::Method,
    Json, Router,
};

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Default)]
struct AppState {
    red: AtomicU64,
    green: AtomicU64,
    blue: AtomicU64,
    yellow: AtomicU64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    tracing_subscriber::fmt::init();
    
    let state = Arc::new(AppState::default());
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        .route("/increment/:color", post(increment))
        .route("/counters", get(get_counters))
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = std::env::var("SERVER_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()?;
    
    tracing::info!("Status - (Rust)Program running on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
        
    Ok(())
}

async fn increment(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    axum::extract::Path(color): axum::extract::Path<String>,
    ) {

    let result = match color.as_str() {
        "red" => state.red.fetch_add(1, Ordering::Relaxed),
        "green" => state.green.fetch_add(1, Ordering::Relaxed),
        "blue" => state.blue.fetch_add(1, Ordering::Relaxed),
        "yellow" => state.yellow.fetch_add(1, Ordering::Relaxed),
        unknown => {
            tracing::warn!("Error - (Rust)increment - Unknown Color Requested: {}", unknown);
            return;
        }
    };
    
    tracing::debug!("(Debug)Status - (Rust)increment - Incremented {} Counter to {}", color, result + 1);
}

async fn get_counters(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    ) -> Json<HashMap<String, u64>> {
        
    let mut counters = HashMap::new();
    counters.insert("red".to_string(), state.red.load(Ordering::Relaxed));
    counters.insert("green".to_string(), state.green.load(Ordering::Relaxed));
    counters.insert("blue".to_string(), state.blue.load(Ordering::Relaxed));
    counters.insert("yellow".to_string(), state.yellow.load(Ordering::Relaxed));
    
    tracing::debug!("(Debug)Status - (Rust)get_counters - Returning Counters: {:?}", counters);
    Json(counters)
}
