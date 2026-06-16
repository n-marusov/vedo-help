use axum::{Extension, Json};

use crate::modules::auth::models::UserInfo;
use crate::modules::auth::service;
use crate::shared::auth::AuthInfo;

/// `GET /api/auth/me` — returns current user info from JWT claims.
///
/// The auth middleware guarantees that only authenticated requests reach
/// this handler. For API-key-authenticated requests, a synthetic admin
/// user is returned.
pub async fn me(
    Extension(auth_info): Extension<AuthInfo>,
) -> Result<Json<UserInfo>, Json<serde_json::Value>> {
    tracing::debug!("GET /api/auth/me");
    let info = service::resolve_user_info(&auth_info);
    Ok(Json(info))
}

/// `POST /api/auth/logout` — client-side logout acknowledgement.
///
/// In a JWT-based SPA, token removal happens on the client side. This
/// endpoint acknowledges the logout and provides a hint that the client
/// should discard its stored tokens. For a full KeyCloak RP-initiated
/// logout, the client should redirect to the KeyCloak end_session endpoint
/// with the id_token_hint.
pub async fn logout(Extension(auth_info): Extension<AuthInfo>) -> Json<serde_json::Value> {
    tracing::info!(
        "POST /api/auth/logout — user logged out (auth={})",
        match auth_info {
            crate::shared::auth::AuthInfo::ApiKey => "api_key".to_string(),
            crate::shared::auth::AuthInfo::User(u) => format!("jwt:sub={}", u.sub),
        }
    );

    Json(serde_json::json!({
        "status": "ok",
        "message": "Logged out successfully. Remove the token on the client side.",
        "logout_url": null
    }))
}
