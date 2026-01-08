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
    println!("📥 RECEIVED VERIFY REQUEST");

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

    // Retry loop: Don't use '?' inside here if we want to actually retry!
    for i in 0..15 {
        match state
            .monero
            .verify_payment_proof(
                inner.tx_id.clone(),
                inner.tx_key.clone(),
                inner.address.clone(),
            )
            .await
        {
            Ok((rec, _)) => {
                received = rec;
                if received >= required {
                    println!(
                        "✅ Attempt {}: Success! Received {} piconero",
                        i + 1,
                        received
                    );
                    break;
                }
                println!(
                    "⏳ Attempt {}: Found TX but amount {} < {}",
                    i + 1,
                    received,
                    required
                );
            }
            Err(e) => {
                println!("⏳ Attempt {}: TX not in mempool yet... ({})", i + 1, e);
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let is_valid = received >= required;

    if !is_valid {
        println!("❌ VERIFICATION FAILED after 15 attempts");
    }

    Ok(Json(VerifyResponse {
        is_valid,
        invalid_reason: if is_valid {
            None
        } else {
            Some("Transaction not found or insufficient".into())
        },
    }))
}

pub async fn settle_payment(
    State(state): State<SharedState>,
    Json(req): Json<X402Request>,
) -> Result<Json<SettleResponse>, AppError> {
    println!("📥 RECEIVED SETTLE REQUEST");
    let inner = req.payment_payload.payload;

    let invoice = sqlx::query!(
        "SELECT amount_required, payer_id FROM invoices WHERE address = ?",
        inner.address
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    // We don't need a loop in settle because verify already caught it
    let (received, _) = state
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

        let resolved_payer = invoice.payer_id.unwrap_or_else(|| "anonymous".to_string());

        println!("🎉 SETTLEMENT SUCCESS: Tx {}", inner.tx_id);
        Ok(Json(SettleResponse {
            success: true,
            transaction: inner.tx_id,
            network: get_network_id(),
            payer: resolved_payer,
        }))
    } else {
        println!("❌ SETTLE FAILED: Received only {}", received);
        Err(AppError::BadRequest("Insufficient funds".into()))
    }
}
