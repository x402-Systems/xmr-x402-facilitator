use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct X402Requirement {
    pub protocol: String,     // "monero"
    pub network: String,      // "mainnet" or "stagenet"
    pub amount_piconero: u64, // Atomic units
    pub address: String,      // The unique subaddress
    pub invoice_id: String,   // To track the session
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub enum AppError {
    Database(String),
    Rpc(String),
    PriceApi(String),
    NotFound,
}

// This allows us to use '?' in handlers and have it auto-convert to an HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            AppError::Rpc(e) => (StatusCode::BAD_GATEWAY, e),
            AppError::PriceApi(e) => (StatusCode::SERVICE_UNAVAILABLE, e),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

// Helper to convert sqlx errors
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}
