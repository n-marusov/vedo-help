use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::config::AppConfig;

/// Simple auth token wrapper.
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub token: String,
}

impl AuthToken {
    /// Validate the Authorization header against the configured API key.
    pub fn check(headers: &HeaderMap, config: &AppConfig) -> Result<Self, Response> {
        let remote_addr = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|token| {
                if token == config.admin_api_key {
                    Ok(AuthToken {
                        token: token.to_string(),
                    })
                } else {
                    tracing::warn!("Unauthorized request from {remote_addr}: invalid API key");
                    Err(auth_failure_response())
                }
            })
            .unwrap_or_else(|| {
                tracing::warn!(
                    "Unauthorized request from {remote_addr}: missing or malformed auth header"
                );
                Err(auth_failure_response())
            })
    }

    /// Check if a request has a valid Authorization header without generating a response.
    pub fn is_authenticated(headers: &HeaderMap, config: &AppConfig) -> bool {
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|token| token == config.admin_api_key)
            .unwrap_or(false)
    }
}

/// Create a standardized auth failure JSON response.
fn auth_failure_response() -> Response {
    let body = json!({
        "error": {
            "type": "unauthorized",
            "message": "Invalid or missing API key"
        }
    });

    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}
