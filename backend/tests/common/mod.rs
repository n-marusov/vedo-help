#![allow(dead_code)]

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use vedo_backend::config::AppConfig;

/// Create a PostgreSQL test pool and run migrations.
///
/// For unit/integration tests that don't use `#[sqlx::test]`, this connects to
/// a local PostgreSQL instance. Set `DATABASE_URL` env var to override the default.
pub async fn setup_test_db() -> PgPool {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://vedo:vedo@localhost:5432/vedo_test".to_string());

    tracing::info!(
        "[test_setup] connecting to PostgreSQL test database: {}",
        redact_url(&db_url)
    );

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Clean all tables for a fresh test state
    sqlx::query("TRUNCATE TABLE git_repositories, messages, sessions, chunks, documents, collections CASCADE")
        .execute(&pool)
        .await
        .expect("Failed to truncate test tables");

    pool
}

/// Create a test AppConfig with sensible defaults for testing.
pub fn setup_test_config() -> AppConfig {
    AppConfig {
        database_url: "postgres://vedo:vedo@localhost:5432/vedo_test".to_string(),
        embedding_service_url: "http://localhost:18001".to_string(),
        chroma_url: "http://localhost:18000".to_string(),
        openrouter_api_key: "test-openrouter-key".to_string(),
        openrouter_model: "test-model".to_string(),
        host: "127.0.0.1".to_string(),
        port: 0,
        rust_log: "off".to_string(),
        frontend_url: "http://localhost:5173".to_string(),
        keycloak_url: "http://localhost:8080".to_string(),
        keycloak_jwks_url: "http://localhost:8080".to_string(),
        keycloak_realm: "vedo-hub".to_string(),
        keycloak_client_id: "vedo-backend".to_string(),
        git_clone_root: "/tmp/test-git-repos".to_string(),
        git_sync_interval_secs: 0,
    }
}

/// Redact the password from a database URL for safe logging.
fn redact_url(url: &str) -> String {
    if let Some(after_scheme) = url.split_once("://") {
        let scheme = after_scheme.0;
        let rest = after_scheme.1;
        if let Some(before_at) = rest.split_once('@') {
            if let Some((user, _password)) = before_at.split_once(':') {
                let after_at = &rest[before_at.len()..];
                return format!("{}://{}:***{}", scheme, user, after_at);
            }
        }
    }
    url.to_string()
}
