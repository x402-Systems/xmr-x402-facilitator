use crate::rpc::MoneroClient;
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct AppState {
    pub monero: MoneroClient,

    pub db: SqlitePool,
}

pub type SharedState = Arc<AppState>;
