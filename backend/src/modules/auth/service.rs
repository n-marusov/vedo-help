use crate::modules::auth::models::{UserContext, UserInfo};
use crate::shared::auth::AuthInfo;

/// Resolve public user info from the validated auth context.
pub fn resolve_user_info(auth_info: &AuthInfo) -> UserInfo {
    match auth_info {
        AuthInfo::ApiKey => UserInfo {
            sub: "admin".to_string(),
            name: Some("Admin".to_string()),
            email: None,
            preferred_username: Some("admin".to_string()),
            provider: Some("admin_api_key".to_string()),
        },
        AuthInfo::User(user) => UserInfo {
            sub: user.sub.clone(),
            name: user.name.clone(),
            email: user.email.clone(),
            preferred_username: user.preferred_username.clone(),
            provider: user.provider.clone(),
        },
    }
}

/// Build a `UserContext` from the validated auth context.
///
/// This is used internally; handlers can also use the `UserContext` extractor
/// directly (see `models.rs`).
pub fn resolve_user_context(auth_info: &AuthInfo) -> UserContext {
    UserContext::from_auth_info(auth_info)
}
