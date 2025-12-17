//! Application Error Types
//!
//! Centralized error handling with Axum integration.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Application error type
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Validation error: {0}")]
    Validation(String),
}

/// Error response body
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
}

/// Field-level validation error
#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, 10001, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, 10002, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, 10003, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, 10004, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, 10005, msg.clone()),
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, 10006, "Rate limited".into()),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, 10007, msg.clone()),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, 10000, "Internal server error".into())
            }
            AppError::Database(e) => {
                tracing::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, 10000, "Internal server error".into())
            }
            AppError::Redis(e) => {
                tracing::error!("Redis error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, 10000, "Internal server error".into())
            }
        };

        let body = ErrorResponse {
            code,
            message,
            errors: None,
        };

        (status, Json(body)).into_response()
    }
}
