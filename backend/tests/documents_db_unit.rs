/// Unit / DB round-trip tests for the DocumentService.
///
/// These tests require a PostgreSQL test database; they do NOT require
/// Chroma, embedding, or any other external service.
///
/// Run:
/// ```bash
/// cargo test --test documents_db_unit -- --test-threads=1
/// ```
mod common;

use uuid::Uuid;

use vedo_backend::modules::collections::repository::CollectionRepository;
use vedo_backend::modules::documents::repository::DocumentRepository;
use vedo_backend::modules::documents::service::DocumentService;
use vedo_backend::shared::AppError;

/// Helper: create an in-memory ZIP with given (filename, content) pairs.
fn make_zip(files: &[(&str, &str)]) -> Vec<u8> {
    use std::io::Write;
    let buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buf);
    for &(name, content) in files {
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file(name, options).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

/// Test: process_upload with Cyrillic text must not panic
/// on debug preview slicing.
#[tokio::test]
async fn test_process_upload_non_ascii_text() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let cyrillic_content = "Привет, мир! Это тестовый文档 с кириллицей.\n\n".repeat(50);
    let content = format!(
        "{cyrillic_content}\n\n\
         Дополнительный параграф для проверки корректной обработки многобайтовых символов на границе чанков.\n\n\
         И ещё один параграф с русским текстом для верности."
    );
    let data = content.as_bytes();
    let filename = "test-cyrillic.md";
    let collection_id = Uuid::new_v4();

    // Insert parent collection to satisfy FK constraint
    sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
        .bind(collection_id)
        .bind(format!("test-collection-non-ascii-{collection_id}"))
        .bind("")
        .execute(&pool)
        .await
        .expect("Failed to insert collection");

    let result = svc
        .process_upload(
            data,
            filename,
            collection_id,
            "text/markdown".to_string(),
            "test-user",
            true,
        )
        .await;

    assert!(result.is_ok(), "process_upload failed: {:?}", result.err());
    let response = result.unwrap();
    assert!(response.chunks_indexed > 0, "Expected at least 1 chunk");
    assert_eq!(response.document_name, filename);

    // Verify document appears in list
    let documents = svc
        .list_documents(collection_id, "test-user", true)
        .await
        .unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].name, filename);
}

/// Test: process_upload with 4-byte UTF-8 (emoji) must not panic.
#[tokio::test]
async fn test_process_upload_emoji_content() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let emoji_line = "😀🚀🌈🧪🔥🎉🎊🎈🎁\n".repeat(30);
    let content = format!("{emoji_line}\nMore emoji text 🎯🎲🎮🕹️🎰.\n");
    let data = content.as_bytes();
    let filename = "test-emoji.md";
    let collection_id = Uuid::new_v4();

    // Insert parent collection to satisfy FK constraint
    sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
        .bind(collection_id)
        .bind(format!("test-collection-emoji-{collection_id}"))
        .bind("")
        .execute(&pool)
        .await
        .expect("Failed to insert collection");

    let result = svc
        .process_upload(
            data,
            filename,
            collection_id,
            "text/markdown".to_string(),
            "test-user",
            true,
        )
        .await;

    assert!(
        result.is_ok(),
        "process_upload with emoji failed: {:?}",
        result.err()
    );
    let response = result.unwrap();
    assert!(response.chunks_indexed > 0);
    assert_eq!(response.document_name, filename);
}

/// Test: process_upload with mixed CJK + Cyrillic + emoji must not panic.
#[tokio::test]
async fn test_process_upload_mixed_encoding() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let mixed = "English text. Привет-мир-你好世界😀🚀\n".repeat(40);
    let content = format!("{mixed}\n\nEND");
    let data = content.as_bytes();
    let filename = "test-mixed.md";
    let collection_id = Uuid::new_v4();

    // Insert parent collection to satisfy FK constraint
    sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
        .bind(collection_id)
        .bind(format!("test-collection-mixed-{collection_id}"))
        .bind("")
        .execute(&pool)
        .await
        .expect("Failed to insert collection");

    let result = svc
        .process_upload(
            data,
            filename,
            collection_id,
            "text/markdown".to_string(),
            "test-user",
            true,
        )
        .await;

    assert!(
        result.is_ok(),
        "process_upload with mixed encoding failed: {:?}",
        result.err()
    );
    let response = result.unwrap();
    assert!(response.chunks_indexed > 0);
    assert_eq!(response.document_name, filename);
}

