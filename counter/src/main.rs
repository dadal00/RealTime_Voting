use axum::{
    extract::Path,
    routing::{get, post},
    http::{Method, StatusCode},
    Json, Router,
};
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use serde::{Deserialize, Serialize};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

const KAFKA_BROKERS: &str = "localhost:9092";
const COUNTERS_TOPIC: &str = "counters-updates";

static RED_COUNTER: AtomicU64 = AtomicU64::new(0);
static GREEN_COUNTER: AtomicU64 = AtomicU64::new(0);
static BLUE_COUNTER: AtomicU64 = AtomicU64::new(0);
static PURPLE_COUNTER: AtomicU64 = AtomicU64::new(0);

static PRODUCER: std::sync::OnceLock<FutureProducer> = std::sync::OnceLock::new();

#[derive(Serialize, Deserialize)]
struct Counters {
    red: u64,
    green: u64,
    blue: u64,
    purple: u64,
}

fn get_all_counters() -> Counters {
    Counters {
        red: RED_COUNTER.load(Ordering::Relaxed),
        green: GREEN_COUNTER.load(Ordering::Relaxed),
        blue: BLUE_COUNTER.load(Ordering::Relaxed),
        purple: PURPLE_COUNTER.load(Ordering::Relaxed),
    }
}

async fn increment_counter(color: &str) -> Result<u64, ()> {
    let counter = match color {
        "red" => &RED_COUNTER,
        "green" => &GREEN_COUNTER,
        "blue" => &BLUE_COUNTER,
        "purple" => &PURPLE_COUNTER,
        _ => return Err(()),
    };

    let previous = counter.fetch_add(1, Ordering::Relaxed);

    if let Some(producer) = PRODUCER.get() {
        let counters = get_all_counters();
        let payload = serde_json::to_string(&counters).unwrap_or_default();
        
        let record = FutureRecord::to(COUNTERS_TOPIC)
            .key(color)
            .payload(&payload);

        match producer.send(record, Duration::from_secs(2)).await {
            Ok(_) => tracing::debug!("Published update for {}", color),
            Err((e, _)) => tracing::error!("Kafka error: {}", e),
        }
    }

    Ok(previous)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::new("info")) // the specific level of logging that can be changed
        .init();
    
    PRODUCER.get_or_init(|| {
        let mut config = ClientConfig::new();
        config
            .set("bootstrap.servers", KAFKA_BROKERS)
            .set("message.timeout.ms", "500")
            .set("linger.ms", "0") 
            .set("socket.timeout.ms", "1000")
            .set("reconnect.backoff.ms", "100")
            .set("reconnect.backoff.max.ms", "1000")
            .set("queue.buffering.max.messages", "1");
        
        config.create().expect("Failed to create Kafka producer")
    });

    let app = Router::new()
        .route("/increment/:color", post(handle_increment))
        .route("/counters", get(handle_get_counters))
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

async fn handle_increment(
        Path(color): Path<String>,
    ) -> Result<StatusCode, StatusCode> {
    increment_counter(&color).await
        .map(|previous| {
            tracing::debug!("Incremented {} to {}", color, previous + 1);
            StatusCode::OK
        })
        .map_err(|_| StatusCode::BAD_REQUEST)
}

async fn handle_get_counters() -> Json<Counters> {
    Json(get_all_counters())
}
