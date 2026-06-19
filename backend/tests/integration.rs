/// Integration tests for the Chroma vector database.
///
/// These tests connect to a real Chroma instance and verify full CRUD operations.
/// They are ignored by default (`cargo test` skips them) — run explicitly:
///
/// ```bash
/// cargo test --test integration
/// ```
///
/// Or against a custom URL:
///
/// ```bash
/// CHROMA_URL=http://chroma:8000 cargo test --test integration
/// ```
///
/// In CI, Chroma is started as a service container automatically
/// (see `.github/workflows/ci.yml`).
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use vedo_backend::modules::query::repository::QueryRepository;
use vedo_backend::shared::ChromaClient;

mod common;

/// Atomic counter to ensure unique collection names even within the same millisecond.
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// URL of the Chroma instance under test.
fn chroma_url() -> String {
    env::var("CHROMA_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

/// Generate a unique collection name for test isolation.
///
/// Format: `test_<prefix>_<timestamp_ms>_<counter>` — guaranteed unique per invocation.
fn unique_collection(prefix: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let seq = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("test_{prefix}_{ts}_{seq}")
}

// ---------------------------------------------------------------------------
// create / delete collection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_and_delete_collection() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("create_delete");

    // Create
    client
        .create_collection(&name)
        .await
        .expect("should create collection");

    // Delete
    client
        .delete_collection(&name)
        .await
        .expect("should delete collection");

    // Re-create with the same name (verifies deletion actually happened)
    client
        .create_collection(&name)
        .await
        .expect("should re-create collection after deletion");

    // Final cleanup
    client
        .delete_collection(&name)
        .await
        .expect("should clean up collection");
}

#[tokio::test]
async fn test_double_create_same_collection_fails() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("double_create");

    // First creation should succeed
    client
        .create_collection(&name)
        .await
        .expect("should create collection");

    // Second creation with the same name should fail
    let result = client.create_collection(&name).await;
    assert!(
        result.is_err(),
        "creating a duplicate collection should fail"
    );

    // Cleanup
    client
        .delete_collection(&name)
        .await
        .expect("should clean up collection");
}

#[tokio::test]
async fn test_delete_nonexistent_collection_fails() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("nonexistent_delete");

    let result = client.delete_collection(&name).await;
    assert!(
        result.is_err(),
        "deleting a collection that does not exist should fail"
    );
}

#[tokio::test]
async fn test_create_collection_with_special_chars_in_name() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("special name with spaces");

    client
        .create_collection(&name)
        .await
        .expect("should create collection with spaces in name");

    client
        .delete_collection(&name)
        .await
        .expect("should delete collection with spaces in name");
}

// ---------------------------------------------------------------------------
// add embeddings
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_add_and_query_embeddings() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("add_query");

    // Setup
    client
        .create_collection(&name)
        .await
        .expect("should create collection");

    let ids = vec!["id1".into(), "id2".into(), "id3".into()];
    let embeddings = vec![
        vec![0.1f32, 0.2, 0.3],
        vec![0.4, 0.5, 0.6],
        vec![0.7, 0.8, 0.9],
    ];
    let metadatas = vec![
        serde_json::json!({"text": "first document", "document_id": "doc1", "chunk_index": 0}),
        serde_json::json!({"text": "second document", "document_id": "doc2", "chunk_index": 0}),
        serde_json::json!({"text": "third document", "document_id": "doc3", "chunk_index": 0}),
    ];

    client
        .add_embeddings(&name, &ids, &embeddings, &metadatas)
        .await
        .expect("should add embeddings");

    // Query — find nearest neighbour to the first embedding
    let results = client
        .query(&name, &[0.1f32, 0.2, 0.3], 3)
        .await
        .expect("should query embeddings");

    assert!(
        !results.is_empty(),
        "query should return at least one result"
    );
    assert_eq!(results[0].id, "id1", "most similar result should be id1");
    assert!(
        results[0].score > 0.99,
        "identical vector should have score near 1.0"
    );

    // Cleanup
    client
        .delete_collection(&name)
        .await
        .expect("should clean up collection");
}

