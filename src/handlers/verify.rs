use crate::models::*;
use crate::state::SharedState;
use axum::{Json, extract::State};
use std::time::Duration;
use tokio::time::sleep;

/// Helper to get the network string from ENV (e.g., "monero:stagenet")
fn get_network_id() -> String {
    let net = std::env::var("XMR_NETWORK").unwrap_or_else(|_| "mainnet".into());
    // Normalize to x402/CAIP style: monero:name
    format!("monero:{}", net)
}

/// GET /supported
pub async fn get_supported() -> Json<SupportedResponse> {
    Json(SupportedResponse {
        kinds: vec![SupportedKind {
            x402_version: 2,
            scheme: "exact".to_string(),
            network: get_network_id(), // Dynamic network name
        }],
    })
}

/// POST /verify
pub async fn verify_payment(
    State(state): State<SharedState>,
    Json(req): Json<X402Request>,
) -> Result<Json<VerifyResponse>, AppError> {
    //println!("📥 RECEIVED VERIFY REQUEST");

    // Access the nested payload
    let inner = req.payment_payload.payload;

    let invoice = sqlx::query!(
        "SELECT amount_required FROM invoices WHERE address = ?",
        inner.address
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let required = invoice.amount_required as u64;
    let mut received = 0;

    for i in 0..10 {
        // Increased retries
        let (rec, _) = state
            .monero
            .verify_payment_proof(
                inner.tx_id.clone(),
                inner.tx_key.clone(),
                inner.address.clone(),
            )
            .await
            .map_err(AppError::Rpc)?;

        received = rec;
        if received >= required {
            break;
        }
        println!(
            "Attempt {}: Received {}/{} - Waiting for mempool...",
            i + 1,
            received,
            required
        );
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    let is_valid = received >= required;
    Ok(Json(VerifyResponse {
        is_valid,
        invalid_reason: if is_valid {
            None
        } else {
            Some("Insufficient funds or tx not found".into())
        },
    }))
}

pub async fn settle_payment(
    State(state): State<SharedState>,
    Json(req): Json<X402Request>,
) -> Result<Json<SettleResponse>, AppError> {
    //println!("📥 RECEIVED SETTLE REQUEST");
    let inner = req.payment_payload.payload;

    let invoice = sqlx::query!(
        "SELECT amount_required FROM invoices WHERE address = ?",
        inner.address
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let (received, confirmations) = state
        .monero
        .verify_payment_proof(
            inner.tx_id.clone(),
            inner.tx_key.clone(),
            inner.address.clone(),
        )
        .await
        .map_err(AppError::Rpc)?;

    if received >= (invoice.amount_required as u64) {
        sqlx::query!(
            "UPDATE invoices SET status = 'paid', tx_id = ? WHERE address = ?",
            inner.tx_id,
            inner.address
        )
        .execute(&state.db)
        .await?;

        //println!("🎉 SETTLEMENT SUCCESS: Tx {}", inner.tx_id);
        Ok(Json(SettleResponse {
            success: true,
            transaction: inner.tx_id,
            network: get_network_id(),
            payer: "anonymous".to_string(),
        }))
    } else {
        Err(AppError::BadRequest("Insufficient funds".into()))
    }
}
