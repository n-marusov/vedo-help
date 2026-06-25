use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Extension;
use serde_json::json;

use crate::modules::auth::models::UserContext;
use crate::shared::auth::AuthInfo;

/// Axum `FromRequestParts` extractor that automatically resolves the
/// current `UserContext` from the authenticated request's `AuthInfo`.
///
/// Use this extractor in any handler behind the auth middleware:
///
/// ```ignore
/// use vedo_backend::shared::user_context::UserContext;
///
/// async fn my_handler(user_ctx: UserContext) -> impl IntoResponse {
///     tracing::info!("User {} is accessing this handler", user_ctx.user_id);
/// }
/// ```
///
/// Returns a 401 response if `AuthInfo` is missing from the request
/// extensions (which should never happen behind the auth middleware).
#[async_trait]
impl<S> FromRequestParts<S> for UserContext
where
    S: Send + Sync,
{
    type Rejection = UserContextRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Extension(auth_info) = Extension::<AuthInfo>::from_request_parts(parts, state)
            .await
            .map_err(|_| UserContextRejection::MissingAuth)?;

        Ok(UserContext::from_auth_info(&auth_info))
    }
}

/// Rejection type for `UserContext` extraction failures.
#[derive(Debug)]
pub enum UserContextRejection {
    MissingAuth,
}

impl IntoResponse for UserContextRejection {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            UserContextRejection::MissingAuth => (
                StatusCode::UNAUTHORIZED,
                json!({
                    "error": {
                        "type": "unauthorized",
                        "message": "Authentication required"
                    }
                }),
            ),
        };

        (status, axum::Json(body)).into_response()
    }
}