/// Test: ASCII-only upload must still work after UTF-8 fixes.
#[tokio::test]
async fn test_process_upload_ascii_regression() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let content =
        "Hello, world!\n\nThis is a test document with ASCII text only.\n\nParagraph three.\n";
    let data = content.as_bytes();
    let filename = "test-ascii.md";
    let collection_id = Uuid::new_v4();

    // Insert parent collection to satisfy FK constraint
    sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
        .bind(collection_id)
        .bind(format!("test-collection-ascii-{collection_id}"))
        .bind("")
        .execute(&pool)
        .await
        .expect("Failed to insert collection");

    let result = svc
        .process_upload(
            data,
            filename,
            collection_id,
            "text/markdown".to_string(),
            "test-user",
            true,
        )
        .await;

    assert!(
        result.is_ok(),
        "process_upload with ASCII failed: {:?}",
        result.err()
    );
    let response = result.unwrap();
    assert_eq!(response.document_name, filename);
}

/// Test: process_zip_upload with 5 valid .md files succeeds.
#[tokio::test]
async fn test_process_zip_with_5_md_files() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let data = make_zip(&[
        ("file1.md", "# File 1"),
        ("file2.md", "# File 2"),
        ("file3.md", "# File 3"),
        ("file4.md", "# File 4"),
        ("file5.md", "# File 5"),
    ]);
    let collection_id = Uuid::new_v4();

    // Insert parent collection to satisfy FK constraint
    sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
        .bind(collection_id)
        .bind(format!("test-collection-zip-5-{collection_id}"))
        .bind("")
        .execute(&pool)
        .await
        .expect("Failed to insert collection");

    let result = svc
        .process_zip_upload(&data, collection_id, "test-user", true)
        .await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.processed, 5);
    assert_eq!(response.failed, 0);
    assert_eq!(response.total_files, 5);
}

/// Test: process_zip_upload with 11 files returns PayloadTooLarge.
#[tokio::test]
async fn test_process_zip_with_11_files_returns_413() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let names: Vec<String> = (0..11).map(|i| format!("doc-{i}.md")).collect();
    let refs: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "# content")).collect();
    let data = make_zip(&refs);
    let result = svc
        .process_zip_upload(&data, Uuid::new_v4(), "test-user", true)
        .await;
    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::PayloadTooLarge(_) => {}
        _ => panic!("Expected PayloadTooLarge error"),
    }
}

/// Test: process_zip_upload with mixed valid and invalid files.
#[tokio::test]
async fn test_process_zip_mixed_valid_invalid() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let data = make_zip(&[
        ("valid.md", "# Valid"),
        ("script.exe", "fake exe"),
        ("notes.txt", "Plain text"),
    ]);
    let collection_id = Uuid::new_v4();

    // Insert parent collection to satisfy FK constraint
    sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
        .bind(collection_id)
        .bind(format!("test-collection-zip-mixed-{collection_id}"))
        .bind("")
        .execute(&pool)
        .await
        .expect("Failed to insert collection");

    let result = svc
        .process_zip_upload(&data, collection_id, "test-user", true)
        .await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.processed > 0);
    assert!(response.processed < response.total_files);
}

/// Test: process_zip_upload with an empty ZIP succeeds (0 files).
#[tokio::test]
async fn test_process_zip_empty() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let data = make_zip(&[]);
    let result = svc
        .process_zip_upload(&data, Uuid::new_v4(), "test-user", true)
        .await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.total_files, 0);
    assert_eq!(response.processed, 0);
}

/// Test: process_zip_upload with corrupted data returns FileError.
#[tokio::test]
async fn test_process_zip_corrupted() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let data = vec![0x00, 0x01, 0x02, 0x03];
    let result = svc
        .process_zip_upload(&data, Uuid::new_v4(), "test-user", true)
        .await;
    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::FileError(_) => {}
        _ => panic!("Expected FileError for corrupted ZIP"),
    }
}

/// Test: process_zip_upload with unsupported file types skips them.
#[tokio::test]
async fn test_process_zip_unsupported_types_skipped() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let data = make_zip(&[
        ("valid.md", "# Valid"),
        ("readme.txt", "Plain text"),
        ("app.exe", "binary"),
    ]);
    let collection_id = Uuid::new_v4();

    // Insert parent collection to satisfy FK constraint
    sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
        .bind(collection_id)
        .bind(format!("test-collection-zip-unsupported-{collection_id}"))
        .bind("")
        .execute(&pool)
        .await
        .expect("Failed to insert collection");

    let result = svc
        .process_zip_upload(&data, collection_id, "test-user", true)
        .await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.processed >= 1);
    // .txt and .exe should be skipped, only .md processed
    assert!(response.failed > 0 || response.processed < response.total_files);
}

