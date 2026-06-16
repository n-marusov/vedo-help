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
}

impl UserContext {
    /// Build a `UserContext` from validated auth info.
    pub fn from_auth_info(auth_info: &AuthInfo) -> Self {
        match auth_info {
            AuthInfo::ApiKey => Self {
                user_id: "admin".to_string(),
                name: Some("Admin".to_string()),
                email: None,
                provider: Some("admin_api_key".to_string()),
            },
            AuthInfo::User(user) => Self {
                user_id: user.sub.clone(),
                name: user.name.clone(),
                email: user.email.clone(),
                provider: user.provider.clone(),
            },
        }
    }
}
