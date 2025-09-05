use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use tracing::error;

/// Application-specific error types
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    Unauthorized(String),

    #[error("Access forbidden: {0}")]
    Forbidden(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Rate limited: {0}")]
    RateLimit(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, error_code) = match &self {
            AppError::Unauthorized(msg) => {
                (StatusCode::UNAUTHORIZED, msg.clone(), "UNAUTHORIZED")
            }
            AppError::Forbidden(msg) => {
                (StatusCode::FORBIDDEN, msg.clone(), "FORBIDDEN")
            }
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, msg.clone(), "NOT_FOUND")
            }
            AppError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, msg.clone(), "BAD_REQUEST")
            }
            AppError::ValidationError(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, msg.clone(), "VALIDATION_ERROR")
            }
            AppError::Conflict(msg) => {
                (StatusCode::CONFLICT, msg.clone(), "CONFLICT")
            }
            AppError::RateLimit(msg) => {
                (StatusCode::TOO_MANY_REQUESTS, msg.clone(), "RATE_LIMITED")
            }
            AppError::Database(e) => {
                error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "A database error occurred".to_string(),
                    "DATABASE_ERROR",
                )
            }
            AppError::Jwt(e) => {
                error!("JWT error: {}", e);
                (
                    StatusCode::UNAUTHORIZED,
                    "Authentication failed".to_string(),
                    "JWT_ERROR",
                )
            }
            AppError::InternalServerError(msg) => {
                error!("Internal server error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal server error occurred".to_string(),
                    "INTERNAL_ERROR",
                )
            }
            AppError::ServiceUnavailable(msg) => {
                (StatusCode::SERVICE_UNAVAILABLE, msg.clone(), "SERVICE_UNAVAILABLE")
            }
            AppError::WebSocket(msg) => {
                error!("WebSocket error: {}", msg);
                (
                    StatusCode::BAD_REQUEST,
                    msg.clone(),
                    "WEBSOCKET_ERROR",
                )
            }
            AppError::Serialization(e) => {
                error!("Serialization error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Data serialization failed".to_string(),
                    "SERIALIZATION_ERROR",
                )
            }
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": error_message,
                "timestamp": chrono::Utc::now()
            }
        }));

        (status, body).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        error!("Anyhow error: {}", err);
        AppError::InternalServerError(err.to_string())
    }
}

/// Result type alias for the application
pub type AppResult<T> = Result<T, AppError>;

/// Helper function to create validation errors
pub fn validation_error(message: &str) -> AppError {
    AppError::ValidationError(message.to_string())
}

/// Helper function to create not found errors
pub fn not_found(resource: &str) -> AppError {
    AppError::NotFound(format!("{} not found", resource))
}

/// Helper function to create unauthorized errors
pub fn unauthorized(message: &str) -> AppError {
    AppError::Unauthorized(message.to_string())
}

/// Helper function to create forbidden errors
pub fn forbidden(message: &str) -> AppError {
    AppError::Forbidden(message.to_string())
}