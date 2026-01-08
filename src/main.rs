mod handlers;
mod models;
mod rpc;
mod state;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // Database setup
    let db_url = std::env::var("DATABASE_URL").unwrap_or("sqlite:facilitator.db".to_string());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Create Universal Schema
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS invoices (
            address TEXT PRIMARY KEY,
            amount_required INTEGER NOT NULL,
            metadata TEXT,
            payer_id TEXT,
            status TEXT,
            tx_id TEXT,
            created_at INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await?;

    let monero_url =
        std::env::var("MONERO_RPC_URL").unwrap_or("http://127.0.0.1:18083/json_rpc".into());

    let shared_state = Arc::new(state::AppState {
        monero: rpc::MoneroClient {
            rpc_url: monero_url,
        },
        db: pool,
        price_per_access_usd: 0.10, // Default fallback
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Universal API Routes
    let app = Router::new()
        // Merchant Routes
        .route("/invoices", post(handlers::create_invoice))
        .route("/invoices/{address}", get(handlers::get_invoice_status))
        // Verification Route (Public/Client)
        .route("/supported", get(handlers::get_supported))
        .route("/verify", post(handlers::verify_payment))
        .route("/settle", post(handlers::settle_payment))
        .layer(cors)
        .with_state(shared_state);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3113".to_string());
    let addr_str = format!("{}:{}", host, port);

    println!("🚀 Universal XMR x402 Facilitator Live on {}", addr_str);

    let listener = tokio::net::TcpListener::bind(&addr_str).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
