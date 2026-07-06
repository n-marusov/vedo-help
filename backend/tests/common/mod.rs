#![allow(dead_code)]

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use vedo_backend::config::AppConfig;

/// Create a PostgreSQL test pool with fresh data but without re-running migrations
/// (migrations are idempotent via `sqlx::migrate!().run()`).
///
/// For parallel execution across test binaries, set the `TEST_DATABASE_ID` env var
/// to a unique identifier per binary (e.g., the binary name). Each binary then gets
/// its own database: `vedo_test_<ID>` (must be pre-created via the setup script).
///
/// If `TEST_DATABASE_ID` is not set, uses the default database from `DATABASE_URL`.
/// Always TRUNCATEs data tables for a clean state.
///
/// # Parallel execution
///
/// ```bash
/// TEST_DATABASE_ID="documents_db_unit" cargo test --test documents_db_unit
/// TEST_DATABASE_ID="git_sync_unit"      cargo test --test git_sync_unit
/// ```
pub async fn setup_test_db() -> PgPool {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://vedo:test-vedo-password@localhost:15432/vedo".to_string());

    // ── Resolve the actual database to connect to ──
    let binary_id = std::env::var("TEST_DATABASE_ID").unwrap_or_default();
    let target_db = if binary_id.is_empty() {
        extract_db_name(&db_url).to_string()
    } else {
        format!("vedo_test_{}", binary_id)
    };

    let target_url = replace_db_name(&db_url, &target_db);

    tracing::info!(
        "[integration] setting up test database: {}",
        redact_url(&target_url)
    );

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&target_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations — idempotent via sqlx (only applies unapplied migrations).
    // Previously this function dropped _sqlx_migrations and re-ran everything,
    // wasting 30-60s per test binary. sqlx::migrate!().run() is already
    // idempotent, so the DROP was unnecessary for sequential runs.
    //
    // For parallel runs (separate databases via TEST_DATABASE_ID), each
    // database gets its own migration state automatically.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Clean all tables for a fresh test state.
    tracing::info!("[integration] truncating test tables for fresh state");
    sqlx::query("TRUNCATE TABLE git_repositories, messages, sessions, chunks, documents, collections CASCADE")
        .execute(&pool)
        .await
        .expect("Failed to truncate test tables");

    tracing::info!("[integration] test database ready");
    pool
}

/// Extract the database name from a PostgreSQL URL.
fn extract_db_name(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or("vedo")
}

/// Replace the database name in a PostgreSQL URL.
fn replace_db_name(url: &str, new_db: &str) -> String {
    let base = url.trim_end_matches(extract_db_name(url));
    format!("{}{}", base, new_db)
}

/// Create a test AppConfig with sensible defaults for testing.
pub fn setup_test_config() -> AppConfig {
    AppConfig {
        database_url: "postgres://vedo:test-vedo-password@localhost:15432/vedo".to_string(),
        chroma_url: "http://localhost:18000".to_string(),
        llm_api_key: "test-openrouter-key".to_string(),
        llm_base_url: "http://llm-mock:18002".to_string(),
        llm_model: "test-model".to_string(),
        embedding_api_key: "test-embedding-key".to_string(),
        embedding_base_url: "https://routerai.ru/api/v1".to_string(),
        embedding_model: "sentence-transformers/all-minilm-l6-v2".to_string(),
        embedding_cache_size: 1000,
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
        advanced_rag_enabled: true,
        rerank_top_k: 5,
        hybrid_top_k: 20,
        multi_query_count: 3,
        llm_rerank_model: "test-model".to_string(),
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
