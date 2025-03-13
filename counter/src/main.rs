use axum::{
    routing::{get, post},
    http::{Method, StatusCode},
    Json, Router,
};

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Default)]
struct AppState {
    counters: [AtomicU64; 4],
}

impl AppState {
    fn increment(&self, color: &str) -> Option<u64> {
        match color {
            "red" => Some(self.counters[0].fetch_add(1, Ordering::Relaxed)),
            "green" => Some(self.counters[1].fetch_add(1, Ordering::Relaxed)),
            "blue" => Some(self.counters[2].fetch_add(1, Ordering::Relaxed)),
            "yellow" => Some(self.counters[3].fetch_add(1, Ordering::Relaxed)),
            _ => None,
        }
    }

    fn get_all(&self) -> Counters {
        Counters {
            red: self.counters[0].load(Ordering::Relaxed),
            green: self.counters[1].load(Ordering::Relaxed),
            blue: self.counters[2].load(Ordering::Relaxed),
            yellow: self.counters[3].load(Ordering::Relaxed),
        }
    }
}

#[derive(serde::Serialize)]
struct Counters {
    red: u64,
    green: u64,
    blue: u64,
    yellow: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::new("info")) // the specific level of logging that can be changed
        .init();
    
    let state = Arc::new(AppState::default());

    let app = Router::new()
        .route("/increment/:color", post(increment))
        .route("/counters", get(get_counters))
        .with_state(state)
        .layer(CorsLayer::new() // the CORS customizations 
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(Any))
        .layer(TraceLayer::new_for_http());

    let addr = "0.0.0.0:3000".parse()?; // the specific IP 
    tracing::info!("Status - (Rust)Program running on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
        
    Ok(())
}

async fn increment(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    axum::extract::Path(color): axum::extract::Path<String>,
    ) -> Result<StatusCode, StatusCode> {

    state.increment(&color)
        .map(|previous| {
            tracing::debug!("(Debug)Status - (Rust)increment - Incremented {} Counter to {}", color, previous + 1);
            StatusCode::OK
        })
        .ok_or(StatusCode::BAD_REQUEST)
}

async fn get_counters(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    ) -> Json<Counters> {
    
    Json(state.get_all())
}
