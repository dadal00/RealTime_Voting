use crate::{error::AppError, state::AppState};
use axum::extract::State;
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    sync::{atomic::Ordering::SeqCst, Arc},
};
use std::{io::Write, path::Path};
use tempfile::NamedTempFile;
use tracing::{error, info, warn};

#[derive(Deserialize)]
struct SavedState {
    total_users: usize,
    red: usize,
    green: usize,
    blue: usize,
    purple: usize,
    total: usize,
}

pub fn load(file_path: &str, State(state): State<Arc<AppState>>) {
    if Path::new(file_path).exists() {
        match fs::read_to_string(file_path) {
            Ok(data) => match serde_json::from_str::<SavedState>(&data) {
                Ok(data_read) => {
                    state.counters.red.store(data_read.red, SeqCst);
                    state.counters.green.store(data_read.green, SeqCst);
                    state.counters.blue.store(data_read.blue, SeqCst);
                    state.counters.purple.store(data_read.purple, SeqCst);
                    state.counters.total.store(data_read.total, SeqCst);
                    state.total_users.store(data_read.total_users, SeqCst);
                    info!("Loaded state");
                }
                Err(e) => {
                    error!("Loading Error parsing file: {}", e);
                }
            },
            Err(e) => {
                warn!("Loading Error reading file: {}", e);
            }
        }
    } else {
        warn!("Loading state file not found");
    }
}

pub async fn save(file_path: &str, State(state): State<Arc<AppState>>) -> Result<(), AppError> {
    
    let saved_state = json!({
        "total_users": state.total_users.load(SeqCst),
        "red": state.counters.red.load(SeqCst),
        "green": state.counters.green.load(SeqCst),
        "blue": state.counters.blue.load(SeqCst),
        "purple": state.counters.purple.load(SeqCst),
        "total": state.counters.total.load(SeqCst),
    });

    let json_data = serde_json::to_string_pretty(&saved_state)?;

    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(json_data.as_bytes())?;
    temp_file.flush()?;

    let temp_path = temp_file.into_temp_path();
    fs::copy(&temp_path, file_path)?;

    temp_path.close()?;

    info!("State saved successfully");

    Ok(())
}
