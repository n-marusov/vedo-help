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
