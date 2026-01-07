use crate::rpc::MoneroClient;
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct AppState {
    /// The client that talks to monero-wallet-rpc
    pub monero: MoneroClient,

    pub db: SqlitePool,
    pub price_per_access_usd: f64, // dynamic pricing
}

pub type SharedState = Arc<AppState>;
