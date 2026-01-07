use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

// --- Standard x402 Wrappers ---

#[derive(Deserialize)]
pub struct X402Request {
    #[serde(rename = "paymentPayload")]
    pub payment_payload: serde_json::Value,
    #[serde(rename = "paymentRequirements")]
    pub payment_requirements: serde_json::Value,
}

#[derive(Serialize)]
pub struct SupportedResponse {
    pub kinds: Vec<SupportedKind>,
}

#[derive(Serialize)]
pub struct SupportedKind {
    #[serde(rename = "x402Version")]
    pub x402_version: i32,
    pub scheme: String,
    pub network: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    #[serde(rename = "isValid")]
    pub is_valid: bool,
    #[serde(rename = "invalidReason")]
    pub invalid_reason: Option<String>,
}

#[derive(Serialize)]
pub struct SettleResponse {
    pub success: bool,
    pub transaction: String,
    pub network: String,
    pub payer: String,
}

// --- Monero Specific Payload (The internal content of paymentPayload) ---

#[derive(Deserialize)]
pub struct MoneroPaymentPayload {
    pub address: String,
    pub tx_id: String,
    pub tx_key: String,
}

// --- Merchant Internal API ---

#[derive(Deserialize)]
pub struct CreateInvoiceRequest {
    pub amount_usd: f64,
    pub metadata: Option<String>,
}

#[derive(Serialize)]
pub struct InvoiceResponse {
    pub address: String,
    pub amount_piconero: u64,
    pub invoice_id: String, // Restored field
    pub status: String,
    pub network: String,
}

// --- Universal Error Handler ---
pub enum AppError {
    Database(String),
    Rpc(String),
    NotFound,
    BadRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            AppError::Rpc(e) => (StatusCode::BAD_GATEWAY, e),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Invoice not found".to_string()),
            AppError::BadRequest(e) => (StatusCode::BAD_REQUEST, e),
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}
