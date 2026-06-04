use crate::rpc::MoneroClient;
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct AppState {
    pub monero: MoneroClient,
    pub db: SqlitePool,
    pub min_confirmations: u64,
    pub verify_max_attempts: usize,
    pub verify_poll_interval: u64,
}

pub type SharedState = Arc<AppState>;
