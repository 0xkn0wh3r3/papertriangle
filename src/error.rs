//! Error type with an axum [`IntoResponse`] implementation.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Application error. Every variant renders as a small JSON body flagged
/// `"mock": true` so no caller mistakes it for a real service error.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NotFound"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BadRequest"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal"),
        };
        let body = Json(json!({
            "error": code,
            "message": self.to_string(),
            "mock": true,
        }));
        (status, body).into_response()
    }
}

/// Convenience result alias.
pub type AppResult<T> = Result<T, AppError>;
