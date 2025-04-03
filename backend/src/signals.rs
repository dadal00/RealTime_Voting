use signal::{
    ctrl_c,
    unix::{signal, SignalKind},
};
use tokio::signal;
use tracing::info;

pub async fn shutdown_signal() {
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
