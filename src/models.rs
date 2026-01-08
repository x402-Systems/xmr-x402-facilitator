use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

// --- Standard x402 Wrappers ---

#[derive(Deserialize, Serialize, Debug)]
pub struct X402Request {
    #[serde(rename = "paymentPayload", alias = "payment_payload")]
    pub payment_payload: X402PaymentPayloadWrapper,
    #[serde(rename = "paymentRequirements", alias = "payment_requirements")]
    pub payment_requirements: serde_json::Value,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct X402PaymentPayloadWrapper {
    #[serde(rename = "x402Version")]
    pub x402_version: i32,
    pub payload: MoneroPaymentPayload,
}

// --- Monero Specific Payload ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MoneroPaymentPayload {
    pub address: String,
    #[serde(alias = "txId")]
    pub tx_id: String,
    #[serde(alias = "txKey")]
    pub tx_key: String,
}

// --- Rest of the file (InvoiceResponse, AppError, etc.) remains the same ---
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

#[derive(Deserialize)]
pub struct CreateInvoiceRequest {
    pub amount_usd: f64,
    pub metadata: Option<String>,
    pub payer_id: Option<String>,
}

#[derive(Serialize)]
pub struct InvoiceResponse {
    pub address: String,
    pub amount_piconero: u64,
    pub invoice_id: String,
    pub status: String,
    pub network: String,
}

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
    fn from(err: sqlx::Error) -> self::AppError {
        AppError::Database(err.to_string())
    }
}
