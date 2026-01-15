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

    let reaper_pool = pool.clone();
    tokio::spawn(async move {
        println!("Invoice Reaper initialized (cleaning pending invoices > 24)");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));

        loop {
            interval.tick().await;

            let cutoff = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 3600;

            let cutoff_i64 = cutoff as i64;

            let result = sqlx::query!(
                "DELETE FROM invoices WHERE status = 'pending' AND created_at < ?",
                cutoff_i64
            )
            .execute(&reaper_pool)
            .await;

            match result {
                Ok(res) => {
                    let affected = res.rows_affected();
                    if affected > 0 {
                        println!("Reaper: Purged {} exired pending invoices.", affected);
                    }
                }
                Err(e) => eprintln!("Reaper Error: {}", e),
            }
        }
    });

    let monero_url =
        std::env::var("MONERO_RPC_URL").unwrap_or("http://127.0.0.1:18083/json_rpc".into());

    let shared_state = Arc::new(state::AppState {
        monero: rpc::MoneroClient {
            rpc_url: monero_url,
        },
        db: pool,
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
