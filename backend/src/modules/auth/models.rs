use serde::Serialize;

use crate::shared::auth::AuthInfo;

/// Public user information returned by the auth endpoints.
#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    pub sub: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
    pub provider: Option<String>,
    /// Realm roles extracted from the JWT.
    #[serde(default)]
    pub roles: Vec<String>,
}

/// User context extracted from the current request's auth info.
///
/// Handlers behind the auth middleware can construct this from the
/// `Extension<AuthInfo>` extractor:
///
/// ```ignore
/// use axum::Extension;
/// use vedo_backend::modules::auth::models::UserContext;
/// use vedo_backend::shared::auth::AuthInfo;
///
/// async fn my_handler(Extension(auth): Extension<AuthInfo>) -> impl IntoResponse {
///     let ctx = UserContext::from_auth_info(&auth);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub provider: Option<String>,
    /// Realm roles extracted from the JWT.
    pub roles: Vec<String>,
}

impl UserContext {
    /// Build a `UserContext` from validated auth info.
    pub fn from_auth_info(auth_info: &AuthInfo) -> Self {
        let user = &auth_info.user;
        Self {
            user_id: user.sub.clone(),
            name: user.name.clone(),
            email: user.email.clone(),
            provider: user.provider.clone(),
            roles: user.roles.clone(),
        }
    }
}
