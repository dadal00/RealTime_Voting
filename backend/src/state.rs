use crate::metrics::Metrics;
use std::sync::atomic::{AtomicI64, AtomicUsize};
use tokio::sync::broadcast;

pub struct AppState {
    pub counters: Counters,
    pub concurrent_users: AtomicUsize,
    pub total_users: AtomicUsize,
    pub broadcast_tx: broadcast::Sender<String>,
    pub metrics: Metrics,
}

pub struct Counters {
    pub red: AtomicI64,
    pub green: AtomicI64,
    pub blue: AtomicI64,
    pub purple: AtomicI64,
    pub total: AtomicI64,
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
