use crate::models::{AppError, X402Requirement};
use crate::state::SharedState;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn get_protected_resource(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let payment_address = headers
        .get("x-monero-address")
        .and_then(|h| h.to_str().ok());
    let tx_id = headers.get("x-monero-tx-id").and_then(|h| h.to_str().ok());
    let tx_key = headers.get("x-monero-tx-key").and_then(|h| h.to_str().ok());

    // If we have all proof components, verify them
    if let (Some(addr), Some(id), Some(key)) = (payment_address, tx_id, tx_key) {
        // Use '?' to return AppError::Database if query fails
        let invoice = sqlx::query!(
            "SELECT amount_required FROM invoices WHERE address = ?",
            addr
        )
        .fetch_optional(&state.db)
        .await?;

        if let Some(inv) = invoice {
            // VERIFICATION STEP:
            // We check the specific TX proof against the expected amount
            match state
                .monero
                .verify_payment_proof(id.to_string(), key.to_string(), addr.to_string())
                .await
            {
                Ok(received) => {
                    if received >= (inv.amount_required as u64) {
                        return Ok(
                            (StatusCode::OK, "ACCESS_GRANTED: Proof Verified").into_response()
                        );
                    } else {
                        return Err(AppError::Rpc("Proof amount insufficient".to_string()));
                    }
                }
                Err(e) => return Err(AppError::Rpc(e)),
            }
        }
    }

    // Otherwise, issue a fresh 402 challenge
    Ok(generate_402_challenge(state).await?)
}

async fn generate_402_challenge(state: SharedState) -> Result<Response, AppError> {
    // 1. Price Check with Error Mapping
    let amount = state
        .monero
        .get_xmr_price_piconero(state.price_per_access_usd)
        .await
        .map_err(AppError::PriceApi)?;

    // 2. Subaddress generation
    let new_address = state
        .monero
        .create_subaddress()
        .await
        .map_err(AppError::Rpc)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let amount_i64 = amount as i64;

    // 3. Database Insert
    sqlx::query!(
        "INSERT INTO invoices (address, amount_required, created_at) VALUES (?, ?, ?)",
        new_address,
        amount_i64,
        now
    )
    .execute(&state.db)
    .await?;

    let requirement = X402Requirement {
        protocol: "monero".to_string(),
        network: "stagenet".to_string(),
        amount_piconero: amount,
        address: new_address,
        invoice_id: uuid::Uuid::new_v4().to_string(),
    };

    Ok((
        StatusCode::PAYMENT_REQUIRED,
        [("WWW-Authenticate", "x402")],
        Json(requirement),
    )
        .into_response())
}
