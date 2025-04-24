use crate::error::AppError;
use tracing::{info, warn};

pub const MAX_BYTES: u8 = 10;

#[derive(Debug, Clone)]
pub struct Config {
    pub rust_port: u16,
    pub svelte_url: String,
    pub state_path: String,
}

impl Config {
    pub fn load() -> Result<Self, AppError> {
        let rust_port = var("RUST_PORT")
            .inspect_err(|_| {
                info!("RUST_PORT not set, using default");
            })
            .unwrap_or_else(|_| "8080".into())
            .parse()
            .map_err(|_| AppError::Config("Invalid RUST_PORT value".into()))?;

        let svelte_url = var("SVELTE_URL")
            .inspect_err(|_| {
                info!("SVELTE_URL not set, using default");
            })
            .unwrap_or_else(|_| "http://localhost:5173".into());

        let state_path = var("RUST_STATE_PATH")
            .inspect_err(|_| {
                info!("RUST_STATE_PATH not set, using default");
            })
            .unwrap_or_else(|_| "/saved_state.json".into());

        Ok(Self {
            rust_port,
            svelte_url,
            state_path,
        })
    }
}

fn var(key: &str) -> Result<String, AppError> {
    std::env::var(key).map_err(|e| {
        warn!("Environment variable {} not found, using default", key);
        AppError::Environment(e)
    })
}
