use axum::{
    body::{to_bytes, Body},
    extract::{ws::Message, ws::WebSocket, ws::WebSocketUpgrade, Path, State},
    http::{
        header::InvalidHeaderValue, header::CONTENT_TYPE, HeaderName, Method, Request, StatusCode,
    },
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use opentelemetry::{
    global,
    runtime::Tokio,
    sdk::{propagation::TraceContextPropagator, trace::config, Resource},
    trace::TraceError,
    KeyValue,
};
use opentelemetry_otlp::{new_exporter, new_pipeline, WithExportConfig};
use reqwest::Client;
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
use tokio::{net::TcpListener, signal, sync::broadcast, time::interval};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use tracing::{
    debug, dispatcher::SetGlobalDefaultError, error, error_span, field::display, info, info_span,
    subscriber::set_global_default, warn,
};
use tracing_subscriber::{filter::ParseError, fmt, layer::SubscriberExt, registry, EnvFilter};

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

    #[error("Invalid color: {0}")]
    InvalidColor(String),

    #[error("OpenTelemetry trace error: {0}")]
    OpenTelemetryTrace(#[from] TraceError),

    #[error("Tracing filter parse error: {0}")]
    TracingFilterParse(#[from] ParseError),

    #[error("Tracing subscriber error: {0}")]
    TracingSubscriber(#[from] SetGlobalDefaultError),
}

type Result<T> = stdResult<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let span = error_span!("app_error");
        span.record("error", display(&self));
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

async fn init_tracing() -> Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let otel_endpoint = var("OTEL_GRPC_ENDPOINT")
        .inspect_err(|_| {
            info!("OTEL_GRPC_ENDPOINT not set, using default");
        })
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let otel_tracer = new_pipeline()
        .tracing()
        .with_trace_config(config().with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            "counter_backend",
        )])))
        .with_exporter(new_exporter().tonic().with_endpoint(&otel_endpoint))
        .install_batch(Tokio)?;

    let otel_layer = tracing_opentelemetry::layer().with_tracer(otel_tracer);

    let fmt_layer = fmt::layer();
    let filter = EnvFilter::from_default_env();

    let subscriber = registry().with(filter).with(fmt_layer).with(otel_layer);

    set_global_default(subscriber)?;

    info!(otel_grpc = %otel_endpoint, level = %EnvFilter::from_default_env() ,"Log configuration");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing().await?;

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

    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_millis(500));
        loop {
            interval.tick().await;
            let count = state_clone.user_count.load(SeqCst);
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
        .allow_origin(AllowOrigin::predicate(move |origin, _req| {
            origin.as_bytes() == svelte_url.as_bytes()
        }))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE, HeaderName::from_static("traceparent")])
        .allow_credentials(true)
        .max_age(Duration::from_secs(60 * 60));

    let app = Router::new()
        .route("/api/increment/:color", post(increment_handler))
        .route("/api/ws", get(websocket_handler))
        .route("/api/otel/*path", post(otel_handler))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                info_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                )
            }),
        )
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

async fn otel_handler(Path(path): Path<String>, request: Request<Body>) -> StatusCode {
    let _ = async {
        let otel_collector =
            var("OTEL_HTTP_ENDPOINT").unwrap_or_else(|_| "http://otel-collector:4318".to_string());

        let target_url = format!("{}/{}", otel_collector, path);

        let body = to_bytes(request.into_body(), usize::MAX).await?;

        Client::new()
            .post(&target_url)
            .header("Content-Type", "application/x-protobuf")
            .body(body)
            .send()
            .await?;

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await
    .map_err(|e: Box<dyn std::error::Error>| {
        error!("Failed to forward telemetry: {}", e);
    });

    StatusCode::OK
}

async fn increment_handler(
    Path(color): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let span = info_span!(
        "increment_counter",
        color = %color,
        user_count = state.user_count.load(SeqCst)
    );
    let _guard = span.enter();
    info!(
        "New Increment Request. Color: {}",
        color.to_lowercase().as_str()
    );
    match color.to_lowercase().as_str() {
        "red" => state.counters.red.fetch_add(1, SeqCst),
        "green" => state.counters.green.fetch_add(1, SeqCst),
        "blue" => state.counters.blue.fetch_add(1, SeqCst),
        "purple" => state.counters.purple.fetch_add(1, SeqCst),
        _ => {
            warn!("{} {}", color.to_lowercase(), "Invalid color provided");
            return Err(AppError::InvalidColor(color));
        }
    };

    state.counters.total.fetch_add(1, SeqCst);

    let message = json!({
        "red": state.counters.red.load(SeqCst),
        "green": state.counters.green.load(SeqCst),
        "blue": state.counters.blue.load(SeqCst),
        "purple": state.counters.purple.load(SeqCst),
        "total": state.counters.total.load(SeqCst),
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

async fn handle_websocket(socket: WebSocket, state: Arc<AppState>) {
    let span = info_span!("websocket_session");
    let _guard = span.enter();

    let prev_count = state.user_count.fetch_add(1, SeqCst);
    info!("New WebSocket connection. User count: {}", prev_count + 1);

    let mut rx = state.broadcast_tx.subscribe();

    let initial = json!({
        "red": state.counters.red.load(SeqCst),
        "green": state.counters.green.load(SeqCst),
        "blue": state.counters.blue.load(SeqCst),
        "purple": state.counters.purple.load(SeqCst),
        "total": state.counters.total.load(SeqCst),
    });

    let mut socket = socket;
    match serde_json::to_string(&initial) {
        Ok(json) => {
            if let Err(e) = socket.send(Message::Text(json)).await {
                error!("Failed to send initial state: {}", e);
                state.user_count.fetch_sub(1, SeqCst);
                return;
            }

            loop {
                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Ok(msg) => {
                                if let Err(e) = socket.send(Message::Text(msg)).await {
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
