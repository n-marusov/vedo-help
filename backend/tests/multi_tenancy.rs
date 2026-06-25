/// Integration tests for multi-tenancy — user_id scoping in repositories.
///
/// These tests require a running PostgreSQL instance with migrations applied.
/// Run sequentially to avoid TRUNCATE race conditions:
///
/// ```bash
/// cargo test --test multi_tenancy -- --test-threads=1
/// ```
///
/// Prerequisites:
///   - PostgreSQL running at the configured DATABASE_URL
///   - Migrations applied (including user_id columns)
use sqlx::PgPool;
use uuid::Uuid;

use vedo_backend::modules::collections::repository::CollectionRepository;
use vedo_backend::modules::conversations::repository::ConversationRepository;
use vedo_backend::modules::documents::repository::DocumentRepository;

mod common;

/// Test user IDs — fixed UUIDs for reproducible test assertions.
const USER_A_ID: &str = "00000000-0000-0000-0000-000000000001";
const USER_B_ID: &str = "00000000-0000-0000-0000-000000000002";
// ---------------------------------------------------------------------------
// Helper: run TRUNCATE to get a clean slate
// ---------------------------------------------------------------------------

async fn truncate_all(pool: &PgPool) {
    sqlx::query(
        "TRUNCATE TABLE git_repositories, messages, sessions, chunks, documents, collections CASCADE",
    )
    .execute(pool)
    .await
    .expect("Failed to truncate test tables");
}

