use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::modules::collections::models::Collection;
use crate::modules::collections::repository::CollectionRepository;
use crate::shared::error::AppError;

/// Create an in-memory SQLite pool with the collections table migrated.
async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(":memory:")
        .await
        .expect("Failed to create in-memory SQLite pool");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS collections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            created_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create collections table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS documents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            file_type TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            uploaded_at TEXT NOT NULL,
            collection_id TEXT NOT NULL,
            FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create documents table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS chunks (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            "index" INTEGER NOT NULL,
            text TEXT NOT NULL,
            FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create chunks table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT 'New Chat',
            collection_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE SET NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create sessions table");

    pool
}

/// Helper to create a collection with default test data.
fn make_collection(id: Uuid, name: &str, description: Option<&str>) -> Collection {
    Collection {
        id,
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        created_at: chrono::Utc::now(),
        document_count: 0,
    }
}

// ---------------------------------------------------------------------------
// CREATE
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_collection() {
    tracing::debug!("Running test: test_create_collection");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);
    let id = Uuid::new_v4();
    let col = make_collection(id, "Test Collection", Some("A test description"));

    let result = repo.create_collection(&col).await;
    assert!(result.is_ok(), "create_collection should succeed");
    assert_eq!(result.unwrap(), id);

    // Verify it appears in the list
    let collections = repo.list_collections().await.unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].name, "Test Collection");
    assert_eq!(
        collections[0].description.as_deref(),
        Some("A test description")
    );
}

#[tokio::test]
async fn test_create_duplicate_collection_fails() {
    tracing::debug!("Running test: test_create_duplicate_collection_fails");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);

    let id1 = Uuid::new_v4();
    let col1 = make_collection(id1, "Duplicate", None);
    repo.create_collection(&col1).await.unwrap();

    let id2 = Uuid::new_v4();
    let col2 = make_collection(id2, "Duplicate", None);
    let result = repo.create_collection(&col2).await;

    assert!(result.is_err(), "duplicate name should fail");
    match result.unwrap_err() {
        AppError::BadRequest(msg) => {
            assert!(
                msg.contains("Duplicate"),
                "error message should mention the collection name"
            );
        }
        other => panic!("expected BadRequest, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// LIST
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_list_collections() {
    tracing::debug!("Running test: test_list_collections");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    repo.create_collection(&make_collection(id1, "Alpha", None))
        .await
        .unwrap();
    repo.create_collection(&make_collection(id2, "Beta", None))
        .await
        .unwrap();

    let collections = repo.list_collections().await.unwrap();
    assert_eq!(collections.len(), 2);
}

#[tokio::test]
async fn test_list_collections_empty() {
    tracing::debug!("Running test: test_list_collections_empty");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);

    let collections = repo.list_collections().await.unwrap();
    assert!(
        collections.is_empty(),
        "empty database should return empty list"
    );
}

// ---------------------------------------------------------------------------
// GET
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_collection_by_id() {
    tracing::debug!("Running test: test_get_collection_by_id");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);

    let id = Uuid::new_v4();
    repo.create_collection(&make_collection(id, "Get Me", None))
        .await
        .unwrap();

    let col = repo.get_collection(id).await.unwrap();
    assert_eq!(col.id, id);
    assert_eq!(col.name, "Get Me");
}

#[tokio::test]
async fn test_get_nonexistent_collection_fails() {
    tracing::debug!("Running test: test_get_nonexistent_collection_fails");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);

    let id = Uuid::new_v4();
    let result = repo.get_collection(id).await;

    assert!(
        result.is_err(),
        "getting nonexistent collection should fail"
    );
    match result.unwrap_err() {
        AppError::NotFound(_) => {} // expected
        other => panic!("expected NotFound, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// DELETE
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_delete_collection() {
    tracing::debug!("Running test: test_delete_collection");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);

    let id = Uuid::new_v4();
    repo.create_collection(&make_collection(id, "To Delete", None))
        .await
        .unwrap();

    // Delete
    let result = repo.delete_collection(id).await;
    assert!(result.is_ok(), "delete should succeed");

    // Verify it's gone
    let get_result = repo.get_collection(id).await;
    assert!(
        get_result.is_err(),
        "deleted collection should not be found"
    );
}

#[tokio::test]
async fn test_delete_nonexistent_collection_fails() {
    tracing::debug!("Running test: test_delete_nonexistent_collection_fails");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);

    let id = Uuid::new_v4();
    let result = repo.delete_collection(id).await;

    assert!(
        result.is_err(),
        "deleting nonexistent collection should fail"
    );
    match result.unwrap_err() {
        AppError::NotFound(_) => {} // expected
        other => panic!("expected NotFound, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// UPDATE (rename)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_update_collection_name() {
    tracing::debug!("Running test: test_update_collection_name");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);

    let id = Uuid::new_v4();
    repo.create_collection(&make_collection(
        id,
        "Old Name",
        Some("Original description"),
    ))
    .await
    .unwrap();

    // Update name only
    let result = repo
        .update_collection(id, &Some("New Name".into()), &None)
        .await;
    assert!(result.is_ok(), "update name should succeed");

    // Verify
    let col = repo.get_collection(id).await.unwrap();
    assert_eq!(col.name, "New Name", "name should be updated");
    assert_eq!(
        col.description.as_deref(),
        Some("Original description"),
        "description should remain unchanged"
    );
}

#[tokio::test]
async fn test_update_collection_description() {
    tracing::debug!("Running test: test_update_collection_description");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);

    let id = Uuid::new_v4();
    repo.create_collection(&make_collection(id, "Stable Name", Some("Old desc")))
        .await
        .unwrap();

    // Update description only
    let result = repo
        .update_collection(id, &None, &Some("New description".into()))
        .await;
    assert!(result.is_ok(), "update description should succeed");

    // Verify
    let col = repo.get_collection(id).await.unwrap();
    assert_eq!(
        col.description.as_deref(),
        Some("New description"),
        "description should be updated"
    );
    assert_eq!(col.name, "Stable Name", "name should remain unchanged");
}

#[tokio::test]
async fn test_update_nonexistent_collection_fails() {
    tracing::debug!("Running test: test_update_nonexistent_collection_fails");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);

    let id = Uuid::new_v4();
    let result = repo
        .update_collection(id, &Some("New Name".into()), &None)
        .await;

    assert!(
        result.is_err(),
        "updating nonexistent collection should fail"
    );
    match result.unwrap_err() {
        AppError::NotFound(_) => {} // expected
        other => panic!("expected NotFound, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_update_collection_duplicate_name_fails() {
    tracing::debug!("Running test: test_update_collection_duplicate_name_fails");
    let db = setup_test_db().await;
    let repo = CollectionRepository::new(db);

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    repo.create_collection(&make_collection(id1, "Alpha", None))
        .await
        .unwrap();
    repo.create_collection(&make_collection(id2, "Beta", None))
        .await
        .unwrap();

    // Try to rename Beta to Alpha
    let result = repo
        .update_collection(id2, &Some("Alpha".into()), &None)
        .await;

    assert!(result.is_err(), "renaming to existing name should fail");
    match result.unwrap_err() {
        AppError::BadRequest(msg) => {
            assert!(
                msg.contains("Alpha"),
                "error message should mention the duplicate name"
            );
        }
        other => panic!("expected BadRequest, got: {other:?}"),
    }
}
