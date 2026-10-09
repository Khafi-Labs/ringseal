use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Not Found: {0}")]
    NotFound(String),
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Rate Limited")]
    RateLimited,
    #[error("Session Expired")]
    SessionExpired,
    #[error("Max Attempts Exceeded")]
    MaxAttemptsExceeded,
    #[error("Invalid request body: {0}")]
    JsonRejection(String),
    #[error("Internal Server Error: {0}")]
    Internal(String),
    #[error("Database Error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<axum::extract::rejection::JsonRejection> for AppError {
    fn from(rejection: axum::extract::rejection::JsonRejection) -> Self {
        AppError::JsonRejection(rejection.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, "NOT_FOUND", m.clone()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", m.clone()),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", m.clone()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, "FORBIDDEN", m.clone()),
            AppError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                "Too many requests".to_string(),
            ),
            AppError::SessionExpired => (
                StatusCode::BAD_REQUEST,
                "SESSION_EXPIRED",
                "Session has expired".to_string(),
            ),
            AppError::MaxAttemptsExceeded => (
                StatusCode::BAD_REQUEST,
                "MAX_ATTEMPTS_EXCEEDED",
                "Maximum attempts exceeded".to_string(),
            ),
            AppError::JsonRejection(m) => (
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                m.clone(),
            ),
            AppError::Internal(m) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                m.clone(),
            ),
            AppError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                e.to_string(),
            ),
        };

        let body = Json(json!({
            "error": {
                "code": code,
                "message": message
            }
        }));

        (status, body).into_response()
    }
}
