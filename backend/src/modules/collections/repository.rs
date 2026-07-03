use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::collections::models::{Collection, CollectionStats};
use crate::shared::error::AppError;

/// Repository for collection data access.
#[derive(Clone, Debug)]
pub struct CollectionRepository {
    db: PgPool,
}

impl CollectionRepository {
    /// Create a new CollectionRepository with the given database pool.
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Insert a new collection into PostgreSQL.
    pub async fn create_collection(&self, collection: &Collection) -> Result<Uuid, AppError> {
        tracing::debug!(component = "collections/repository", collection_name = %collection.name, "collection.create.started");

        sqlx::query(
            "INSERT INTO collections (id, name, description, user_id, created_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(collection.id)
        .bind(&collection.name)
        .bind(&collection.description)
        .bind(&collection.user_id)
        .bind(collection.created_at)
        .execute(&self.db)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                AppError::BadRequest(format!("Collection '{}' already exists", collection.name))
            } else {
                AppError::InternalError(format!("Failed to create collection: {e}"))
            }
        })?;

        tracing::info!(component = "collections/repository", collection_id = %collection.id, collection_name = %collection.name, "collection.created");

        Ok(collection.id)
    }

    /// List all collections visible to a specific user.
    /// Non-admin users see only their own collections; admin users see all.
    pub async fn list_collections_by_user(
        &self,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<Collection>, AppError> {
        tracing::debug!(
            component = "collections/repository",
            "collection.list.started"
        );

        let rows = if is_admin {
            sqlx::query_as::<
                _,
                (uuid::Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>, String),
            >(
                "SELECT id, name, description, created_at, user_id FROM collections ORDER BY created_at DESC",
            )
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        } else {
            sqlx::query_as::<
                _,
                (uuid::Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>, String),
            >(
                "SELECT id, name, description, created_at, user_id FROM collections WHERE user_id = $1 ORDER BY created_at DESC",
            )
            .bind(user_id)
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        };

        let mut collections = Vec::with_capacity(rows.len());
        for row in rows {
            let count = self.get_document_count(row.0).await.unwrap_or(0);

            collections.push(Collection {
                id: row.0,
                name: row.1,
                description: row.2,
                created_at: row.3,
                document_count: count,
                user_id: row.4,
            });
        }

        tracing::debug!(
            component = "collections/repository",
            count = collections.len(),
            "collection.list.found"
        );
        Ok(collections)
    }

    /// Legacy alias: list all collections (for backward compat / admin).
    pub async fn list_collections(&self) -> Result<Vec<Collection>, AppError> {
        self.list_collections_by_user("", true).await
    }

    /// Retrieve a single collection by ID and verify user ownership.
    /// Non-admin users can only access their own collections.
    pub async fn get_collection_for_user(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Collection, AppError> {
        tracing::debug!(component = "collections/repository", collection_id = %id, "collection.get.started");

        let row = if is_admin {
            sqlx::query_as::<
                _,
                (
                    uuid::Uuid,
                    String,
                    Option<String>,
                    chrono::DateTime<chrono::Utc>,
                    String,
                ),
            >(
                "SELECT id, name, description, created_at, user_id FROM collections WHERE id = $1"
            )
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::NotFound(format!("Collection {id} not found")))?
        } else {
            sqlx::query_as::<
                _,
                (uuid::Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>, String),
            >("SELECT id, name, description, created_at, user_id FROM collections WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::NotFound(format!("Collection {id} not found")))?
        };

        let count = self.get_document_count(id).await.unwrap_or(0);

        Ok(Collection {
            id: row.0,
            name: row.1,
            description: row.2,
            created_at: row.3,
            document_count: count,
            user_id: row.4,
        })
    }

    /// Retrieve a single collection by ID (legacy, no ownership check).
    pub async fn get_collection(&self, id: Uuid) -> Result<Collection, AppError> {
        self.get_collection_for_user(id, "", true).await
    }

    /// Find a collection by ID and user, returning None if not found or not owned.
    pub async fn find_by_id_and_user(
        &self,
        id: Uuid,
        user_id: &str,
    ) -> Result<Option<Collection>, AppError> {
        tracing::debug!(component = "collections/repository", collection_id = %id, "collection.find_by_user.started");

        let row = sqlx::query_as::<
            _,
            (uuid::Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>, String),
        >("SELECT id, name, description, created_at, user_id FROM collections WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        match row {
            Some(r) => {
                let count = self.get_document_count(id).await.unwrap_or(0);
                Ok(Some(Collection {
                    id: r.0,
                    name: r.1,
                    description: r.2,
                    created_at: r.3,
                    document_count: count,
                    user_id: r.4,
                }))
            }
            None => Ok(None),
        }
    }

    /// Delete a collection by ID (after ownership verification).
    pub async fn delete_collection(&self, id: Uuid) -> Result<(), AppError> {
        tracing::debug!(component = "collections/repository", collection_id = %id, "collection.delete.started");

        // Delete chunks for documents in this collection
        sqlx::query(
            "DELETE FROM chunks WHERE document_id IN (SELECT id FROM documents WHERE collection_id = $1)",
        )
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to delete chunks: {e}")))?;

        // Delete documents in this collection
        sqlx::query("DELETE FROM documents WHERE collection_id = $1")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete documents: {e}")))?;

        // Update sessions referencing this collection to set collection_id to NULL
        sqlx::query("UPDATE sessions SET collection_id = NULL WHERE collection_id = $1")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to update sessions: {e}")))?;

        // Delete the collection itself
        let affected = sqlx::query("DELETE FROM collections WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete collection: {e}")))?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Collection {id} not found")));
        }

        tracing::info!(component = "collections/repository", collection_id = %id, "collection.deleted");
        Ok(())
    }

    /// Delete a collection by ID with ownership check.
    pub async fn delete_collection_for_user(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<(), AppError> {
        tracing::debug!(component = "collections/repository", collection_id = %id, "collection.delete_for_user.started");

        if is_admin {
            return self.delete_collection(id).await;
        }

        // Ownership check: verify the collection belongs to this user
        let owned = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM collections WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
            > 0;

        if !owned {
            return Err(AppError::NotFound(format!("Collection {id} not found")));
        }

        self.delete_collection(id).await
    }

    /// Count documents belonging to a collection.
    pub async fn get_document_count(&self, id: Uuid) -> Result<i64, AppError> {
        let row =
            sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM documents WHERE collection_id = $1")
                .bind(id)
                .fetch_one(&self.db)
                .await
                .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        Ok(row.0)
    }

    /// Access the underlying PostgreSQL pool.
    pub fn db(&self) -> &PgPool {
        &self.db
    }

    /// Get comprehensive statistics for a collection.
    pub async fn get_collection_stats(
        &self,
        collection_id: Uuid,
    ) -> Result<CollectionStats, AppError> {
        tracing::debug!(component = "collections/repository", collection_id = %collection_id, "collection.stats.fetch");

        // Query A: document counts by source + total file size (3 metrics in 1 query)
        let doc_stats: Vec<(String, i64, Option<i64>)> = sqlx::query_as(
            "SELECT source, COUNT(*) as cnt, SUM(file_size) as total_size
             FROM documents
             WHERE collection_id = $1 AND is_active = TRUE
             GROUP BY source",
        )
        .bind(collection_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let mut total_documents = 0i64;
        let mut upload_documents = 0i64;
        let mut git_documents = 0i64;
        let mut total_file_size_bytes = 0i64;
        for (source, count, file_size) in &doc_stats {
            total_documents += count;
            total_file_size_bytes += file_size.unwrap_or(0);
            match source.as_str() {
                "upload" => upload_documents = *count,
                "git" => git_documents = *count,
                _ => {}
            }
        }

        // Query B: chunk counts by source (total + breakdown in 1 query)
        let chunk_stats: Vec<(String, i64)> = sqlx::query_as(
            "SELECT d.source, COUNT(*) as cnt
             FROM chunks c
             JOIN documents d ON c.document_id = d.id
             WHERE d.collection_id = $1 AND c.is_active = TRUE AND d.is_active = TRUE
             GROUP BY d.source",
        )
        .bind(collection_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let mut total_chunks = 0i64;
        let mut upload_chunks = 0i64;
        let mut git_chunks = 0i64;
        for (source, count) in &chunk_stats {
            total_chunks += count;
            match source.as_str() {
                "upload" => upload_chunks = *count,
                "git" => git_chunks = *count,
                _ => {}
            }
        }

        // Query C: file type breakdown
        let document_types: Vec<(String, i64)> = sqlx::query_as(
            "SELECT file_type, COUNT(*) FROM documents WHERE collection_id = $1 AND is_active = TRUE GROUP BY file_type",
        )
        .bind(collection_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        // Query D: git repos count
        let (total_git_repos,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM git_repositories WHERE collection_id = $1")
                .bind(collection_id)
                .fetch_one(&self.db)
                .await
                .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let stats = CollectionStats {
            total_documents,
            total_chunks,
            total_git_repos,
            upload_documents,
            git_documents,
            upload_chunks,
            git_chunks,
            total_file_size_bytes,
            document_types: document_types.into_iter().collect(),
        };

        tracing::debug!(
            component = "collections/repository",
            collection_id = %collection_id,
            total_documents = stats.total_documents,
            total_chunks = stats.total_chunks,
            "collection.stats.fetched"
        );

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    // Tests migrated to sqlx::test with PostgreSQL fixtures (Phase 3)
}
