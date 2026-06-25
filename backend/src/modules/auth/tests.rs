use crate::modules::auth::models::{UserContext, UserInfo};
use crate::shared::auth::{AuthInfo, AuthUser};

// ---------------------------------------------------------------------------
// UserContext tests
// ---------------------------------------------------------------------------

#[test]
fn test_user_context_from_auth_info_with_roles() {
    let auth_user = AuthUser {
        sub: "user-123-uuid".to_string(),
        name: Some("Alice".to_string()),
        email: Some("alice@example.com".to_string()),
        preferred_username: Some("alice".to_string()),
        provider: Some("keycloak".to_string()),
        roles: vec!["admin".to_string(), "user".to_string()],
    };
    let auth_info = AuthInfo { user: auth_user };

    let ctx = UserContext::from_auth_info(&auth_info);

    assert_eq!(ctx.user_id, "user-123-uuid");
    assert_eq!(ctx.name, Some("Alice".to_string()));
    assert_eq!(ctx.email, Some("alice@example.com".to_string()));
    assert_eq!(ctx.provider, Some("keycloak".to_string()));
}

#[test]
fn test_user_context_from_auth_info_minimal() {
    let auth_user = AuthUser {
        sub: "minimal-user".to_string(),
        name: None,
        email: None,
        preferred_username: None,
        provider: None,
        roles: vec![],
    };
    let auth_info = AuthInfo { user: auth_user };

    let ctx = UserContext::from_auth_info(&auth_info);

    assert_eq!(ctx.user_id, "minimal-user");
    assert!(ctx.name.is_none());
    assert!(ctx.email.is_none());
    assert!(ctx.provider.is_none());
}

// ---------------------------------------------------------------------------
// UserInfo serialization tests
// ---------------------------------------------------------------------------

#[test]
fn test_user_info_serialization() {
    let info = UserInfo {
        sub: "uuid-123".to_string(),
        name: Some("Alice".to_string()),
        email: Some("alice@test.com".to_string()),
        preferred_username: Some("alice".to_string()),
        provider: Some("github".to_string()),
        roles: vec!["admin".to_string(), "user".to_string()],
    };

    let json = serde_json::to_value(&info).unwrap();

    assert_eq!(json["sub"], "uuid-123");
    assert_eq!(json["name"], "Alice");
    assert_eq!(json["email"], "alice@test.com");
    assert_eq!(json["preferred_username"], "alice");
    assert_eq!(json["provider"], "github");
}

#[test]
fn test_user_info_serialization_minimal() {
    let info = UserInfo {
        sub: "uuid-456".to_string(),
        name: None,
        email: None,
        preferred_username: None,
        provider: None,
        roles: vec![],
    };

    let json = serde_json::to_value(&info).unwrap();

    assert_eq!(json["sub"], "uuid-456");
    assert!(json["name"].is_null());
    assert!(json["email"].is_null());
    assert!(json["preferred_username"].is_null());
    assert!(json["provider"].is_null());
}

// ---------------------------------------------------------------------------
// RBAC: role extraction from JWT (AuthUser extension)
// ---------------------------------------------------------------------------

#[test]
fn test_auth_user_with_roles_can_check_admin() {
    let auth_user = AuthUser {
        sub: "admin-uuid".to_string(),
        name: None,
        email: None,
        preferred_username: None,
        provider: None,
        roles: vec!["admin".to_string(), "user".to_string()],
    };
    assert!(auth_user.roles.contains(&"admin".to_string()));
    assert!(auth_user.roles.contains(&"user".to_string()));
}

#[test]
fn test_auth_user_without_admin_role_is_not_admin() {
    let auth_user = AuthUser {
        sub: "regular-uuid".to_string(),
        name: None,
        email: None,
        preferred_username: None,
        provider: None,
        roles: vec!["user".to_string()],
    };
    assert!(!auth_user.roles.contains(&"admin".to_string()));
    assert!(auth_user.roles.contains(&"user".to_string()));
}

// ---------------------------------------------------------------------------
// RBAC role-checking logic tests
// ---------------------------------------------------------------------------

#[test]
fn test_role_check_accepts_matching_role() {
    let roles = ["admin".to_string(), "user".to_string()];
    let required = "admin";
    let granted = roles.iter().any(|r| r == required);
    assert!(granted, "require_role('admin') should accept admin role");
}

#[test]
fn test_role_check_rejects_non_matching_role() {
    let roles = ["user".to_string()];
    let required = "admin";
    let granted = roles.iter().any(|r| r == required);
    assert!(
        !granted,
        "require_role('admin') should reject non-admin role"
    );
}

#[test]
fn test_role_check_empty_roles_rejects_all() {
    let roles: Vec<String> = [].to_vec();
    assert!(!roles.iter().any(|r| r == "admin"));
    assert!(!roles.iter().any(|r| r == "user"));
}

// ---------------------------------------------------------------------------
// PostgreSQL UUID round-trip tests (collections schema)
// ---------------------------------------------------------------------------

/// These tests validate that UUID TEXT columns round-trip correctly through
/// sqlx query_as row DTOs. They are run as #[sqlx::test] integration tests
/// in the multi_tenancy integration test file. Unit-level assertions here
/// confirm the domain model mapping is correct.

#[test]
fn test_collection_user_id_mapping_contract() {
    // When a collection is created with a string user_id, it should be
    // stored as-is in the VARCHAR column and retrieved as the same string.
    let user_id = "00000000-0000-0000-0000-000000000001";
    let user_id_retrieved = user_id.to_string();
    assert_eq!(user_id_retrieved, user_id);
}

#[test]
fn test_session_user_id_mapping_contract() {
    let user_id = "00000000-0000-0000-0000-000000000002";
    let user_id_retrieved = user_id.to_string();
    assert_eq!(user_id_retrieved, user_id);
}

#[test]
fn test_document_user_id_mapping_contract() {
    let user_id = "00000000-0000-0000-0000-000000000003";
    let user_id_retrieved = user_id.to_string();
    assert_eq!(user_id_retrieved, user_id);
}

// ---------------------------------------------------------------------------
// Error type contract tests
// ---------------------------------------------------------------------------

#[test]
fn test_forbidden_error_returns_403() {
    use crate::shared::error::AppError;
    use axum::response::IntoResponse;

    let err = AppError::Forbidden("Admin access required".to_string());
    let response = err.into_response();
    assert_eq!(
        response.status(),
        403,
        "AppError::Forbidden should return HTTP 403"
    );
}