/// Test: reload_document deactivates old chunks and saves new active chunks.
#[tokio::test]
async fn test_reload_document_deactivates_old_chunks_and_saves_new_active_chunks() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo.clone(), collection_repo);

    let collection_id = Uuid::new_v4();

    // Insert a collection for FK
    sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
        .bind(collection_id)
        .bind(format!("test-collection-{collection_id}"))
        .bind("")
        .execute(&pool)
        .await
        .expect("Failed to insert collection");

    // First upload: create document with initial content
    let initial_content = b"# Initial version\n\nThis is the first version of the document.";
    let upload_result = svc
        .process_upload(
            initial_content,
            "test.md",
            collection_id,
            "text/markdown".into(),
            "test-user",
            true,
        )
        .await
        .expect("first upload should succeed");
    let doc_id = upload_result.document_id;

    // Reload with new content
    let reload_content = b"# Updated version\n\nThis is the reloaded version with different text.";
    svc.reload_document(reload_content, "test.md", doc_id, "test-user", true)
        .await
        .expect("reload should succeed");

    // Assert: old chunks are inactive
    let old_chunks = repo
        .get_chunks(doc_id)
        .await
        .expect("should fetch all chunks");
    assert!(
        !old_chunks.is_empty(),
        "there should be some chunks in the database"
    );

    // Check is_active via direct SQL for all chunks
    let rows: Vec<(Uuid, bool)> = sqlx::query_as(
        r#"SELECT id, is_active FROM chunks WHERE document_id = $1 ORDER BY "index""#,
    )
    .bind(doc_id)
    .fetch_all(&pool)
    .await
    .expect("should query chunks");

    // Count active vs inactive
    let active_count = rows.iter().filter(|(_, active)| *active).count();
    let inactive_count = rows.iter().filter(|(_, active)| !*active).count();

    assert!(
        inactive_count > 0,
        "old chunks should be deactivated (found {inactive_count} inactive)"
    );
    assert!(
        active_count > 0,
        "new chunks should be active (found {active_count} active)"
    );
}

/// Test: soft delete keeps rows but removes from active results.
#[tokio::test]
async fn test_soft_delete_keeps_rows_but_removes_from_active_results() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let collection_id = Uuid::new_v4();

    // Insert a collection for FK
    sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
        .bind(collection_id)
        .bind(format!("test-collection-del-{collection_id}"))
        .bind("")
        .execute(&pool)
        .await
        .expect("Failed to insert collection");

    // Upload a document
    let content = b"# Test document\n\nThis document will be deleted.";
    let upload_result = svc
        .process_upload(
            content,
            "delete-me.md",
            collection_id,
            "text/markdown".into(),
            "test-user",
            true,
        )
        .await
        .expect("upload should succeed");
    let doc_id = upload_result.document_id;

    // Confirm document is visible in active results
    let docs_before = svc
        .list_documents(collection_id, "test-user", true)
        .await
        .expect("should list documents");
    assert!(
        docs_before.iter().any(|d| d.id == doc_id),
        "document should be listed before delete"
    );

    // Delete the document (soft delete)
    svc.delete_document(doc_id, "test-user", true)
        .await
        .expect("soft delete should succeed");

    // Assert: document row still exists in the database
    let doc_row: Option<(Uuid, bool)> =
        sqlx::query_as("SELECT id, is_active FROM documents WHERE id = $1")
            .bind(doc_id)
            .fetch_optional(&pool)
            .await
            .expect("should query document");
    assert!(
        doc_row.is_some(),
        "document row should still exist after soft delete"
    );
    let (_, is_active) = doc_row.unwrap();
    assert!(!is_active, "document should be marked inactive");

    // Assert: chunks remain but inactive
    let chunk_rows: Vec<(Uuid, bool)> =
        sqlx::query_as(r#"SELECT id, is_active FROM chunks WHERE document_id = $1"#)
            .bind(doc_id)
            .fetch_all(&pool)
            .await
            .expect("should query chunks");
    assert!(
        !chunk_rows.is_empty(),
        "chunks should still exist after soft delete"
    );
    for (chunk_id, active) in &chunk_rows {
        assert!(
            !*active,
            "chunk {chunk_id} should be inactive after soft delete"
        );
    }

    // Assert: document does not appear in active listing
    let docs_after = svc
        .list_documents(collection_id, "test-user", true)
        .await
        .expect("should list documents after delete");
    assert!(
        !docs_after.iter().any(|d| d.id == doc_id),
        "document should not appear in active listing after soft delete"
    );
}

