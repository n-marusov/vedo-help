use sqlx::SqlitePool;
use uuid::Uuid;

use crate::modules::documents::models::{Chunk, Document};
use crate::shared::error::AppError;

/// Repository for document and chunk data access.
#[derive(Clone, Debug)]
pub struct DocumentRepository {
    db: SqlitePool,
}

impl DocumentRepository {
    /// Create a new DocumentRepository with the given database pool.
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Save a document record to SQLite.
    pub async fn save_document(&self, doc: &Document) -> Result<Uuid, AppError> {
        tracing::debug!(
            "Saving document: {doc_name} ({size} bytes)",
            doc_name = doc.name,
            size = doc.file_size
        );

        sqlx::query(
            "INSERT INTO documents (id, name, file_type, file_size, uploaded_at, collection_id)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(doc.id.to_string())
        .bind(&doc.name)
        .bind(&doc.file_type)
        .bind(doc.file_size)
        .bind(doc.uploaded_at.to_rfc3339())
        .bind(doc.collection_id.to_string())
        .execute(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to save document: {e}")))?;

        tracing::info!(
            "Document saved: {id} ({name}, {size} bytes)",
            id = doc.id,
            name = doc.name,
            size = doc.file_size
        );

        Ok(doc.id)
    }

    /// Retrieve a document by its ID.
    pub async fn get_document(&self, id: Uuid) -> Result<Document, AppError> {
        tracing::debug!("Fetching document: {id}");

        let row = sqlx::query_as::<_, (String, String, String, i64, String, String)>(
            "SELECT id, name, file_type, file_size, uploaded_at, collection_id FROM documents WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?;

        Ok(Document {
            id: Uuid::parse_str(&row.0).unwrap_or(id),
            name: row.1,
            file_type: row.2,
            file_size: row.3,
            uploaded_at: row.4.parse().unwrap_or_else(|_| chrono::Utc::now()),
            collection_id: Uuid::parse_str(&row.5).unwrap_or_default(),
        })
    }

    /// List documents belonging to a collection.
    pub async fn list_documents(&self, collection_id: Uuid) -> Result<Vec<Document>, AppError> {
        tracing::debug!("Listing documents for collection: {collection_id}");

        let rows = sqlx::query_as::<_, (String, String, String, i64, String, String)>(
            "SELECT id, name, file_type, file_size, uploaded_at, collection_id FROM documents WHERE collection_id = ?",
        )
        .bind(collection_id.to_string())
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let documents: Vec<Document> = rows
            .into_iter()
            .map(|row| Document {
                id: Uuid::parse_str(&row.0).unwrap_or_default(),
                name: row.1,
                file_type: row.2,
                file_size: row.3,
                uploaded_at: row.4.parse().unwrap_or_else(|_| chrono::Utc::now()),
                collection_id: Uuid::parse_str(&row.5).unwrap_or(collection_id),
            })
            .collect();

        tracing::debug!(
            "Found {} documents in collection {collection_id}",
            documents.len()
        );

        Ok(documents)
    }

    /// Delete a document and its associated chunks.
    pub async fn delete_document(&self, id: Uuid) -> Result<(), AppError> {
        tracing::debug!("Deleting document: {id}");

        // Delete chunks first (explicit cascade for clarity)
        sqlx::query("DELETE FROM chunks WHERE document_id = ?")
            .bind(id.to_string())
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete chunks: {e}")))?;

        // Delete the document
        let affected = sqlx::query("DELETE FROM documents WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete document: {e}")))?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Document {id} not found")));
        }

        tracing::info!("Document deleted: {id}");

        Ok(())
    }

