use crate::{error::AppError, AppState};
use axum::extract::State;
use prometheus::{
    register_int_counter, register_int_counter_vec, register_int_gauge, Encoder, IntCounter,
    IntCounterVec, IntGauge, Registry, TextEncoder,
};
use std::sync::Arc;
use tracing::debug;

#[derive(Debug)]
pub struct Metrics {
    pub concurrent_users: IntGauge,
    pub total_users: IntCounter,
    pub votes: IntCounterVec,
    registry: Registry,
}

impl Default for Metrics {
    fn default() -> Self {
        let registry = prometheus::Registry::new();

        let concurrent_users =
            register_int_gauge!("concurrent_users", "Number of currently connected users")
                .expect("Can't create concurrent_users metric");

        let total_users =
            register_int_counter!("total_users", "Total number of users since startup")
                .expect("Can't create total_users metric");

        let votes = register_int_counter_vec!("votes", "Current vote counts", &["color"])
            .expect("Can't create votes metric");

        registry
            .register(Box::new(concurrent_users.clone()))
            .unwrap();
        registry.register(Box::new(total_users.clone())).unwrap();
        registry.register(Box::new(votes.clone())).unwrap();

        Metrics {
            concurrent_users,
            total_users,
            votes,
            registry,
        }
    }
}

impl Metrics {
    pub fn gather(&self) -> Result<String, AppError> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

pub async fn metrics_handler(State(state): State<Arc<AppState>>) -> Result<String, AppError> {
    debug!("Metrics being scrapped");
    state.metrics.gather()
}
