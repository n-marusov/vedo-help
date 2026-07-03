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
    user_id: String,
    source: String,
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
            user_id: row.user_id,
            source: row.source,
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
            component = "documents/repository",
            file_name = %doc.name,
            file_size = doc.file_size,
            "document.save.started"
        );

        sqlx::query(
                    "INSERT INTO documents (id, name, file_type, file_size, uploaded_at, collection_id, is_active, user_id, source)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                )
                .bind(doc.id)
                .bind(&doc.name)
                .bind(&doc.file_type)
                .bind(doc.file_size)
                .bind(doc.uploaded_at)
                .bind(doc.collection_id)
                .bind(doc.is_active)
                .bind(&doc.user_id)
                .bind(&doc.source)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to save document: {e}")))?;

        tracing::info!(
            component = "documents/repository",
            document_id = %doc.id,
            file_name = %doc.name,
            file_size = doc.file_size,
            "document.saved"
        );

        Ok(doc.id)
    }

    /// Retrieve a document by its ID.
    pub async fn get_document(&self, id: Uuid) -> Result<Document, AppError> {
        tracing::debug!(component = "documents/repository", document_id = %id, "document.fetch");

        let row = sqlx::query_as::<_, (uuid::Uuid, String, String, i64, chrono::DateTime<chrono::Utc>, uuid::Uuid, bool, String, String)>(
                    "SELECT id, name, file_type, file_size, uploaded_at, collection_id, is_active, user_id, source FROM documents WHERE id = $1",
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
            user_id: row.7,
            source: row.8,
        })
    }

    /// Retrieve a document by its ID with user ownership verification.
    /// Non-admin users can only access their own documents.
    pub async fn get_document_for_user(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Document, AppError> {
        tracing::debug!(component = "documents/repository", document_id = %id, "document.fetch_for_user");

        let row = if is_admin {
            sqlx::query_as::<_, (uuid::Uuid, String, String, i64, chrono::DateTime<chrono::Utc>, uuid::Uuid, bool, String, String)>(
                        "SELECT id, name, file_type, file_size, uploaded_at, collection_id, is_active, user_id, source FROM documents WHERE id = $1",
                    )
                    .bind(id)
                    .fetch_optional(&self.db)
                    .await
                    .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
                    .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?
        } else {
            sqlx::query_as::<_, (uuid::Uuid, String, String, i64, chrono::DateTime<chrono::Utc>, uuid::Uuid, bool, String, String)>(
                        "SELECT id, name, file_type, file_size, uploaded_at, collection_id, is_active, user_id, source FROM documents WHERE id = $1 AND user_id = $2",
                    )
                    .bind(id)
                    .bind(user_id)
                    .fetch_optional(&self.db)
                    .await
                    .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
                    .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?
        };

        Ok(Document {
            id: row.0,
            name: row.1,
            file_type: row.2,
            file_size: row.3,
            uploaded_at: row.4,
            collection_id: row.5,
            is_active: row.6,
            user_id: row.7,
            source: row.8,
        })
    }

    /// List documents belonging to a collection (legacy, no ownership check).
    pub async fn list_documents(&self, collection_id: Uuid) -> Result<Vec<Document>, AppError> {
        tracing::debug!(component = "documents/repository", collection_id = %collection_id, "document.list");

        let rows = sqlx::query_as::<_, (uuid::Uuid, String, String, i64, chrono::DateTime<chrono::Utc>, uuid::Uuid, bool, String, String)>(
                    "SELECT id, name, file_type, file_size, uploaded_at, collection_id, is_active, user_id, source FROM documents WHERE collection_id = $1 AND is_active = TRUE",
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
                user_id: row.7,
                source: row.8,
            })
            .collect();

        tracing::debug!(
            component = "documents/repository",
            collection_id = %collection_id,
            document_count = documents.len(),
            "document.list.found"
        );

        Ok(documents)
    }

    /// List documents belonging to a collection with user ownership filter.
    /// Non-admin users see only their own documents; admin users see all.
    pub async fn list_documents_for_user(
        &self,
        collection_id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<Document>, AppError> {
        tracing::debug!(component = "documents/repository", collection_id = %collection_id, "document.list_for_user");

        let rows = if is_admin {
            sqlx::query_as::<_, (uuid::Uuid, String, String, i64, chrono::DateTime<chrono::Utc>, uuid::Uuid, bool, String, String)>(
                        "SELECT id, name, file_type, file_size, uploaded_at, collection_id, is_active, user_id, source FROM documents WHERE collection_id = $1 AND is_active = TRUE",
                    )
                    .bind(collection_id)
                    .fetch_all(&self.db)
                    .await
                    .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        } else {
            sqlx::query_as::<_, (uuid::Uuid, String, String, i64, chrono::DateTime<chrono::Utc>, uuid::Uuid, bool, String, String)>(
                        "SELECT id, name, file_type, file_size, uploaded_at, collection_id, is_active, user_id, source FROM documents WHERE collection_id = $1 AND is_active = TRUE AND user_id = $2",
                    )
                    .bind(collection_id)
                    .bind(user_id)
                    .fetch_all(&self.db)
                    .await
                    .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        };

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
                user_id: row.7,
                source: row.8,
            })
            .collect();

        tracing::debug!(
            component = "documents/repository",
            collection_id = %collection_id,
            document_count = documents.len(),
            "document.list_for_user.found"
        );

        Ok(documents)
    }

    /// Retrieve documents by their IDs.
    pub async fn get_documents_by_ids(&self, ids: &[Uuid]) -> Result<Vec<Document>, AppError> {
        if ids.is_empty() {
            tracing::debug!(
                component = "documents/repository",
                "documents.get_by_ids.empty"
            );
            return Ok(Vec::new());
        }

        tracing::debug!(
            component = "documents/repository",
            request_count = ids.len(),
            "documents.get_by_ids.start"
        );

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT id, name, file_type, file_size, uploaded_at, collection_id, is_active, user_id, source FROM documents WHERE id IN ("
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
            component = "documents/repository",
            request_count = ids.len(),
            found_count = documents.len(),
            "documents.get_by_ids.found"
        );

        Ok(documents)
    }

    /// Soft-deactivate all chunks belonging to any of the supplied documents.
    pub async fn deactivate_chunks_batch(&self, document_ids: &[Uuid]) -> Result<u64, AppError> {
        if document_ids.is_empty() {
            tracing::debug!(
                component = "documents/repository",
                "chunks.deactivate_batch.empty"
            );
            return Ok(0);
        }

        tracing::debug!(
            component = "documents/repository",
            document_count = document_ids.len(),
            "chunks.deactivate_batch.start"
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
                component = "documents/repository",
                document_count = document_ids.len(),
                "chunks.deactivate_batch.none_found"
            );
        } else {
            tracing::info!(
                component = "documents/repository",
                affected_count = count,
                document_count = document_ids.len(),
                "chunks.deactivate_batch.complete"
            );
        }

        Ok(count)
    }

    /// Soft-deactivate all supplied documents.
    pub async fn deactivate_documents_batch(&self, ids: &[Uuid]) -> Result<u64, AppError> {
        if ids.is_empty() {
            tracing::debug!(
                component = "documents/repository",
                "documents.deactivate_batch.empty"
            );
            return Ok(0);
        }

        tracing::debug!(
            component = "documents/repository",
            request_count = ids.len(),
            "documents.deactivate_batch.start"
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
                component = "documents/repository",
                request_count = ids.len(),
                "documents.deactivate_batch.none_found"
            );
        } else {
            tracing::info!(
                component = "documents/repository",
                affected_count = count,
                request_count = ids.len(),
                "documents.deactivate_batch.complete"
            );
        }

        Ok(count)
    }

    /// Delete a document and its associated chunks.
    pub async fn delete_document(&self, id: Uuid) -> Result<(), AppError> {
        tracing::debug!(component = "documents/repository", document_id = %id, "document.delete.started");

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

        tracing::info!(component = "documents/repository", document_id = %id, "document.deleted");

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
        tracing::debug!(component = "documents/repository", document_id = %document_id, "chunks.deactivate.started");

        let affected = sqlx::query("UPDATE chunks SET is_active = FALSE WHERE document_id = $1")
            .bind(document_id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to deactivate chunks: {e}")))?;

        let count = affected.rows_affected();
        tracing::debug!(
            component = "documents/repository",
            chunk_count = count,
            document_id = %document_id,
            "chunks.deactivated"
        );

        Ok(())
    }

    /// Deactivate a document (soft delete) with ownership verification.
    /// Non-admin users can only deactivate their own documents.
    pub async fn deactivate_document_for_user(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Document, AppError> {
        tracing::debug!(component = "documents/repository", document_id = %id, "document.deactivate_for_user.started");

        if is_admin {
            // Admin: no ownership check
            let doc = self.get_document(id).await?;
            let affected = sqlx::query("UPDATE documents SET is_active = FALSE WHERE id = $1")
                .bind(id)
                .execute(&self.db)
                .await
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to deactivate document: {e}"))
                })?;

            if affected.rows_affected() == 0 {
                return Err(AppError::NotFound(format!("Document {id} not found")));
            }

            tracing::info!(component = "documents/repository", document_id = %id, "document.deactivated");
            Ok(doc)
        } else {
            // Non-admin: verify ownership
            let doc = self.get_document_for_user(id, user_id, false).await?;
            let affected = sqlx::query(
                "UPDATE documents SET is_active = FALSE WHERE id = $1 AND user_id = $2",
            )
            .bind(id)
            .bind(user_id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to deactivate document: {e}")))?;

            if affected.rows_affected() == 0 {
                return Err(AppError::NotFound(format!("Document {id} not found")));
            }

            tracing::info!(component = "documents/repository", document_id = %id, "document.deactivated");
            Ok(doc)
        }
    }

    /// Deactivate a document (soft delete) without removing the row (legacy, no owner check).
    pub async fn deactivate_document(&self, id: Uuid) -> Result<(), AppError> {
        tracing::debug!(component = "documents/repository", document_id = %id, "document.deactivate.started");

        let affected = sqlx::query("UPDATE documents SET is_active = FALSE WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to deactivate document: {e}")))?;

        if affected.rows_affected() == 0 {
            tracing::warn!(component = "documents/repository", document_id = %id, "document.deactivate.not_found");
            return Err(AppError::NotFound(format!("Document {id} not found")));
        }

        tracing::info!(component = "documents/repository", document_id = %id, "document.deactivated");

        Ok(())
    }

    /// Deactivate specific chunks by ID.
    pub async fn deactivate_chunks_by_ids(&self, chunk_ids: &[Uuid]) -> Result<(), AppError> {
        if chunk_ids.is_empty() {
            tracing::debug!(
                component = "documents/repository",
                "chunks.deactivate_by_ids.empty"
            );
            return Ok(());
        }

        tracing::debug!(
            component = "documents/repository",
            request_count = chunk_ids.len(),
            "chunks.deactivate_by_ids.start"
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
            component = "documents/repository",
            affected_count = affected_total,
            request_count = chunk_ids.len(),
            "chunks.deactivate_by_ids.complete"
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
            component = "documents/repository",
            document_id = %id,
            file_name = %name,
            file_type = %file_type,
            file_size = file_size,
            "document.metadata_update.started"
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
            tracing::warn!(component = "documents/repository", document_id = %id, "document.metadata_update.not_found");
            return Err(AppError::NotFound(format!("Document {id} not found")));
        }

        tracing::info!(component = "documents/repository", document_id = %id, "document.metadata_updated");
        Ok(())
    }
    /// Retrieve only active chunks for a document, ordered by index.
    pub async fn get_active_chunks(&self, document_id: Uuid) -> Result<Vec<Chunk>, AppError> {
        tracing::debug!(component = "documents/repository", document_id = %document_id, "chunks.get_active.started");

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
            component = "documents/repository",
            document_id = %document_id,
            chunk_count = chunks.len(),
            "chunks.get_active.found"
        );

        Ok(chunks)
    }

    /// Expose the database pool for test assertions.
    #[cfg(test)]
    pub fn db_pool(&self) -> &PgPool {
        &self.db
    }

    /// Find an active git document by collection ID and name (file path).
    pub async fn get_active_git_document_by_name(
        &self,
        collection_id: Uuid,
        name: &str,
    ) -> Result<Option<Document>, AppError> {
        tracing::debug!(
            component = "documents/repository",
            collection_id = %collection_id,
            document_name = %name,
            "document.get_active_git"
        );

        let row = sqlx::query_as::<_, DocumentRow>(
            "SELECT id, name, file_type, file_size, uploaded_at, collection_id, is_active, user_id, source
             FROM documents
             WHERE collection_id = $1 AND name = $2 AND source = 'git' AND is_active = TRUE",
        )
        .bind(collection_id)
        .bind(name)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        row.map(Document::try_from).transpose()
    }

    /// Soft-deactivate all git-sourced documents (and their chunks) in a collection.
    /// Returns the number of documents deactivated.
    pub async fn deactivate_git_documents_for_collection(
        &self,
        collection_id: Uuid,
    ) -> Result<u64, AppError> {
        tracing::debug!(
            component = "documents/repository",
            collection_id = %collection_id,
            "documents.deactivate_git_for_collection"
        );

        // First deactivate chunks for all git documents
        let doc_ids: Vec<Uuid> = sqlx::query_as::<_, (Uuid,)>(
            "SELECT id FROM documents WHERE collection_id = $1 AND source = 'git' AND is_active = TRUE"
        )
        .bind(collection_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        .into_iter()
        .map(|(id,)| id)
        .collect();

        if !doc_ids.is_empty() {
            self.deactivate_chunks_batch(&doc_ids).await?;
        }

        // Then deactivate the documents themselves
        let affected = if doc_ids.is_empty() {
            0
        } else {
            let mut query_builder: QueryBuilder<Postgres> =
                QueryBuilder::new("UPDATE documents SET is_active = FALSE WHERE id IN (");
            let mut separated = query_builder.separated(", ");
            for id in &doc_ids {
                separated.push_bind(id);
            }
            separated.push_unseparated(")");
            query_builder
                .build()
                .execute(&self.db)
                .await
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to deactivate git documents: {e}"))
                })?
                .rows_affected()
        };

        tracing::info!(
            component = "documents/repository",
            collection_id = %collection_id,
            deactivated_count = affected,
            "documents.deactivate_git_for_collection.complete"
        );

        Ok(affected)
    }

    /// Soft-deactivate git documents by their file names within a collection.
    /// Only targets active `source='git'` documents matching the given names.
    pub async fn deactivate_git_documents_by_names(
        &self,
        collection_id: Uuid,
        names: &[&str],
    ) -> Result<u64, AppError> {
        if names.is_empty() {
            return Ok(0);
        }

        tracing::debug!(
            component = "documents/repository",
            collection_id = %collection_id,
            name_count = names.len(),
            "documents.deactivate_git_by_names"
        );

        // Find matching document IDs
        let mut qb: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT id FROM documents WHERE collection_id = ");
        qb.push_bind(collection_id);
        qb.push(" AND source = 'git' AND is_active = TRUE AND name IN (");
        let mut separated = qb.separated(", ");
        for name in names {
            separated.push_bind(name);
        }
        separated.push_unseparated(")");

        let doc_ids: Vec<Uuid> = qb
            .build_query_as::<(Uuid,)>()
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
            .into_iter()
            .map(|(id,)| id)
            .collect();

        if !doc_ids.is_empty() {
            self.deactivate_chunks_batch(&doc_ids).await?;

            let mut update_qb: QueryBuilder<Postgres> =
                QueryBuilder::new("UPDATE documents SET is_active = FALSE WHERE id IN (");
            let mut separated = update_qb.separated(", ");
            for id in &doc_ids {
                separated.push_bind(id);
            }
            separated.push_unseparated(")");
            let affected = update_qb
                .build()
                .execute(&self.db)
                .await
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to deactivate git documents: {e}"))
                })?
                .rows_affected();

            tracing::info!(
                component = "documents/repository",
                collection_id = %collection_id,
                deactivated_count = affected,
                "documents.deactivate_git_by_names.complete"
            );

            return Ok(affected);
        }

        tracing::warn!(
            component = "documents/repository",
            collection_id = %collection_id,
            "documents.deactivate_git_by_names.no_matches"
        );

        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    // Tests migrated to sqlx::test with PostgreSQL fixtures (Phase 3)
}