/// Test: batch delete keeps rows but removes from active results.
#[tokio::test]
async fn test_batch_delete_keeps_rows_but_removes_from_active_results() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let collection_id = Uuid::new_v4();

    // Insert a collection
    sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
        .bind(collection_id)
        .bind(format!("test-collection-batch-{collection_id}"))
        .bind("")
        .execute(&pool)
        .await
        .expect("Failed to insert collection");

    // Upload 3 documents
    let mut doc_ids = Vec::new();
    for i in 0..3 {
        let content = format!("# Document {i}\n\nThis is document number {i}.");
        let result = svc
            .process_upload(
                content.as_bytes(),
                &format!("doc-{i}.md"),
                collection_id,
                "text/markdown".into(),
                "test-user",
                true,
            )
            .await
            .expect("upload should succeed");
        doc_ids.push(result.document_id);
    }

    // Confirm all 3 are visible
    let docs_before = svc
        .list_documents(collection_id, "test-user", true)
        .await
        .expect("should list documents");
    assert_eq!(docs_before.len(), 3, "all 3 documents should be visible");

    // Delete 2 via batch
    let to_delete = vec![doc_ids[0], doc_ids[1]];
    let batch_result = svc
        .delete_documents_batch(to_delete, "test-user", true)
        .await
        .expect("batch delete should succeed");
    assert_eq!(batch_result.deleted_count, 2);

    // Assert: remaining 1 is visible, 2 are invisible
    let docs_after = svc
        .list_documents(collection_id, "test-user", true)
        .await
        .expect("should list documents after batch delete");
    assert_eq!(docs_after.len(), 1, "only 1 document should remain visible");
    assert!(
        docs_after.iter().any(|d| d.id == doc_ids[2]),
        "the third document should still be visible"
    );

    // Assert: rows still exist but are inactive
    for deleted_id in &[doc_ids[0], doc_ids[1]] {
        let doc_row: Option<(Uuid, bool)> =
            sqlx::query_as("SELECT id, is_active FROM documents WHERE id = $1")
                .bind(*deleted_id)
                .fetch_optional(&pool)
                .await
                .expect("should query document");
        assert!(doc_row.is_some(), "deleted document row should still exist");
        assert!(!doc_row.unwrap().1, "document should be marked inactive");
    }
}

/// Test: batch delete across multiple collections.
#[tokio::test]
async fn test_batch_delete_with_mixed_collections() {
    let pool = common::setup_test_db().await;
    let repo = DocumentRepository::new(pool.clone());
    let collection_repo = CollectionRepository::new(pool.clone());
    let svc = DocumentService::new(repo, collection_repo);

    let collection_a = Uuid::new_v4();
    let collection_b = Uuid::new_v4();

    // Insert both collections
    for col_id in &[collection_a, collection_b] {
        sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
            .bind(*col_id)
            .bind(format!("col-{col_id}"))
            .bind("")
            .execute(&pool)
            .await
            .expect("Failed to insert collection");
    }

    // Upload 2 docs to col A, 2 docs to col B
    let mut col_a_ids = Vec::new();
    let mut col_b_ids = Vec::new();

    for i in 0..2 {
        let content = format!("# Doc A{i}");
        let result = svc
            .process_upload(
                content.as_bytes(),
                &format!("a-{i}.md"),
                collection_a,
                "text/markdown".into(),
                "test-user",
                true,
            )
            .await
            .expect("upload to col A should succeed");
        col_a_ids.push(result.document_id);
    }
    for i in 0..2 {
        let content = format!("# Doc B{i}");
        let result = svc
            .process_upload(
                content.as_bytes(),
                &format!("b-{i}.md"),
                collection_b,
                "text/markdown".into(),
                "test-user",
                true,
            )
            .await
            .expect("upload to col B should succeed");
        col_b_ids.push(result.document_id);
    }

    // Delete 1 doc from col A + 1 doc from col B in one batch
    let to_delete = vec![col_a_ids[0], col_b_ids[0]];
    let result = svc
        .delete_documents_batch(to_delete, "test-user", true)
        .await
        .expect("batch delete across collections should succeed");
    assert_eq!(result.deleted_count, 2);

    // Assert correct per-collection active state
    let docs_a = svc
        .list_documents(collection_a, "test-user", true)
        .await
        .expect("should list col A");
    assert_eq!(docs_a.len(), 1, "col A should have 1 doc remaining");
    assert_eq!(
        docs_a[0].id, col_a_ids[1],
        "col A should keep the second doc"
    );

    let docs_b = svc
        .list_documents(collection_b, "test-user", true)
        .await
        .expect("should list col B");
    assert_eq!(docs_b.len(), 1, "col B should have 1 doc remaining");
    assert_eq!(
        docs_b[0].id, col_b_ids[1],
        "col B should keep the second doc"
    );
}
