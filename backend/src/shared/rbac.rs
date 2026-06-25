use axum::{extract::State, http::Request, middleware, response::Response, Extension};

use crate::shared::auth::AuthInfo;
use crate::shared::error::AppError;

/// Create an axum middleware layer that rejects requests where the
/// authenticated user lacks the specified realm role.
///
/// # Example
///
/// ```ignore
/// use axum::middleware;
/// use vedo_backend::shared::rbac;
///
/// let admin_router = Router::new()
///     .route("/api/admin/collections", get(handler))
///     .route_layer(middleware::from_fn_with_state(
///         "admin".to_string(),
///         rbac::require_role,
///     ));
/// ```
pub async fn require_role(
    Extension(auth): Extension<AuthInfo>,
    State(required_role): State<String>,
    req: Request<axum::body::Body>,
    next: middleware::Next,
) -> Result<Response, AppError> {
    let granted = auth.user.roles.iter().any(|r| r == &required_role);

    tracing::debug!(
        component = "auth/rbac",
        user_id = %auth.user.sub,
        required_role = %required_role,
        actual_roles = %auth.user.roles.join(","),
        granted = granted,
        "rbac.authorization"
    );

    if granted {
        Ok(next.run(req).await)
    } else {
        tracing::warn!(
            component = "auth/rbac",
            user_id = %auth.user.sub,
            required_role = %required_role,
            actual_roles = %auth.user.roles.join(","),
            "rbac.denied"
        );
        Err(AppError::Forbidden(format!(
            "Required role '{}' not found",
            required_role
        )))
    }
}

/// Create an axum middleware layer that rejects requests where the
/// authenticated user lacks any of the specified realm roles.
///
/// # Example
///
/// ```ignore
/// use axum::middleware;
/// use vedo_backend::shared::rbac;
///
/// let router = Router::new()
///     .route("/api/moderator", get(handler))
///     .route_layer(middleware::from_fn_with_state(
///         vec!["admin".to_string(), "moderator".to_string()],
///         rbac::require_one_of,
///     ));
/// ```
pub async fn require_one_of(
    Extension(auth): Extension<AuthInfo>,
    State(required_roles): State<Vec<String>>,
    req: Request<axum::body::Body>,
    next: middleware::Next,
) -> Result<Response, AppError> {
    let granted = required_roles.iter().any(|r| auth.user.roles.contains(r));

    tracing::debug!(
        component = "auth/rbac",
        user_id = %auth.user.sub,
        required_roles = %required_roles.join(","),
        actual_roles = %auth.user.roles.join(","),
        granted = granted,
        "rbac.authorization"
    );

    if granted {
        Ok(next.run(req).await)
    } else {
        tracing::warn!(
            component = "auth/rbac",
            user_id = %auth.user.sub,
            required_roles = %required_roles.join(","),
            actual_roles = %auth.user.roles.join(","),
            "rbac.denied"
        );
        Err(AppError::Forbidden(format!(
            "Required one of roles '{:?}' not found",
            required_roles
        )))
    }
}
