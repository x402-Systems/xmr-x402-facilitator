use crate::models::*;
use crate::state::SharedState;
use axum::{Json, extract::State};

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
    let payload: MoneroPaymentPayload = serde_json::from_value(req.payment_payload)
        .map_err(|_| AppError::BadRequest("Invalid Monero payload".into()))?;

    let invoice = sqlx::query!(
        "SELECT amount_required FROM invoices WHERE address = ?",
        payload.address
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let (received, _) = state
        .monero
        .verify_payment_proof(payload.tx_id, payload.tx_key, payload.address)
        .await
        .map_err(AppError::Rpc)?;

    let is_valid = received >= (invoice.amount_required as u64);

    Ok(Json(VerifyResponse {
        is_valid,
        invalid_reason: if is_valid {
            None
        } else {
            Some("Insufficient amount".into())
        },
    }))
}

/// POST /settle
pub async fn settle_payment(
    State(state): State<SharedState>,
    Json(req): Json<X402Request>,
) -> Result<Json<SettleResponse>, AppError> {
    let payload: MoneroPaymentPayload = serde_json::from_value(req.payment_payload)
        .map_err(|_| AppError::BadRequest("Invalid Monero payload".into()))?;

    let invoice = sqlx::query!(
        "SELECT amount_required FROM invoices WHERE address = ?",
        payload.address
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let (received, confirmations) = state
        .monero
        .verify_payment_proof(
            payload.tx_id.clone(),
            payload.tx_key.clone(),
            payload.address.clone(),
        )
        .await
        .map_err(AppError::Rpc)?;

    let required_confs = std::env::var("CONFIRMATIONS_REQUIRED")
        .unwrap_or_else(|_| "0".to_string())
        .parse::<u64>()
        .unwrap_or(0);

    if received >= (invoice.amount_required as u64) && confirmations >= required_confs {
        sqlx::query!(
            "UPDATE invoices SET status = 'paid', tx_id = ? WHERE address = ?",
            payload.tx_id,
            payload.address
        )
        .execute(&state.db)
        .await?;

        Ok(Json(SettleResponse {
            success: true,
            transaction: payload.tx_id,
            network: get_network_id(), // Dynamic network name
            payer: "anonymous".to_string(),
        }))
    } else {
        Err(AppError::BadRequest(format!(
            "Payment failed. Received: {}/{} piconero. Confirmations: {}/{}",
            received, invoice.amount_required, confirmations, required_confs
        )))
    }
}
