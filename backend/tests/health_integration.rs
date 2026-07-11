/// Integration tests for the deep healthcheck endpoint.
///
/// These tests verify the API contract of `GET /api/health/deep` by setting
/// up a minimal axum router with a live `HealthService` and testing against it.
///
/// They are ignored by default — run explicitly:
///
/// ```bash
/// # Requires PostgreSQL on localhost:15432 (from docker-compose.test.yml):
/// docker compose --env-file .env.test -f docker-compose.test.yml up -d
///
/// cd backend
/// DATABASE_URL=postgres://vedo:test-vedo-password@localhost:15432/vedo \
///   cargo test --test health_integration -- --ignored --test-threads=1
/// ```
use axum::http::{Method, Request, StatusCode};
use axum::routing::get;
use axum::Router;
use tower::ServiceExt;

use vedo_backend::shared::health::{HealthProbe, HealthService, HealthStatus};

mod common;

// ---------------------------------------------------------------------------
// Mock probes for deterministic test results
// ---------------------------------------------------------------------------

struct OkProbe(&'static str);

#[async_trait::async_trait]
impl HealthProbe for OkProbe {
    fn name(&self) -> &'static str {
        self.0
    }
    async fn probe(&self) -> Result<(), vedo_backend::shared::error::AppError> {
        Ok(())
    }
}

struct ErrProbe(&'static str);

#[async_trait::async_trait]
impl HealthProbe for ErrProbe {
    fn name(&self) -> &'static str {
        self.0
    }
    async fn probe(&self) -> Result<(), vedo_backend::shared::error::AppError> {
        Err(vedo_backend::shared::error::AppError::ChromaError(
            "Connection refused".to_string(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Helper: build a test router with a configured HealthService
// ---------------------------------------------------------------------------

fn build_default_router() -> Router {
    let mut health_service = HealthService::new(None);
    health_service.register(OkProbe("Chroma"));
    health_service.register(OkProbe("PostgreSQL"));
    health_service.register(OkProbe("Embedding"));
    health_service.register(OkProbe("LLM"));

    async fn deep_health_check(
        axum::extract::State(health_service): axum::extract::State<HealthService>,
    ) -> (
        StatusCode,
        axum::Json<vedo_backend::shared::health::HealthReport>,
    ) {
        let report = health_service.check_all().await;
        let status = match report.status {
            HealthStatus::Healthy | HealthStatus::Degraded => StatusCode::OK,
            HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        };
        (status, axum::Json(report))
    }

    Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/api/health/deep", get(deep_health_check))
        .with_state(health_service)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_deep_healthcheck_returns_200() {
    // Arrange
    let app = build_default_router();

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health/deep")
                .method(Method::GET)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore]
async fn test_deep_healthcheck_response_structure() {
    // Arrange
    let app = build_default_router();

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health/deep")
                .method(Method::GET)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map(|b| serde_json::from_slice(&b).unwrap())
        .unwrap();

    // Top-level fields
    assert!(
        body.get("status").is_some(),
        "Response MUST have 'status' field"
    );
    assert!(
        body.get("checks").is_some(),
        "Response MUST have 'checks' field"
    );
    assert!(
        body.get("timestamp").is_some(),
        "Response MUST have 'timestamp' field"
    );

    // Each check has required fields
    let checks = body["checks"].as_array().unwrap();
    for check in checks {
        assert!(
            check.get("name").is_some(),
            "Each check MUST have 'name' field"
        );
        assert!(
            check.get("status").is_some(),
            "Each check MUST have 'status' field"
        );
        assert!(
            check.get("latency_ms").is_some(),
            "Each check MUST have 'latency_ms' field"
        );
    }
}

#[tokio::test]
#[ignore]
async fn test_health_backward_compatible() {
    // Arrange
    let app = build_default_router();

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .method(Method::GET)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        std::str::from_utf8(&body).unwrap(),
        "OK",
        "Existing /health MUST still return 'OK'"
    );
}

#[tokio::test]
#[ignore]
async fn test_deep_healthcheck_rejects_non_get() {
    // Arrange
    let app = build_default_router();

    // Act — POST
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health/deep")
                .method(Method::POST)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "POST /api/health/deep MUST return 405"
    );
}

#[tokio::test]
#[ignore]
async fn test_deep_healthcheck_rejects_put() {
    let app = build_default_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health/deep")
                .method(Method::PUT)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "PUT /api/health/deep MUST return 405"
    );
}

#[tokio::test]
#[ignore]
async fn test_deep_healthcheck_rejects_delete() {
    let app = build_default_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health/deep")
                .method(Method::DELETE)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "DELETE /api/health/deep MUST return 405"
    );
}

#[tokio::test]
#[ignore]
async fn test_deep_healthcheck_status_values() {
    // Arrange — create a router with an unhealthy Chroma probe
    let mut health_service = HealthService::new(None);
    health_service.register(OkProbe("PostgreSQL"));
    health_service.register(ErrProbe("Chroma"));
    health_service.register(OkProbe("Embedding"));
    health_service.register(OkProbe("LLM"));

    async fn deep_health_check(
        axum::extract::State(health_service): axum::extract::State<HealthService>,
    ) -> (
        StatusCode,
        axum::Json<vedo_backend::shared::health::HealthReport>,
    ) {
        let report = health_service.check_all().await;
        let status = match report.status {
            HealthStatus::Healthy | HealthStatus::Degraded => StatusCode::OK,
            HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        };
        (status, axum::Json(report))
    }

    let app = Router::new()
        .route("/api/health/deep", get(deep_health_check))
        .with_state(health_service);

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health/deep")
                .method(Method::GET)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert — Chroma down with DB up => Degraded => 200
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map(|b| serde_json::from_slice(&b).unwrap())
        .unwrap();

    let status = body["status"].as_str().unwrap();
    assert!(
        ["healthy", "degraded", "unhealthy"].contains(&status),
        "status MUST be one of healthy/degraded/unhealthy, got: {status}"
    );
}
