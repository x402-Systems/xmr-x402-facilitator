use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateInvoiceRequest {
    pub amount_usd: f64,
    pub metadata: Option<String>,
}

#[derive(Serialize)]
pub struct InvoiceResponse {
    pub address: String,
    pub amount_piconero: u64,
    pub invoice_id: String,
    pub status: String,
    pub network: String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub address: String,
    pub tx_id: String,
    pub tx_key: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub amount_received: u64,
}

// Universal Error Handler
pub enum AppError {
    Database(String),
    Rpc(String),
    NotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            AppError::Rpc(e) => (StatusCode::BAD_GATEWAY, e),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Invoice not found".to_string()),
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}