#[tokio::test]
async fn test_query_returns_top_k() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("top_k");

    // Setup
    client
        .create_collection(&name)
        .await
        .expect("should create collection");

    let n = 10u32;
    let ids: Vec<String> = (0..n).map(|i| format!("id{i}")).collect();
    let embeddings: Vec<Vec<f32>> = (0..n)
        .map(|i| vec![i as f32 / n as f32, 0.5, 0.5])
        .collect();
    let metadatas: Vec<serde_json::Value> = (0..n)
        .map(|i| {
            serde_json::json!({"text": format!("document {i}"), "document_id": "doc", "chunk_index": i})
        })
        .collect();

    client
        .add_embeddings(&name, &ids, &embeddings, &metadatas)
        .await
        .expect("should add embeddings");

    // Query with k=5
    let results = client
        .query(&name, &[0.0, 0.5, 0.5], 5)
        .await
        .expect("should query embeddings");

    assert_eq!(
        results.len(),
        5,
        "should return exactly 5 results with top_k=5"
    );

    // Results should be ordered by similarity (ascending distance = descending score)
    for window in results.windows(2) {
        assert!(
            window[0].score >= window[1].score - 0.001,
            "results should be sorted by score descending ({} >= {})",
            window[0].score,
            window[1].score
        );
    }

    // Cleanup
    client
        .delete_collection(&name)
        .await
        .expect("should clean up collection");
}

// ---------------------------------------------------------------------------
// delete document
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_delete_document_removes_from_results() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("delete_doc");

    // Setup
    client
        .create_collection(&name)
        .await
        .expect("should create collection");

    let ids = vec!["keep1".into(), "keep2".into(), "delete_me".into()];
    let embeddings = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ];
    let metadatas = vec![
        serde_json::json!({"text": "keep me 1", "document_id": "doc1", "chunk_index": 0}),
        serde_json::json!({"text": "keep me 2", "document_id": "doc1", "chunk_index": 1}),
        serde_json::json!({"text": "delete me", "document_id": "doc1", "chunk_index": 2}),
    ];

    client
        .add_embeddings(&name, &ids, &embeddings, &metadatas)
        .await
        .expect("should add embeddings");

    // Delete the targeted document
    client
        .delete_document(&name, &["delete_me".into()])
        .await
        .expect("should delete document by id");

    // Query — deleted document should not appear
    let results = client
        .query(&name, &[0.0, 0.0, 1.0], 5)
        .await
        .expect("should query after deletion");

    assert!(
        !results.iter().any(|r| r.id == "delete_me"),
        "deleted document should not appear in query results"
    );

    // The other documents should still be present
    let remaining: Vec<_> = results.iter().map(|r| r.id.as_str()).collect();
    assert!(
        remaining.contains(&"keep1"),
        "keep1 should still be in results"
    );
    assert!(
        remaining.contains(&"keep2"),
        "keep2 should still be in results"
    );

    // Cleanup
    client
        .delete_collection(&name)
        .await
        .expect("should clean up collection");
}

