use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::collections::models::Collection;
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
            "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(collection.id)
        .bind(&collection.name)
        .bind(&collection.description)
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

    /// List all collections with their document counts.
    pub async fn list_collections(&self) -> Result<Vec<Collection>, AppError> {
        tracing::debug!(
            component = "collections/repository",
            "collection.list.started"
        );

        let rows = sqlx::query_as::<
            _,
            (
                uuid::Uuid,
                String,
                Option<String>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            "SELECT id, name, description, created_at FROM collections ORDER BY created_at DESC",
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let mut collections = Vec::with_capacity(rows.len());
        for row in rows {
            let count = self.get_document_count(row.0).await.unwrap_or(0);

            collections.push(Collection {
                id: row.0,
                name: row.1,
                description: row.2,
                created_at: row.3,
                document_count: count,
            });
        }

        tracing::debug!(
            component = "collections/repository",
            count = collections.len(),
            "collection.list.found"
        );
        Ok(collections)
    }

    /// Retrieve a single collection by ID.
    pub async fn get_collection(&self, id: Uuid) -> Result<Collection, AppError> {
        tracing::debug!(component = "collections/repository", collection_id = %id, "collection.get.started");

        let row =
            sqlx::query_as::<
                _,
                (
                    uuid::Uuid,
                    String,
                    Option<String>,
                    chrono::DateTime<chrono::Utc>,
                ),
            >("SELECT id, name, description, created_at FROM collections WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::NotFound(format!("Collection {id} not found")))?;

        let count = self.get_document_count(id).await.unwrap_or(0);

        Ok(Collection {
            id: row.0,
            name: row.1,
            description: row.2,
            created_at: row.3,
            document_count: count,
        })
    }

    /// Delete a collection and its associated documents and chunks.
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
}

#[cfg(test)]
mod tests {
    // Tests migrated to sqlx::test with PostgreSQL fixtures (Phase 3)
}
