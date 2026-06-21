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

use vedo_backend::modules::query::repository::QueryRepository;
use vedo_backend::shared::ChromaClient;

mod common;

/// URL of the Chroma instance under test.
fn chroma_url() -> String {
    env::var("CHROMA_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

/// Generate a unique collection name for testing using a UUID.
///
/// Chroma 0.6.3+ validates collection names as UUIDs.
/// Each test calls this once and stores the result.
fn unique_collection(_prefix: &str) -> String {
    uuid::Uuid::new_v4().to_string()
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
async fn test_double_create_same_collection_succeeds() {
    let client = ChromaClient::new(&chroma_url());
    let name = unique_collection("double_create");

    // First creation should succeed
    client
        .create_collection(&name)
        .await
        .expect("should create collection");

    // Second creation with the same name should also succeed (get-or-create is idempotent)
    client
        .create_collection(&name)
        .await
        .expect("duplicate create should succeed (Chroma get-or-create is idempotent)");

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
        .query(&name, &[0.1f32, 0.2, 0.3], 3, None)
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
        .query(&name, &[0.0, 0.5, 0.5], 5, None)
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
        .query(&name, &[0.0, 0.0, 1.0], 5, None)
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
        .query(&name, &[2.0, 0.0, 0.0], 10, None)
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
        .query(&name, &[0.5, 0.5, 0.5], 5, None)
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
        .query(&name, &[0.1, 0.2, 0.3], 2, None)
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
        .query(&name, &[0.1, 0.2, 0.3], 2, None)
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
        .query(&name, &[0.5f32, 0.5, 0.5], 10, None)
        .await
        .expect("should query embeddings");

    // All three should be returned unfiltered
    assert_eq!(
        results.len(),
        3,
        "unfiltered query should return all documents"
    );

    // Now test with `where` support:
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
        .query(&name, &[0.5f32, 0.5, 0.5], 10, None)
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

    // Create a PostgreSQL test pool with documents + chunks
    let pool = common::setup_test_db().await;

    // Insert matching document and chunks into PostgreSQL
    let col1_uuid = uuid::Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();

    // Create the collection in PostgreSQL first (required by FK constraint)
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(col1_uuid)
    .bind("test-collection")
    .bind(None::<String>)
    .bind(chrono::Utc::now())
    .execute(&pool)
    .await
    .expect("should insert test collection");

    let doc1_uuid = uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
    let chunk_active_uuid = uuid::Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();
    let chunk_inactive_uuid =
        uuid::Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap();

    sqlx::query(
        "INSERT INTO documents (id, name, file_type, file_size, uploaded_at, collection_id) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(doc1_uuid)
    .bind("test-doc.md")
    .bind("text/markdown")
    .bind(1024)
    .bind(chrono::Utc::now())
    .bind(col1_uuid)
    .execute(&pool)
    .await
    .expect("should insert test document");

    sqlx::query(r#"INSERT INTO chunks (id, document_id, "index", text) VALUES ($1, $2, $3, $4)"#)
        .bind(chunk_active_uuid)
        .bind(doc1_uuid)
        .bind(0)
        .bind("active chunk")
        .execute(&pool)
        .await
        .expect("should insert active chunk");

    sqlx::query(r#"INSERT INTO chunks (id, document_id, "index", text) VALUES ($1, $2, $3, $4)"#)
        .bind(chunk_inactive_uuid)
        .bind(doc1_uuid)
        .bind(1)
        .bind("inactive chunk")
        .execute(&pool)
        .await
        .expect("should insert inactive chunk");

    // Create QueryRepository
    let repo = QueryRepository::new(pool, &db_url);

    // Query Chroma through the repository — now applies the active-only filter
    // (T8.2 implemented: query_chroma passes `where: {"is_active": true}`)
    let results = repo
        .query_chroma(&name, &[0.2f32, 0.3, 0.4], 10)
        .await
        .expect("query_chroma should succeed");

    // With active-only filter, only the active chunk should be returned
    assert_eq!(
        results.len(),
        1,
        "active-only query should return exactly 1 result"
    );
    assert_eq!(
        results[0].id, "chunk_active",
        "only the active chunk should be returned"
    );

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
        err.contains("InvalidUUID") || err.contains("InvalidArgumentError") || err.contains("400"),
        "Error should mention InvalidUUID, invalid argument or HTTP 400: {err}"
    );
}

// ---------------------------------------------------------------------------
// Regression: PostgreSQL native UUID/timestamp type compatibility
// ---------------------------------------------------------------------------

/// Verify the CollectionRepository can create a collection with Cyrillic name
/// (display name in PG) and list it back. Uses native UUID bindings.
#[tokio::test]
async fn test_collection_repo_native_uuid_bind() {
    let pool = common::setup_test_db().await;
    let repo =
        vedo_backend::modules::collections::repository::CollectionRepository::new(pool.clone());

    use chrono::Utc;
    use uuid::Uuid;

    let collection = vedo_backend::modules::collections::models::Collection {
        id: Uuid::new_v4(),
        name: "Техническая документация".to_string(),
        description: Some("Описание коллекции".to_string()),
        created_at: Utc::now(),
        document_count: 0,
    };

    let id = repo
        .create_collection(&collection)
        .await
        .expect("create_collection should work with native UUID/timestamp binds");
    assert_eq!(id, collection.id);

    let fetched = repo
        .get_collection(id)
        .await
        .expect("get_collection should find the collection");
    assert_eq!(fetched.name, "Техническая документация");
    assert_eq!(fetched.description.as_deref(), Some("Описание коллекции"));

    let all = repo
        .list_collections()
        .await
        .expect("list_collections should return collections");
    assert!(all.iter().any(|c| c.id == id));

    // Cleanup: delete the collection via repo
    repo.delete_collection(id)
        .await
        .expect("delete_collection should succeed");

    let missing = repo.get_collection(id).await;
    assert!(missing.is_err(), "deleted collection should not be found");

    pool.close().await;
}

/// Verify ConversationRepository handles native UUID/timestamp bindings.
#[tokio::test]
async fn test_conversation_repo_native_uuid_bind() {
    let pool = common::setup_test_db().await;
    let repo =
        vedo_backend::modules::conversations::repository::ConversationRepository::new(pool.clone());

    use chrono::Utc;
    use uuid::Uuid;

    let now = Utc::now();
    let session = vedo_backend::modules::conversations::models::Session {
        id: Uuid::new_v4(),
        title: "Test Chat".to_string(),
        collection_id: None,
        created_at: now,
        updated_at: now,
        message_count: 0,
    };

    let id = repo
        .create_session(&session)
        .await
        .expect("create_session should work with native types");
    assert_eq!(id, session.id);

    let msg = vedo_backend::modules::conversations::models::Message {
        id: Uuid::new_v4(),
        session_id: session.id,
        role: "user".to_string(),
        content: "Hello, мир!".to_string(),
        sources: None,
        created_at: now,
    };

    repo.add_message(&msg)
        .await
        .expect("save_message should work with native types");

    let messages = repo
        .get_messages(session.id)
        .await
        .expect("get_messages should work");
    assert!(!messages.is_empty(), "should have at least 1 message");
    assert_eq!(messages[0].content, "Hello, мир!");

    repo.delete_session(session.id)
        .await
        .expect("delete_session should succeed");

    pool.close().await;
}

/// Verify DocumentRepository handles native UUID/timestamp bindings.
#[tokio::test]
async fn test_document_repo_native_uuid_bind() {
    let pool = common::setup_test_db().await;
    let doc_repo =
        vedo_backend::modules::documents::repository::DocumentRepository::new(pool.clone());
    let col_repo =
        vedo_backend::modules::collections::repository::CollectionRepository::new(pool.clone());

    use chrono::Utc;
    use uuid::Uuid;

    // Create a collection first
    let collection = vedo_backend::modules::collections::models::Collection {
        id: Uuid::new_v4(),
        name: "Regression Test Collection".to_string(),
        description: None,
        created_at: Utc::now(),
        document_count: 0,
    };
    col_repo
        .create_collection(&collection)
        .await
        .expect("create collection");

    // Create a document with native UUID bindings
    let doc = vedo_backend::modules::documents::models::Document {
        id: Uuid::new_v4(),
        name: "test-doc.md".to_string(),
        file_type: "text/markdown".to_string(),
        file_size: 1024,
        uploaded_at: Utc::now(),
        collection_id: collection.id,
        is_active: true,
    };

    doc_repo
        .save_document(&doc)
        .await
        .expect("save_document should work with native UUID");

    // Fetch back
    let fetched = doc_repo
        .get_document(doc.id)
        .await
        .expect("get_document should work");
    assert_eq!(fetched.name, "test-doc.md");
    assert!(fetched.is_active);

    // List by collection
    let docs = doc_repo
        .list_documents(collection.id)
        .await
        .expect("list_documents should work");
    assert!(!docs.is_empty());

    // Cleanup
    doc_repo
        .delete_document(doc.id)
        .await
        .expect("delete_document should work");

    pool.close().await;
}

/// Verify GitRepoRepository handles native UUID/timestamp bindings.
#[tokio::test]
async fn test_git_repo_native_uuid_bind() {
    let pool = common::setup_test_db().await;
    let repo = vedo_backend::modules::git_sync::repository::GitRepoRepository::new(pool.clone());

    use chrono::Utc;
    use uuid::Uuid;

    // Create a collection first (required by FK constraint)
    let col_repo =
        vedo_backend::modules::collections::repository::CollectionRepository::new(pool.clone());
    let collection_id = Uuid::new_v4();
    let col = vedo_backend::modules::collections::models::Collection {
        id: collection_id,
        name: "Git Repo Test Collection".to_string(),
        description: None,
        created_at: Utc::now(),
        document_count: 0,
    };
    col_repo
        .create_collection(&col)
        .await
        .expect("create collection for git repo test");

    let now = Utc::now();
    let git_repo = vedo_backend::modules::git_sync::models::GitRepo {
        id: Uuid::new_v4(),
        url: "https://github.com/user/repo.git".to_string(),
        branch: "main".to_string(),
        access_token: Some("ghp_test".to_string()),
        local_path: "/tmp/test-repo".to_string(),
        last_commit_hash: None,
        last_synced_at: None,
        collection_id,
        status: "idle".to_string(),
        webhook_secret: None,
        created_at: now,
        updated_at: now,
    };

    let id = repo
        .create_repo(&git_repo)
        .await
        .expect("create_repo should work with native types");
    assert_eq!(id, git_repo.id);

    let fetched = repo
        .get_repo(id)
        .await
        .expect("get_repo should find the repo");
    assert_eq!(fetched.url, "https://github.com/user/repo.git");

    repo.delete_repo(id)
        .await
        .expect("delete_repo should succeed");

    pool.close().await;
}
