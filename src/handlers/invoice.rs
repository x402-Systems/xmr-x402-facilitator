use crate::models::*;
use crate::state::SharedState;
use axum::{
    Json,
    extract::{Path, State},
};
use std::time::{SystemTime, UNIX_EPOCH};

fn get_network_id() -> String {
    let net = std::env::var("XMR_NETWORK").unwrap_or_else(|_| "mainnet".into());
    format!("monero:{}", net)
}

pub async fn create_invoice(
    State(state): State<SharedState>,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<Json<InvoiceResponse>, AppError> {
    // 1. Check if a pending invoice already exists for this metadata
    if let Some(metadata) = &payload.metadata {
        let existing = sqlx::query!(
            "SELECT address, amount_required FROM invoices WHERE metadata = ? AND status = 'pending' LIMIT 1",
            metadata
        )
        .fetch_optional(&state.db)
        .await?;

        if let Some(row) = existing {
            return Ok(Json(InvoiceResponse {
                // FIXED: row.address is Option<String>, convert to String
                address: row.address.unwrap_or_default(),
                amount_piconero: row.amount_required as u64,
                invoice_id: metadata.clone(),
                status: "pending".to_string(),
                network: get_network_id(),
            }));
        }
    }

    // 2. Proceed with creating a new one
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

    let invoice_id = payload
        .metadata
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let amount_i64 = amount as i64;

    sqlx::query!(
        "INSERT INTO invoices (address, amount_required, metadata, payer_id, status, created_at) VALUES (?, ?, ?, ?, 'pending', ?)",
        address,
        amount_i64,
        invoice_id,
        payload.payer_id,
        now
    )
    .execute(&state.db)
    .await?;

    Ok(Json(InvoiceResponse {
        address,
        amount_piconero: amount,
        invoice_id,
        status: "pending".to_string(),
        network: get_network_id(),
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
        // FIXED: row.address is Option<String>, convert to String
        address: row.address.unwrap_or_default(),
        amount_piconero: row.amount_required as u64,
        invoice_id: row.metadata.unwrap_or_default(),
        status: row.status.unwrap_or_else(|| "unknown".to_string()),
        network: get_network_id(),
    }))
}
