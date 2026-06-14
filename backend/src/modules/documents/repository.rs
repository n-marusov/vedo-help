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
        sqlx::query("INSERT INTO chunks (id, document_id, index, text) VALUES (?, ?, ?, ?)")
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
            "SELECT id, document_id, index, text FROM chunks WHERE document_id = ? ORDER BY index",
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
}
