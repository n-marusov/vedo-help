#![allow(dead_code)]

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use vedo_backend::config::AppConfig;

/// Create a PostgreSQL test pool and run migrations.
///
/// For unit/integration tests that don't use `#[sqlx::test]`, this connects to
/// a local PostgreSQL instance. Set `DATABASE_URL` env var to override the default.
///
/// # Race condition mitigation
///
/// Integration tests that share a database must NOT run in parallel because
/// `TRUNCATE ... CASCADE` wipes all tables, destroying data created by
/// concurrently running tests. Always run integration tests sequentially:
///
/// ```bash
/// cargo test --test integration -- --test-threads=1
/// ```
pub async fn setup_test_db() -> PgPool {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://vedo:test-vedo-password@localhost:15432/vedo".to_string());

    tracing::info!(
        "[integration] setting up test database: {}",
        redact_url(&db_url)
    );

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Clean all tables for a fresh test state.
    // IMPORTANT: tests using this function MUST run with --test-threads=1
    // to prevent race conditions where parallel TRUNCATE wipes data from
    // other tests mid-execution.
    tracing::info!("[integration] truncating test tables for fresh state");
    sqlx::query("TRUNCATE TABLE git_repositories, messages, sessions, chunks, documents, collections CASCADE")
        .execute(&pool)
        .await
        .expect("Failed to truncate test tables");

    tracing::info!("[integration] test database ready");
    pool
}

/// Create a test AppConfig with sensible defaults for testing.
pub fn setup_test_config() -> AppConfig {
    AppConfig {
        database_url: "postgres://vedo:test-vedo-password@localhost:15432/vedo".to_string(),
        embedding_service_url: "http://localhost:18001".to_string(),
        chroma_url: "http://localhost:18000".to_string(),
        llm_api_key: "test-openrouter-key".to_string(),
        llm_base_url: "http://llm-mock:18002".to_string(),
        llm_model: "test-model".to_string(),
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
        llm_max_history_messages: 20,
        llm_context_token_budget: 6000,
        otel_endpoint: String::new(),
        service_name: "vedo-backend-test".to_string(),
        environment: "test".to_string(),
    }
}

/// Redact the password from a database URL for safe logging.
fn redact_url(url: &str) -> String {
    if let Some(after_scheme) = url.split_once("://") {
        let scheme = after_scheme.0;
        let rest = after_scheme.1;
        if let Some((before_at, after_at)) = rest.split_once('@') {
            if let Some((user, _password)) = before_at.split_once(':') {
                return format!("{}://{}:***@{}", scheme, user, after_at);
            }
        }
    }
    url.to_string()
}