    /// Save a chunk record to SQLite.
    pub async fn save_chunk(&self, chunk: &Chunk) -> Result<(), AppError> {
        sqlx::query(r#"INSERT INTO chunks (id, document_id, "index", text) VALUES (?, ?, ?, ?)"#)
            .bind(chunk.id.to_string())
            .bind(chunk.document_id.to_string())
            .bind(chunk.index as i64)
            .bind(&chunk.text)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to save chunk: {e}")))?;

        Ok(())
    }

    /// Retrieve chunks by document ID, ordered by index.
    pub async fn get_chunks(&self, document_id: Uuid) -> Result<Vec<Chunk>, AppError> {
        let rows = sqlx::query_as::<_, (String, String, i64, String)>(
            r#"SELECT id, document_id, "index", text FROM chunks WHERE document_id = ? ORDER BY "index""#
        )
        .bind(document_id.to_string())
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let chunks = rows
            .into_iter()
            .map(|row| Chunk {
                id: Uuid::parse_str(&row.0).unwrap_or_default(),
                document_id: Uuid::parse_str(&row.1).unwrap_or(document_id),
                index: row.2 as usize,
                text: row.3,
            })
            .collect();

        Ok(chunks)
    }

    /// Expose the database pool for test assertions.
    #[cfg(test)]
    pub fn db_pool(&self) -> &SqlitePool {
        &self.db
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// Create an in-memory test database with the full schema.
    async fn setup_test_db() -> SqlitePool {
        let pool = sqlx::SqlitePool::connect("sqlite:file::memory:?cache=shared")
            .await
            .expect("Failed to create in-memory SQLite pool");

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .ok();

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
        .ok();

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
        .ok();

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS chunks (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                "index" INTEGER NOT NULL,
                text TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1
            )"#,
        )
        .execute(&pool)
        .await
        .ok();

        pool
    }

    fn test_doc(id: &str, collection_id: &str) -> Document {
        Document {
            id: Uuid::parse_str(id).unwrap(),
            name: format!("doc-{id}"),
            file_type: "text/markdown".to_string(),
            file_size: 100,
            uploaded_at: chrono::Utc::now(),
            collection_id: Uuid::parse_str(collection_id).unwrap(),
        }
    }

    fn test_chunk(id: &str, document_id: &str, index: usize) -> Chunk {
        Chunk {
            id: Uuid::parse_str(id).unwrap(),
            document_id: Uuid::parse_str(document_id).unwrap(),
            index,
            text: format!("chunk {id} text"),
        }
    }

    #[tokio::test]
    async fn test_deactivate_chunks_sets_all_matching_chunks_inactive() {
        let pool = setup_test_db().await;
        let repo = DocumentRepository::new(pool);
        let doc_id = "00000000-0000-0000-0000-000000000001";
        let col_id = "00000000-0000-0000-0000-000000000010";

        // Insert a document
        let doc = test_doc(doc_id, col_id);
        repo.save_document(&doc)
            .await
            .expect("should save document");

        // Insert two chunks for this document
        let chunk1 = test_chunk("00000000-0000-0000-0000-000000000011", doc_id, 0);
        let chunk2 = test_chunk("00000000-0000-0000-0000-000000000012", doc_id, 1);
        repo.save_chunk(&chunk1).await.expect("should save chunk1");
        repo.save_chunk(&chunk2).await.expect("should save chunk2");

        // Act: deactivate all chunks for this document
        repo.deactivate_chunks(Uuid::parse_str(doc_id).unwrap())
            .await
            .expect("should deactivate chunks");

        // Assert: both chunks are now inactive
        let active_chunks = repo
            .get_chunks(Uuid::parse_str(doc_id).unwrap())
            .await
            .expect("should fetch chunks");
        // With the current get_chunks which does not filter by is_active,
        // all chunks are returned. Check is_active via direct query.
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"SELECT id, is_active FROM chunks WHERE document_id = ? ORDER BY "index""#,
        )
        .bind(doc_id)
        .fetch_all(repo.db_pool())
        .await
        .expect("should query chunks");

        assert_eq!(rows.len(), 2, "both chunks should still exist");
        for (chunk_id, is_active) in &rows {
            assert_eq!(
                *is_active, 0,
                "chunk {chunk_id} should be inactive after deactivation"
            );
        }
    }

    #[tokio::test]
    async fn test_deactivate_document_sets_document_inactive_keeps_row() {
        let pool = setup_test_db().await;
        let repo = DocumentRepository::new(pool);
        let doc_id = "00000000-0000-0000-0000-000000000002";
        let col_id = "00000000-0000-0000-0000-000000000010";

        // Insert a document
        let doc = test_doc(doc_id, col_id);
        repo.save_document(&doc)
            .await
            .expect("should save document");

        // Act: deactivate the document
        repo.deactivate_document(Uuid::parse_str(doc_id).unwrap())
            .await
            .expect("should deactivate document");

        // Assert: document row still exists but is inactive
        let is_active: i64 = sqlx::query_scalar("SELECT is_active FROM documents WHERE id = ?")
            .bind(doc_id)
            .fetch_one(repo.db_pool())
            .await
            .expect("should query document");
        assert_eq!(
            is_active, 0,
            "document should be inactive after deactivation"
        );

        // Assert: get_document still returns the document
        let doc = repo
            .get_document(Uuid::parse_str(doc_id).unwrap())
            .await
            .expect("should still retrieve document after deactivation");
        assert_eq!(doc.id.to_string(), doc_id, "document identity preserved");
    }

