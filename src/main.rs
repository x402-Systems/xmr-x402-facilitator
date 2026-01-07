mod handlers;
mod models;
mod rpc;
mod state;

use axum::{Router, routing::get};
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:facilitator.db".to_string());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Apply migrations/schema check
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS invoices (
            address TEXT PRIMARY KEY,
            amount_required INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await?;

    let monero_url = std::env::var("MONERO_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:18083/json_rpc".into());

    let shared_state = Arc::new(state::AppState {
        monero: rpc::MoneroClient {
            rpc_url: monero_url,
        },
        db: pool,
        price_per_access_usd: 0.10,
    });

    let app = Router::new()
        .route("/content", get(handlers::get_protected_resource))
        .with_state(shared_state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3113));
    println!("🚀 Monero x402 Facilitator Live on {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