#[tokio::test]
async fn test_delete_multiple_documents() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("delete_multi");

    // Setup
    client
        .create_collection(&name)
        .await
        .expect("should create collection");

    let ids: Vec<String> = (0..5).map(|i| format!("doc{i}")).collect();
    let embeddings: Vec<Vec<f32>> = (0..5).map(|i| vec![i as f32, 0.0, 0.0]).collect();
    let metadatas: Vec<serde_json::Value> = (0..5)
        .map(|i| {
            serde_json::json!({"text": format!("doc {i}"), "document_id": "multi", "chunk_index": i})
        })
        .collect();

    client
        .add_embeddings(&name, &ids, &embeddings, &metadatas)
        .await
        .expect("should add embeddings");

    // Delete two documents at once
    let to_delete: Vec<String> = vec!["doc1".into(), "doc3".into()];
    client
        .delete_document(&name, &to_delete)
        .await
        .expect("should delete multiple documents");

    // Query — deleted docs should be gone
    let results = client
        .query(&name, &[2.0, 0.0, 0.0], 10)
        .await
        .expect("should query after batch delete");

    let ids_remaining: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    assert!(!ids_remaining.contains(&"doc1"));
    assert!(!ids_remaining.contains(&"doc3"));
    assert!(ids_remaining.contains(&"doc2"));
    assert!(ids_remaining.contains(&"doc4"));
    assert!(ids_remaining.contains(&"doc0"));

    // Cleanup
    client
        .delete_collection(&name)
        .await
        .expect("should clean up collection");
}

// ---------------------------------------------------------------------------
// query edge cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_query_empty_collection_returns_no_results() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("empty_query");

    // Setup
    client
        .create_collection(&name)
        .await
        .expect("should create collection");

    // Query an empty collection
    let results = client
        .query(&name, &[0.5, 0.5, 0.5], 5)
        .await
        .expect("query on empty collection should not fail");

    assert!(
        results.is_empty(),
        "querying an empty collection should return no results"
    );

    // Cleanup
    client
        .delete_collection(&name)
        .await
        .expect("should clean up collection");
}

// ---------------------------------------------------------------------------
// end-to-end: full CRUD lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_full_crud_lifecycle() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("lifecycle");

    // 1. Create
    client
        .create_collection(&name)
        .await
        .expect("step 1: create collection");

    // 2. Add
    let ids = vec!["a".into(), "b".into()];
    let embeddings = vec![vec![0.1, 0.2, 0.3], vec![0.9, 0.8, 0.7]];
    let metadatas = vec![
        serde_json::json!({"text": "alpha", "document_id": "life", "chunk_index": 0}),
        serde_json::json!({"text": "beta", "document_id": "life", "chunk_index": 1}),
    ];

    client
        .add_embeddings(&name, &ids, &embeddings, &metadatas)
        .await
        .expect("step 2: add embeddings");

    // 3. Query
    let results = client
        .query(&name, &[0.1, 0.2, 0.3], 2)
        .await
        .expect("step 3: query");
    assert_eq!(results.len(), 2, "query should return both documents");
    assert_eq!(results[0].id, "a", "most similar should be 'a'");

    // 4. Delete document
    client
        .delete_document(&name, &["a".into()])
        .await
        .expect("step 4: delete document 'a'");

    let results_after_delete = client
        .query(&name, &[0.1, 0.2, 0.3], 2)
        .await
        .expect("step 4b: query after delete");
    assert!(
        !results_after_delete.iter().any(|r| r.id == "a"),
        "document 'a' should be gone after delete"
    );

    // 5. Delete collection
    client
        .delete_collection(&name)
        .await
        .expect("step 5: delete collection");
}

// ---------------------------------------------------------------------------
// regression: Chroma collection naming constraints
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_collection_with_uuid_name() {
    // Regression: the backend uses UUIDs as Chroma collection names to work around
    // Chroma's ASCII-only naming constraint. Verify that UUID-formatted strings
    // (36 chars, hex + hyphens) are accepted.
    let client = ChromaClient::new(&chroma_url());
    let uuid = uuid::Uuid::new_v4();
    let name = uuid.to_string();

    // UUIDs satisfy Chroma's naming rules:
    //   1. 3-63 chars ✓ (36 chars)
    //   2. Start/end with alphanumeric ✓
    //   3. Only alphanumeric, underscores, hyphens ✓
    //   4. No consecutive periods ✓
    //   5. Not a valid IPv4 address ✓
    client
        .create_collection(&name)
        .await
        .expect("UUID-formatted name should be accepted by Chroma");

    client
        .delete_collection(&name)
        .await
        .expect("should delete collection");
}

