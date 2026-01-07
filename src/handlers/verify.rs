use crate::models::*;
use crate::state::SharedState;
use axum::{Json, extract::State};

pub async fn verify_payment(
    State(state): State<SharedState>,
    Json(payload): Json<VerifyRequest>,
) -> Result<Json<StatusResponse>, AppError> {
    // 1. Check if invoice exists
    let invoice = sqlx::query!(
        "SELECT amount_required, status FROM invoices WHERE address = ?",
        payload.address
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if invoice.status == Some("paid".to_string()) {
        return Ok(Json(StatusResponse {
            status: "paid".to_string(),
            amount_received: invoice.amount_required as u64,
        }));
    }

    // 2. Cryptographic Proof Check
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

    if received >= (invoice.amount_required as u64) {
        if confirmations >= required_confs {
            // FULLY SETTLED
            sqlx::query!(
                "UPDATE invoices SET status = 'paid', tx_id = ? WHERE address = ?",
                payload.tx_id,
                payload.address
            )
            .execute(&state.db)
            .await?;

            Ok(Json(StatusResponse {
                status: "paid".to_string(),
                amount_received: received,
            }))
        } else {
            // RECEIVED BUT UNCONFIRMED
            Ok(Json(StatusResponse {
                status: "waiting_confirmations".to_string(),
                amount_received: received,
            }))
        }
    } else {
        Ok(Json(StatusResponse {
            status: "insufficient".to_string(),
            amount_received: received,
        }))
    }
}
