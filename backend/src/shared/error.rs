use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Unified error type for the application.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Embedding service error: {0}")]
    EmbeddingError(String),

    #[error("Chroma error: {0}")]
    ChromaError(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("File error: {0}")]
    FileError(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Payload too large: {0}")]
    PayloadTooLarge(String),

    #[error("Unprocessable entity: {0}")]
    UnprocessableEntity(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type) = match &self {
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            AppError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
            AppError::EmbeddingError(_) => (StatusCode::BAD_GATEWAY, "embedding_error"),
            AppError::ChromaError(_) => (StatusCode::BAD_GATEWAY, "chroma_error"),
            AppError::LlmError(_) => (StatusCode::BAD_GATEWAY, "llm_error"),
            AppError::FileError(_) => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "file_error"),
            AppError::RateLimited(_) => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            AppError::PayloadTooLarge(_) => (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large"),
            AppError::UnprocessableEntity(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "unprocessable_entity")
            }
        };

        let body = json!({
            "error": {
                "type": error_type,
                "message": self.to_string(),
            }
        });

        (status, axum::Json(body)).into_response()
    }
}