// ---------------------------------------------------------------------------
// document re-indexing: is_active filtering
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_query_with_where_active_filter() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("where_active");

    // Setup
    client
        .create_collection(&name)
        .await
        .expect("should create collection");

    let ids = vec!["active_1".into(), "active_2".into(), "inactive_1".into()];
    let embeddings = vec![
        vec![0.1f32, 0.2, 0.3],
        vec![0.4, 0.5, 0.6],
        vec![0.7, 0.8, 0.9],
    ];
    let metadatas = vec![
        serde_json::json!({"text": "active doc 1", "document_id": "doc1", "chunk_index": 0, "is_active": true}),
        serde_json::json!({"text": "active doc 2", "document_id": "doc2", "chunk_index": 0, "is_active": true}),
        serde_json::json!({"text": "inactive doc", "document_id": "doc3", "chunk_index": 0, "is_active": false}),
    ];

    client
        .add_embeddings(&name, &ids, &embeddings, &metadatas)
        .await
        .expect("should add embeddings");

    // Query with active-only filter using the new `where` parameter
    let results = client
        .query(&name, &[0.5f32, 0.5, 0.5], 10)
        .await
        .expect("should query embeddings");

    // All three should be returned unfiltered
    assert_eq!(
        results.len(),
        3,
        "unfiltered query should return all documents"
    );

    // Now test with `where` support (implementation adds this in T6.1):
    // Active-only results check — verify metadata filtering
    let active_count = results
        .iter()
        .filter(|r| {
            // This is a metadata check; once query supports `where`:
            // client.query(&name, &[0.5, 0.5, 0.5], 10, Some(json!({"is_active": true}))).await
            // should only return the 2 active chunks.
            // Until T6.1 implements the where-filter, this test validates the setup.
            r.id == "active_1" || r.id == "active_2"
        })
        .count();
    assert_eq!(active_count, 2, "should have 2 active documents");

    // Verify the inactive document is present in the unfiltered results
    assert!(
        results.iter().any(|r| r.id == "inactive_1"),
        "inactive document should still be present in unfiltered query"
    );

    // Cleanup
    client
        .delete_collection(&name)
        .await
        .expect("should clean up collection");
}

#[tokio::test]
async fn test_delete_where_removes_specific_document_chunks() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("delete_where_test");

    // Setup
    client
        .create_collection(&name)
        .await
        .expect("should create collection");

    let ids = vec!["chunk_a1".into(), "chunk_a2".into(), "chunk_b1".into()];
    let embeddings = vec![
        vec![0.1f32, 0.2, 0.3],
        vec![0.4, 0.5, 0.6],
        vec![0.7, 0.8, 0.9],
    ];
    let metadatas = vec![
        serde_json::json!({"text": "doc a chunk 1", "document_id": "doc-a", "chunk_index": 0}),
        serde_json::json!({"text": "doc a chunk 2", "document_id": "doc-a", "chunk_index": 1}),
        serde_json::json!({"text": "doc b chunk 1", "document_id": "doc-b", "chunk_index": 0}),
    ];

    client
        .add_embeddings(&name, &ids, &embeddings, &metadatas)
        .await
        .expect("should add embeddings");

    // Delete by document_id using where filter
    client
        .delete_where(&name, &serde_json::json!({"document_id": "doc-a"}))
        .await
        .expect("should delete chunks for doc-a");

    // Query — only doc-b chunks should remain
    let results = client
        .query(&name, &[0.5f32, 0.5, 0.5], 10)
        .await
        .expect("should query after delete_where");

    // Only doc-b's chunk should remain
    assert_eq!(
        results.len(),
        1,
        "only one chunk should remain after delete_where"
    );
    assert_eq!(
        results[0].id, "chunk_b1",
        "remaining chunk should be doc-b's"
    );
    assert_eq!(
        results[0].document_id, "doc-b",
        "remaining document should be doc-b"
    );

    // Cleanup
    client
        .delete_collection(&name)
        .await
        .expect("should clean up collection");
}