    #[tokio::test]
    async fn test_deactivation_does_not_affect_other_documents() {
        let pool = setup_test_db().await;
        let repo = DocumentRepository::new(pool);
        let doc_id_1 = "00000000-0000-0000-0000-000000000003";
        let doc_id_2 = "00000000-0000-0000-0000-000000000004";
        let col_id = "00000000-0000-0000-0000-000000000010";

        // Insert two documents
        let doc1 = test_doc(doc_id_1, col_id);
        let doc2 = test_doc(doc_id_2, col_id);
        repo.save_document(&doc1).await.expect("should save doc1");
        repo.save_document(&doc2).await.expect("should save doc2");

        // Insert chunks for both
        let chunk1 = test_chunk("00000000-0000-0000-0000-000000000031", doc_id_1, 0);
        let chunk2 = test_chunk("00000000-0000-0000-0000-000000000032", doc_id_2, 0);
        repo.save_chunk(&chunk1).await.expect("should save chunk1");
        repo.save_chunk(&chunk2).await.expect("should save chunk2");

        // Act: deactivate doc1's chunks
        repo.deactivate_chunks(Uuid::parse_str(doc_id_1).unwrap())
            .await
            .expect("should deactivate chunks for doc1");

        // Assert: doc1's chunk is inactive, doc2's chunk remains active
        let is_active_1: i64 = sqlx::query_scalar("SELECT is_active FROM chunks WHERE id = ?")
            .bind("00000000-0000-0000-0000-000000000031")
            .fetch_one(repo.db_pool())
            .await
            .expect("should query chunk1");
        assert_eq!(is_active_1, 0, "doc1's chunk should be inactive");

        let is_active_2: i64 = sqlx::query_scalar("SELECT is_active FROM chunks WHERE id = ?")
            .bind("00000000-0000-0000-0000-000000000032")
            .fetch_one(repo.db_pool())
            .await
            .expect("should query chunk2");
        assert_eq!(is_active_2, 1, "doc2's chunk should remain active");
    }

    #[tokio::test]
    async fn test_get_active_chunks_returns_only_active_ordered_by_index() {
        let pool = setup_test_db().await;
        let repo = DocumentRepository::new(pool);
        let doc_id = "00000000-0000-0000-0000-000000000005";
        let col_id = "00000000-0000-0000-0000-000000000010";

        // Insert a document
        let doc = test_doc(doc_id, col_id);
        repo.save_document(&doc)
            .await
            .expect("should save document");

        // Insert both active and inactive chunks
        let chunk_active_1 = test_chunk("00000000-0000-0000-0000-000000000051", doc_id, 0);
        let chunk_inactive = test_chunk("00000000-0000-0000-0000-000000000052", doc_id, 1);
        let chunk_active_2 = test_chunk("00000000-0000-0000-0000-000000000053", doc_id, 2);
        repo.save_chunk(&chunk_active_1)
            .await
            .expect("should save active chunk 1");
        repo.save_chunk(&chunk_inactive)
            .await
            .expect("should save inactive chunk");
        repo.save_chunk(&chunk_active_2)
            .await
            .expect("should save active chunk 2");

        // Manually mark the middle chunk as inactive (simulating deactivation)
        sqlx::query("UPDATE chunks SET is_active = 0 WHERE id = ?")
            .bind("00000000-0000-0000-0000-000000000052")
            .execute(repo.db_pool())
            .await
            .expect("should deactivate middle chunk");

        // Act: fetch only active chunks
        let active_chunks = repo
            .get_active_chunks(Uuid::parse_str(doc_id).unwrap())
            .await
            .expect("should fetch active chunks");

        // Assert: only 2 active chunks, ordered by index
        assert_eq!(active_chunks.len(), 2, "should return only active chunks");
        assert_eq!(
            active_chunks[0].index, 0,
            "first active chunk should have index 0"
        );
        assert_eq!(
            active_chunks[0].id.to_string(),
            "00000000-0000-0000-0000-000000000051",
            "first active chunk should be chunk 1"
        );
        assert_eq!(
            active_chunks[1].index, 2,
            "second active chunk should have index 2"
        );
        assert_eq!(
            active_chunks[1].id.to_string(),
            "00000000-0000-0000-0000-000000000053",
            "second active chunk should be chunk 3"
        );
    }

    #[tokio::test]
    async fn test_get_active_chunks_returns_empty_when_all_inactive() {
        let pool = setup_test_db().await;
        let repo = DocumentRepository::new(pool);
        let doc_id = "00000000-0000-0000-0000-000000000006";
        let col_id = "00000000-0000-0000-0000-000000000010";

        // Insert a document
        let doc = test_doc(doc_id, col_id);
        repo.save_document(&doc)
            .await
            .expect("should save document");

        // Insert a chunk then deactivate it
        let chunk = test_chunk("00000000-0000-0000-0000-000000000061", doc_id, 0);
        repo.save_chunk(&chunk).await.expect("should save chunk");
        sqlx::query("UPDATE chunks SET is_active = 0 WHERE id = ?")
            .bind("00000000-0000-0000-0000-000000000061")
            .execute(repo.db_pool())
            .await
            .expect("should deactivate chunk");

        // Act
        let active_chunks = repo
            .get_active_chunks(Uuid::parse_str(doc_id).unwrap())
            .await
            .expect("should fetch active chunks");

        // Assert
        assert!(
            active_chunks.is_empty(),
            "should return empty vec when all chunks are inactive"
        );
    }
}
