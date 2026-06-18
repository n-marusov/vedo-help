use crate::modules::auth::models::{UserContext, UserInfo};
use crate::shared::auth::AuthInfo;

/// Resolve public user info from the validated auth context.
pub fn resolve_user_info(auth_info: &AuthInfo) -> UserInfo {
    let user = &auth_info.user;
    UserInfo {
        sub: user.sub.clone(),
        name: user.name.clone(),
        email: user.email.clone(),
        preferred_username: user.preferred_username.clone(),
        provider: user.provider.clone(),
    }
}

/// Build a `UserContext` from the validated auth context.
///
/// This is used internally; handlers can also use the `UserContext` extractor
/// directly (see `models.rs`).
pub fn resolve_user_context(auth_info: &AuthInfo) -> UserContext {
    UserContext::from_auth_info(auth_info)
}
