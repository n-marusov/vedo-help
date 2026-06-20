use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::modules::documents::models::{Chunk, Document};
use crate::shared::error::AppError;

#[derive(Debug, sqlx::FromRow)]
struct DocumentRow {
    id: uuid::Uuid,
    name: String,
    file_type: String,
    file_size: i64,
    uploaded_at: chrono::DateTime<chrono::Utc>,
    collection_id: uuid::Uuid,
    is_active: bool,
}

impl TryFrom<DocumentRow> for Document {
    type Error = AppError;

    fn try_from(row: DocumentRow) -> Result<Self, Self::Error> {
        Ok(Document {
            id: row.id,
            name: row.name,
            file_type: row.file_type,
            file_size: row.file_size,
            uploaded_at: row.uploaded_at,
            collection_id: row.collection_id,
            is_active: row.is_active,
        })
    }
}

/// Repository for document and chunk data access.
#[derive(Clone, Debug)]
pub struct DocumentRepository {
    db: PgPool,
}

impl DocumentRepository {
    /// Create a new DocumentRepository with the given database pool.
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Save a document record to PostgreSQL.
    pub async fn save_document(&self, doc: &Document) -> Result<Uuid, AppError> {
        tracing::debug!(
            "Saving document: {doc_name} ({size} bytes)",
            doc_name = doc.name,
            size = doc.file_size
        );

        sqlx::query(
            "INSERT INTO documents (id, name, file_type, file_size, uploaded_at, collection_id, is_active)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(doc.id)
        .bind(&doc.name)
        .bind(&doc.file_type)
        .bind(doc.file_size)
        .bind(doc.uploaded_at)
        .bind(doc.collection_id)
        .bind(doc.is_active)
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

        let row = sqlx::query_as::<_, (uuid::Uuid, String, String, i64, chrono::DateTime<chrono::Utc>, uuid::Uuid, bool)>(
            "SELECT id, name, file_type, file_size, uploaded_at, collection_id, is_active FROM documents WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?;

        Ok(Document {
            id: row.0,
            name: row.1,
            file_type: row.2,
            file_size: row.3,
            uploaded_at: row.4,
            collection_id: row.5,
            is_active: row.6,
        })
    }

    /// List documents belonging to a collection.
    pub async fn list_documents(&self, collection_id: Uuid) -> Result<Vec<Document>, AppError> {
        tracing::debug!("Listing documents for collection: {collection_id}");

        let rows = sqlx::query_as::<_, (uuid::Uuid, String, String, i64, chrono::DateTime<chrono::Utc>, uuid::Uuid, bool)>(
            "SELECT id, name, file_type, file_size, uploaded_at, collection_id, is_active FROM documents WHERE collection_id = $1 AND is_active = TRUE",
        )
        .bind(collection_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let documents: Vec<Document> = rows
            .into_iter()
            .map(|row| Document {
                id: row.0,
                name: row.1,
                file_type: row.2,
                file_size: row.3,
                uploaded_at: row.4,
                collection_id: row.5,
                is_active: row.6,
            })
            .collect();

        tracing::debug!(
            "Found {} documents in collection {collection_id}",
            documents.len()
        );

        Ok(documents)
    }

    /// Retrieve documents by their IDs.
    pub async fn get_documents_by_ids(&self, ids: &[Uuid]) -> Result<Vec<Document>, AppError> {
        if ids.is_empty() {
            tracing::debug!("[DocumentRepository.get_documents_by_ids] no ids supplied");
            return Ok(Vec::new());
        }

        tracing::debug!(
            "[DocumentRepository.get_documents_by_ids] fetching documents: count={count}",
            count = ids.len()
        );

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT id, name, file_type, file_size, uploaded_at, collection_id, is_active FROM documents WHERE id IN ("
        );
        let mut separated = query_builder.separated(", ");
        for id in ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");

        let rows = query_builder
            .build_query_as::<DocumentRow>()
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let documents = rows
            .into_iter()
            .map(Document::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        tracing::info!(
            "[DocumentRepository.get_documents_by_ids] fetched documents: requested={requested}, found={found}",
            requested = ids.len(),
            found = documents.len()
        );

        Ok(documents)
    }

    /// Soft-deactivate all chunks belonging to any of the supplied documents.
    pub async fn deactivate_chunks_batch(&self, document_ids: &[Uuid]) -> Result<u64, AppError> {
        if document_ids.is_empty() {
            tracing::debug!(
                "[DocumentRepository.deactivate_chunks_batch] no document ids supplied"
            );
            return Ok(0);
        }

        tracing::debug!(
            "[DocumentRepository.deactivate_chunks_batch] deactivating chunks: document_count={count}",
            count = document_ids.len()
        );

        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE chunks SET is_active = FALSE WHERE document_id IN (");
        let mut separated = query_builder.separated(", ");
        for id in document_ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");

        let affected =
            query_builder.build().execute(&self.db).await.map_err(|e| {
                AppError::InternalError(format!("Failed to deactivate chunks: {e}"))
            })?;
        let count = affected.rows_affected();

        if count == 0 {
            tracing::warn!(
                "[DocumentRepository.deactivate_chunks_batch] no chunks were deactivated: document_count={requested}",
                requested = document_ids.len()
            );
        } else {
            tracing::info!(
                "[DocumentRepository.deactivate_chunks_batch] deactivated chunks: affected={count}, document_count={requested}",
                requested = document_ids.len()
            );
        }

        Ok(count)
    }

    /// Soft-deactivate all supplied documents.
    pub async fn deactivate_documents_batch(&self, ids: &[Uuid]) -> Result<u64, AppError> {
        if ids.is_empty() {
            tracing::debug!("[DocumentRepository.deactivate_documents_batch] no ids supplied");
            return Ok(0);
        }

        tracing::debug!(
            "[DocumentRepository.deactivate_documents_batch] deactivating documents: count={count}",
            count = ids.len()
        );

        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE documents SET is_active = FALSE WHERE id IN (");
        let mut separated = query_builder.separated(", ");
        for id in ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");

        let affected =
            query_builder.build().execute(&self.db).await.map_err(|e| {
                AppError::InternalError(format!("Failed to deactivate documents: {e}"))
            })?;
        let count = affected.rows_affected();

        if count == 0 {
            tracing::warn!(
                "[DocumentRepository.deactivate_documents_batch] no documents were deactivated: requested={requested}",
                requested = ids.len()
            );
        } else {
            tracing::info!(
                "[DocumentRepository.deactivate_documents_batch] deactivated documents: affected={count}, requested={requested}",
                requested = ids.len()
            );
        }

        Ok(count)
    }

    /// Delete a document and its associated chunks.
    pub async fn delete_document(&self, id: Uuid) -> Result<(), AppError> {
        tracing::debug!("Deleting document: {id}");

        // Delete chunks first (explicit cascade for clarity)
        sqlx::query("DELETE FROM chunks WHERE document_id = $1")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete chunks: {e}")))?;

        // Delete the document
        let affected = sqlx::query("DELETE FROM documents WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete document: {e}")))?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Document {id} not found")));
        }

        tracing::info!("Document deleted: {id}");

        Ok(())
    }

    /// Save a chunk record to PostgreSQL.
    pub async fn save_chunk(&self, chunk: &Chunk) -> Result<(), AppError> {
        sqlx::query(r#"INSERT INTO chunks (id, document_id, "index", text, is_active) VALUES ($1, $2, $3, $4, $5)"#)
            .bind(chunk.id)
            .bind(chunk.document_id)
            .bind(chunk.index as i32)
            .bind(&chunk.text)
            .bind(chunk.is_active)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to save chunk: {e}")))?;

        Ok(())
    }

    /// Retrieve chunks by document ID, ordered by index.
    pub async fn get_chunks(&self, document_id: Uuid) -> Result<Vec<Chunk>, AppError> {
        let rows = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, i32, String, bool)>(
            r#"SELECT id, document_id, "index", text, is_active FROM chunks WHERE document_id = $1 ORDER BY "index""#
        )
        .bind(document_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let chunks = rows
            .into_iter()
            .map(|row| Chunk {
                id: row.0,
                document_id: row.1,
                index: row.2 as usize,
                text: row.3,
                is_active: row.4,
            })
            .collect();

        Ok(chunks)
    }

    /// Deactivate all chunks belonging to a document.
    /// Sets `is_active = FALSE` for all matching chunks (soft delete).
    pub async fn deactivate_chunks(&self, document_id: Uuid) -> Result<(), AppError> {
        tracing::debug!("Deactivating chunks for document: {document_id}");

        let affected = sqlx::query("UPDATE chunks SET is_active = FALSE WHERE document_id = $1")
            .bind(document_id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to deactivate chunks: {e}")))?;

        let count = affected.rows_affected();
        tracing::debug!("Deactivated {count} chunks for document {document_id}");

        Ok(())
    }

    /// Deactivate a document (soft delete) without removing the row.
    pub async fn deactivate_document(&self, id: Uuid) -> Result<(), AppError> {
        tracing::debug!("Deactivating document: {id}");

        let affected = sqlx::query("UPDATE documents SET is_active = FALSE WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to deactivate document: {e}")))?;

        if affected.rows_affected() == 0 {
            tracing::warn!("Deactivation target not found: document {id}");
            return Err(AppError::NotFound(format!("Document {id} not found")));
        }

        tracing::info!("Document deactivated: {id}");

        Ok(())
    }

    /// Deactivate specific chunks by ID.
    pub async fn deactivate_chunks_by_ids(&self, chunk_ids: &[Uuid]) -> Result<(), AppError> {
        if chunk_ids.is_empty() {
            tracing::debug!("No chunk ids supplied for targeted deactivation");
            return Ok(());
        }

        tracing::debug!(
            "Deactivating {count} targeted chunks",
            count = chunk_ids.len()
        );

        let mut affected_total = 0u64;
        for chunk_id in chunk_ids {
            let affected = sqlx::query("UPDATE chunks SET is_active = FALSE WHERE id = $1")
                .bind(chunk_id)
                .execute(&self.db)
                .await
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to deactivate chunk {chunk_id}: {e}"))
                })?;
            affected_total += affected.rows_affected();
        }

        tracing::debug!(
            "Deactivated {affected_total} targeted chunks out of {requested}",
            requested = chunk_ids.len()
        );

        Ok(())
    }

    /// Update document metadata after a successful reload.
    pub async fn update_document_metadata(
        &self,
        id: Uuid,
        name: &str,
        file_type: &str,
        file_size: i64,
    ) -> Result<(), AppError> {
        tracing::debug!(
            "Updating document metadata: {id}, name={name}, type={file_type}, size={file_size}"
        );

        let affected = sqlx::query(
            "UPDATE documents SET name = $1, file_type = $2, file_size = $3, uploaded_at = $4, is_active = TRUE WHERE id = $5",
        )
        .bind(name)
        .bind(file_type)
        .bind(file_size)
        .bind(chrono::Utc::now())
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to update document metadata: {e}")))?;

        if affected.rows_affected() == 0 {
            tracing::warn!("Metadata update target not found: document {id}");
            return Err(AppError::NotFound(format!("Document {id} not found")));
        }

        tracing::info!("Document metadata updated: {id}");
        Ok(())
    }
    /// Retrieve only active chunks for a document, ordered by index.
    pub async fn get_active_chunks(&self, document_id: Uuid) -> Result<Vec<Chunk>, AppError> {
        tracing::debug!("Fetching active chunks for document: {document_id}");

        let rows = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, i32, String, bool)>(
            r#"SELECT id, document_id, "index", text, is_active FROM chunks
               WHERE document_id = $1 AND is_active = TRUE
               ORDER BY "index""#,
        )
        .bind(document_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let chunks: Vec<Chunk> = rows
            .into_iter()
            .map(|row| Chunk {
                id: row.0,
                document_id: row.1,
                index: row.2 as usize,
                text: row.3,
                is_active: row.4,
            })
            .collect();

        tracing::debug!(
            "Found {} active chunks for document {document_id}",
            chunks.len()
        );

        Ok(chunks)
    }

    /// Expose the database pool for test assertions.
    #[cfg(test)]
    pub fn db_pool(&self) -> &PgPool {
        &self.db
    }
}

#[cfg(test)]
mod tests {
    // Tests migrated to sqlx::test with PostgreSQL fixtures (Phase 3)
}