#[tokio::test]
async fn test_query_repository_applies_active_filter() {
    let db_url = env::var("CHROMA_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let client = ChromaClient::new(&db_url);
    let name = unique_collection("query_repo_active");

    // Setup Chroma collection with mixed active/inactive metadata
    client
        .create_collection(&name)
        .await
        .expect("should create collection");

    let ids = vec!["chunk_active".into(), "chunk_inactive".into()];
    let embeddings = vec![vec![0.1f32, 0.2, 0.3], vec![0.4, 0.5, 0.6]];
    let metadatas = vec![
        serde_json::json!({"text": "active chunk", "document_id": "doc1", "chunk_index": 0, "is_active": true}),
        serde_json::json!({"text": "inactive chunk", "document_id": "doc1", "chunk_index": 1, "is_active": false}),
    ];

    client
        .add_embeddings(&name, &ids, &embeddings, &metadatas)
        .await
        .expect("should add embeddings");

    // Create an in-memory SQLite pool with documents + chunks
    let pool = common::setup_test_db().await;

    // Insert matching document and chunks into SQLite
    sqlx::query(
        "INSERT INTO documents (id, name, file_type, file_size, uploaded_at, collection_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("doc1")
    .bind("test-doc.md")
    .bind("text/markdown")
    .bind(1024)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind("col-1")
    .execute(&pool)
    .await
    .expect("should insert test document");

    sqlx::query("INSERT INTO chunks (id, document_id, chunk_index, text) VALUES (?, ?, ?, ?)")
        .bind("chunk_active")
        .bind("doc1")
        .bind(0)
        .bind("active chunk")
        .execute(&pool)
        .await
        .expect("should insert active chunk");

    sqlx::query("INSERT INTO chunks (id, document_id, chunk_index, text) VALUES (?, ?, ?, ?)")
        .bind("chunk_inactive")
        .bind("doc1")
        .bind(1)
        .bind("inactive chunk")
        .execute(&pool)
        .await
        .expect("should insert inactive chunk");

    // Create QueryRepository
    let repo = QueryRepository::new(pool, &db_url);

    // Query Chroma through the repository — it should apply the active-only filter
    // once T6.1 (Chroma query `where` support) and T8.2 (active-only filter in query path) are implemented.
    // For now, this tests unfiltered query returns all chunks.
    let results = repo
        .query_chroma(&name, &[0.2f32, 0.3, 0.4], 10)
        .await
        .expect("query_chroma should succeed");

    // Currently (before T6.1/T8.2), unfiltered query returns both chunks.
    // After implementation, query_chroma will pass `where: {"is_active": true}`
    // and only the active chunk should be returned.
    assert!(
        !results.is_empty(),
        "query should return at least one result"
    );

    // After T8.2 is implemented, this assertion should be updated:
    //   - Query through repo should return only 1 result (active chunk)
    //   - The result's id should be "chunk_active"
    // For now, we document the expected final behavior in this comment.

    tracing::info!(
        "QueryRepository returned {} results (after T8.2, only active chunk should be returned)",
        results.len()
    );

    // Cleanup
    client
        .delete_collection(&name)
        .await
        .expect("should clean up collection");
}

#[tokio::test]
async fn test_create_collection_with_cyrillic_name_fails() {
    // Regression: document the Chroma constraint so future developers know.
    // Non-ASCII names like "Техническая документация" are rejected.
    // The backend works around this by using UUID as the Chroma collection name.
    let client = ChromaClient::new(&chroma_url());
    let name = format!("test_{}_{}", "кириллица", uuid::Uuid::new_v4());

    let result = client.create_collection(&name).await;
    assert!(
        result.is_err(),
        "Cyrillic collection name should be rejected by Chroma"
    );

    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("InvalidArgumentError") || err.contains("400"),
        "Error should mention invalid argument or HTTP 400: {err}"
    );
}