// ---------------------------------------------------------------------------
// Collection multi-tenancy tests
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_collections_are_scoped_by_user(pool: PgPool) -> sqlx::Result<()> {
    truncate_all(&pool).await;

    let repo = CollectionRepository::new(pool.clone());

    // Create collections for user A and user B
    let coll_a_id = Uuid::new_v4();
    let coll_b_id = Uuid::new_v4();
    let coll_name_a = format!("user-a-collection-{}", Uuid::new_v4());
    let coll_name_b = format!("user-b-collection-{}", Uuid::new_v4());

    // Insert collections with user_id
    sqlx::query(
        "INSERT INTO collections (id, name, description, user_id, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(coll_a_id)
    .bind(&coll_name_a)
    .bind("User A's collection")
    .bind(USER_A_ID)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO collections (id, name, description, user_id, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(coll_b_id)
    .bind(&coll_name_b)
    .bind("User B's collection")
    .bind(USER_B_ID)
    .execute(&pool)
    .await?;

    // User A should see only their own collection
    let colls_a = repo
        .list_collections_by_user(USER_A_ID, false)
        .await
        .unwrap();
    assert_eq!(colls_a.len(), 1, "User A should see exactly 1 collection");
    assert_eq!(colls_a[0].id, coll_a_id, "User A's collection should match");

    // User B should see only their own collection
    let colls_b = repo
        .list_collections_by_user(USER_B_ID, false)
        .await
        .unwrap();
    assert_eq!(colls_b.len(), 1, "User B should see exactly 1 collection");
    assert_eq!(colls_b[0].id, coll_b_id, "User B's collection should match");

    // Verify user A cannot find user B's collection by ID
    let found = repo
        .find_by_id_and_user(coll_b_id, USER_A_ID)
        .await
        .unwrap();
    assert!(
        found.is_none(),
        "User A should NOT find User B's collection"
    );

    Ok(())
}

#[sqlx::test]
async fn test_collection_ownership_verification(pool: PgPool) -> sqlx::Result<()> {
    truncate_all(&pool).await;

    let coll_id = Uuid::new_v4();
    let coll_name = format!("ownership-test-{}", Uuid::new_v4());

    sqlx::query(
        "INSERT INTO collections (id, name, description, user_id, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(coll_id)
    .bind(&coll_name)
    .bind("Ownership test")
    .bind(USER_A_ID)
    .execute(&pool)
    .await?;

    // User A should be able to find and delete their own collection
    let repo = CollectionRepository::new(pool.clone());
    let found = repo.find_by_id_and_user(coll_id, USER_A_ID).await.unwrap();
    assert!(found.is_some(), "User A should find their own collection");

    // User B should NOT find it (ownership check)
    let not_found = repo.find_by_id_and_user(coll_id, USER_B_ID).await.unwrap();
    assert!(
        not_found.is_none(),
        "User B should NOT find User A's collection"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Document multi-tenancy tests
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_documents_are_scoped_via_collection_ownership(pool: PgPool) -> sqlx::Result<()> {
    truncate_all(&pool).await;

    let coll_a_id = Uuid::new_v4();
    let coll_b_id = Uuid::new_v4();
    let doc_a_id = Uuid::new_v4();

    // Create collections owned by different users
    sqlx::query(
        "INSERT INTO collections (id, name, description, user_id, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(coll_a_id)
    .bind("doc-scope-coll-a")
    .bind("User A's collection")
    .bind(USER_A_ID)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO collections (id, name, description, user_id, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(coll_b_id)
    .bind("doc-scope-coll-b")
    .bind("User B's collection")
    .bind(USER_B_ID)
    .execute(&pool)
    .await?;

    // Create document in User A's collection
    sqlx::query(
        "INSERT INTO documents (id, name, file_type, file_size, collection_id, user_id, is_active, uploaded_at) VALUES ($1, $2, $3, $4, $5, $6, true, NOW())",
    )
    .bind(doc_a_id)
    .bind("test-doc.md")
    .bind("text/markdown")
    .bind(1024i64)
    .bind(coll_a_id)
    .bind(USER_A_ID)
    .execute(&pool)
    .await?;

    let repo = DocumentRepository::new(pool.clone());

    // User A should see documents in their collection
    let docs_a = repo
        .list_documents_for_user(coll_a_id, USER_A_ID, false)
        .await
        .unwrap();
    assert_eq!(docs_a.len(), 1, "User A should see 1 document");
    assert_eq!(docs_a[0].id, doc_a_id);

    // User B should NOT see documents in User A's collection
    let docs_b_in_a = repo
        .list_documents_for_user(coll_a_id, USER_B_ID, false)
        .await
        .unwrap();
    assert_eq!(
        docs_b_in_a.len(),
        0,
        "User B should NOT see User A's documents"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Session multi-tenancy tests
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_sessions_are_scoped_by_user(pool: PgPool) -> sqlx::Result<()> {
    truncate_all(&pool).await;

    let session_a_id = Uuid::new_v4();
    let session_b_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO sessions (id, title, user_id, created_at, updated_at) VALUES ($1, $2, $3, NOW(), NOW())",
    )
    .bind(session_a_id)
    .bind("User A's session")
    .bind(USER_A_ID)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO sessions (id, title, user_id, created_at, updated_at) VALUES ($1, $2, $3, NOW(), NOW())",
    )
    .bind(session_b_id)
    .bind("User B's session")
    .bind(USER_B_ID)
    .execute(&pool)
    .await?;

    let repo = ConversationRepository::new(pool.clone());

    // User A should see only their sessions
    let sessions_a = repo.list_sessions_by_user(USER_A_ID, false).await.unwrap();
    assert_eq!(sessions_a.len(), 1, "User A should see 1 session");
    assert_eq!(sessions_a[0].id, session_a_id);

    // User B should see only their sessions
    let sessions_b = repo.list_sessions_by_user(USER_B_ID, false).await.unwrap();
    assert_eq!(sessions_b.len(), 1, "User B should see 1 session");
    assert_eq!(sessions_b[0].id, session_b_id);

    // Verify ownership check — User B should get NotFound for User A's session
    let repo = ConversationRepository::new(pool.clone());
    let result = repo
        .get_session_for_user(session_a_id, USER_B_ID, false)
        .await;
    assert!(result.is_err(), "User B should NOT find User A's session");

    Ok(())
}

#[sqlx::test]
async fn test_session_messages_are_scoped_via_session_ownership(pool: PgPool) -> sqlx::Result<()> {
    truncate_all(&pool).await;

    let session_a_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();

    // Create session owned by User A
    sqlx::query(
        "INSERT INTO sessions (id, title, user_id, created_at, updated_at) VALUES ($1, $2, $3, NOW(), NOW())",
    )
    .bind(session_a_id)
    .bind("Session for message scoping")
    .bind(USER_A_ID)
    .execute(&pool)
    .await?;

    // Create a message in that session
    sqlx::query(
        "INSERT INTO messages (id, session_id, role, content, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(message_id)
    .bind(session_a_id)
    .bind("user")
    .bind("Test message")
    .execute(&pool)
    .await?;

    let repo = ConversationRepository::new(pool.clone());

    // User A should see the message (ownership verified via session check)
    let _session = repo
        .get_session_for_user(session_a_id, USER_A_ID, false)
        .await
        .unwrap();
    let msgs_a = repo.get_messages(session_a_id).await.unwrap();
    assert_eq!(msgs_a.len(), 1, "User A should see 1 message");

    // User B should not find the session (ownership check)
    let result = repo
        .get_session_for_user(session_a_id, USER_B_ID, false)
        .await;
    assert!(result.is_err(), "User B should NOT find User A's session");

    Ok(())
}

// ---------------------------------------------------------------------------
// PostgreSQL UUID round-trip tests
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_uuid_text_round_trip_through_collections_row_dto(pool: PgPool) -> sqlx::Result<()> {
    truncate_all(&pool).await;

    let coll_id = Uuid::new_v4();
    let coll_name = format!("uuid-roundtrip-{}", Uuid::new_v4());

    // Insert with TEXT user_id
    sqlx::query(
        "INSERT INTO collections (id, name, description, user_id, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(coll_id)
    .bind(&coll_name)
    .bind("UUID round-trip test")
    .bind(USER_A_ID)
    .execute(&pool)
    .await?;

    // Read back and verify user_id round-trips correctly
    let row: (Uuid, String, Option<String>, String) =
        sqlx::query_as("SELECT id, name, description, user_id FROM collections WHERE id = $1")
            .bind(coll_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(row.0, coll_id, "UUID id should round-trip");
    assert_eq!(row.1, coll_name, "name should match");
    assert_eq!(
        row.3, USER_A_ID,
        "user_id VARCHAR should round-trip as text"
    );

    Ok(())
}

#[sqlx::test]
async fn test_uuid_text_round_trip_through_sessions_row_dto(pool: PgPool) -> sqlx::Result<()> {
    truncate_all(&pool).await;

    let session_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO sessions (id, title, user_id, created_at, updated_at) VALUES ($1, $2, $3, NOW(), NOW())",
    )
    .bind(session_id)
    .bind("UUID round-trip session")
    .bind(USER_B_ID)
    .execute(&pool)
    .await?;

    let row: (Uuid, String, String) =
        sqlx::query_as("SELECT id, title, user_id FROM sessions WHERE id = $1")
            .bind(session_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(row.0, session_id, "UUID id should round-trip");
    assert_eq!(row.1, "UUID round-trip session");
    assert_eq!(
        row.2, USER_B_ID,
        "user_id VARCHAR should round-trip as text"
    );

    Ok(())
}

#[sqlx::test]
async fn test_uuid_text_round_trip_through_documents_row_dto(pool: PgPool) -> sqlx::Result<()> {
    truncate_all(&pool).await;

    let coll_id = Uuid::new_v4();
    let doc_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO collections (id, name, description, user_id, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(coll_id)
    .bind("doc-rt-coll")
    .bind("Parent collection for document round-trip")
    .bind(USER_A_ID)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO documents (id, name, file_type, file_size, collection_id, user_id, is_active, uploaded_at) VALUES ($1, $2, $3, $4, $5, $6, true, NOW())",
    )
    .bind(doc_id)
    .bind("roundtrip-test.md")
    .bind("text/markdown")
    .bind(2048i64)
    .bind(coll_id)
    .bind(USER_A_ID)
    .execute(&pool)
    .await?;

    let row: (Uuid, String, String) =
        sqlx::query_as("SELECT id, name, user_id FROM documents WHERE id = $1")
            .bind(doc_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(row.0, doc_id, "UUID id should round-trip");
    assert_eq!(row.1, "roundtrip-test.md");
    assert_eq!(row.2, USER_A_ID, "user_id VARCHAR should round-trip");

    Ok(())
}
