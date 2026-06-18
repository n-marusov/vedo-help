use axum::http::{HeaderMap, HeaderValue, StatusCode};
use vedo_backend::shared::auth::authenticate_request;

// ---------------------------------------------------------------------------
// Auth Integration Tests
//
// These tests specify the *target* contract after ADMIN_API_KEY removal:
//   - No legacy API key support
//   - Only JWT tokens are accepted
//   - Requests without valid JWT → 401
// ---------------------------------------------------------------------------

/// Helper to build a HeaderMap with an Authorization bearer token.
fn bearer_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    headers
}

/// A request without any Authorization header must be rejected.
#[tokio::test]
async fn test_no_auth_header_returns_401() {
    let headers = HeaderMap::new();
    let result = authenticate_request(&headers, None).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status(), StatusCode::UNAUTHORIZED);
}

/// A request with an obviously invalid token must be rejected.
#[tokio::test]
async fn test_invalid_token_returns_401() {
    let headers = bearer_headers("this-is-not-a-valid-jwt-token");
    let result = authenticate_request(&headers, None).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status(), StatusCode::UNAUTHORIZED);
}

/// The old ADMIN_API_KEY must NO LONGER be accepted.
///
/// Before the refactor this test will FAIL because `authenticate_request`
/// still checks the API key and returns 200. After the API key path is removed
/// it will return 401 as expected — this is the core TDD cycle.
#[tokio::test]
async fn test_old_api_key_rejected_returns_401() {
    let headers = bearer_headers("test-api-key");
    let result = authenticate_request(&headers, None).await;

    // ASSERT: a static API key must NOT authenticate (only JWT tokens are valid)
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status(), StatusCode::UNAUTHORIZED);
}
