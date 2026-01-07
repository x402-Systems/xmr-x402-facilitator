use crate::models::*;
use crate::state::SharedState;
use axum::{
    Json,
    extract::{Path, State},
};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn create_invoice(
    State(state): State<SharedState>,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<Json<InvoiceResponse>, AppError> {
    let amount = state
        .monero
        .get_xmr_price_piconero(payload.amount_usd)
        .await
        .map_err(AppError::Rpc)?;

    let address = state
        .monero
        .create_subaddress()
        .await
        .map_err(AppError::Rpc)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let amount_i64 = amount as i64;

    sqlx::query!(
        "INSERT INTO invoices (address, amount_required, metadata, status, created_at) VALUES (?, ?, ?, 'pending', ?)",
        address,
        amount_i64,
        payload.metadata,
        now
    )
    .execute(&state.db)
    .await?;

    Ok(Json(InvoiceResponse {
        address,
        amount_piconero: amount,
        invoice_id: uuid::Uuid::new_v4().to_string(),
        status: "pending".to_string(),
        network: std::env::var("XMR_NETWORK").unwrap_or("mainnet".into()),
    }))
}

pub async fn get_invoice_status(
    State(state): State<SharedState>,
    Path(address): Path<String>,
) -> Result<Json<InvoiceResponse>, AppError> {
    let row = sqlx::query!(
        "SELECT address, amount_required, status, metadata FROM invoices WHERE address = ?",
        address
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(InvoiceResponse {
        address: row.address.unwrap_or_default(),
        amount_piconero: row.amount_required as u64,
        invoice_id: row.metadata.unwrap_or_default(),
        status: row.status.unwrap_or_else(|| "unknown".to_string()),
        network: std::env::var("XMR_NETWORK").unwrap_or("mainnet".into()),
    }))
}
