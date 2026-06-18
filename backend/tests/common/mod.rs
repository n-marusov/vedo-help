#![allow(dead_code)]

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

use vedo_backend::config::AppConfig;

/// Create an in-memory SQLite pool for tests.
pub async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(":memory:")
        .await
        .expect("Failed to create test database");

    // Run migrations inline for test purposes
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS documents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            file_type TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            uploaded_at TEXT NOT NULL,
            collection_id TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create documents table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS chunks (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            text TEXT NOT NULL,
            FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create chunks table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            created_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create collections table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT 'New Chat',
            collection_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE SET NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create sessions table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
            content TEXT NOT NULL,
            sources TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create messages table");

    pool
}

/// Create a test AppConfig with sensible defaults for testing.
pub fn setup_test_config() -> AppConfig {
    AppConfig {
        database_url: ":memory:".to_string(),
        embedding_service_url: "http://localhost:18001".to_string(),
        chroma_url: "http://localhost:18000".to_string(),
        openrouter_api_key: "test-openrouter-key".to_string(),
        openrouter_model: "test-model".to_string(),
        host: "127.0.0.1".to_string(),
        port: 0,
        rust_log: "off".to_string(),
        frontend_url: "http://localhost:5173".to_string(),
        keycloak_url: "http://localhost:8080".to_string(),
        keycloak_realm: "vedo-hub".to_string(),
        keycloak_client_id: "vedo-backend".to_string(),
        git_clone_root: "/tmp/test-git-repos".to_string(),
        git_sync_interval_secs: 0,
    }
}
