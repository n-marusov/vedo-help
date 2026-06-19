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
            collection_id TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create documents table");

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS chunks (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            "index" INTEGER NOT NULL,
            text TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
        )"#,
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

    // Verify is_active columns exist and default to 1
    let doc_cols: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM pragma_table_info('documents') WHERE name = 'is_active'",
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to query documents table info");
    assert_eq!(
        doc_cols.len(),
        1,
        "documents table must have is_active column"
    );

    let chunk_cols: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('chunks') WHERE name = 'is_active'")
            .fetch_all(&pool)
            .await
            .expect("Failed to query chunks table info");
    assert_eq!(
        chunk_cols.len(),
        1,
        "chunks table must have is_active column"
    );

    // Verify default is 1 by inserting a row without is_active and reading it back
    sqlx::query("INSERT INTO documents (id, name, file_type, file_size, uploaded_at, collection_id) VALUES ('assert_test', 'test.md', 'text/markdown', 100, '2024-01-01T00:00:00Z', 'col-1')")
        .execute(&pool)
        .await
        .expect("Failed to insert test document for is_active default check");
    let is_active_default: i64 =
        sqlx::query_scalar("SELECT is_active FROM documents WHERE id = 'assert_test'")
            .fetch_one(&pool)
            .await
            .expect("Failed to query is_active default");
    assert_eq!(is_active_default, 1, "is_active must default to 1");

    sqlx::query("INSERT INTO chunks (id, document_id, \"index\", text) VALUES ('assert_chunk', 'assert_test', 0, 'test')")
        .execute(&pool)
        .await
        .expect("Failed to insert test chunk for is_active default check");
    let chunk_is_active: i64 =
        sqlx::query_scalar("SELECT is_active FROM chunks WHERE id = 'assert_chunk'")
            .fetch_one(&pool)
            .await
            .expect("Failed to query chunk is_active default");
    assert_eq!(chunk_is_active, 1, "chunk is_active must default to 1");

    // Clean up the assert rows
    sqlx::query("DELETE FROM chunks WHERE id = 'assert_chunk'")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM documents WHERE id = 'assert_test'")
        .execute(&pool)
        .await
        .ok();

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
        keycloak_jwks_url: "http://localhost:8080".to_string(),
        keycloak_realm: "vedo-hub".to_string(),
        keycloak_client_id: "vedo-backend".to_string(),
        git_clone_root: "/tmp/test-git-repos".to_string(),
        git_sync_interval_secs: 0,
    }
}
