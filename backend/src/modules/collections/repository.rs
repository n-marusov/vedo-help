use sqlx::SqlitePool;
use uuid::Uuid;

use crate::modules::collections::models::Collection;
use crate::shared::error::AppError;

/// Repository for collection data access.
#[derive(Clone, Debug)]
pub struct CollectionRepository {
    db: SqlitePool,
}

impl CollectionRepository {
    /// Create a new CollectionRepository with the given database pool.
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Insert a new collection into SQLite.
    pub async fn create_collection(&self, collection: &Collection) -> Result<Uuid, AppError> {
        tracing::debug!("Creating collection: {}", collection.name);

        sqlx::query(
            "INSERT INTO collections (id, name, description, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(collection.id.to_string())
        .bind(&collection.name)
        .bind(&collection.description)
        .bind(collection.created_at.to_rfc3339())
        .execute(&self.db)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                AppError::BadRequest(format!("Collection '{}' already exists", collection.name))
            } else {
                AppError::InternalError(format!("Failed to create collection: {e}"))
            }
        })?;

        tracing::info!(
            "Collection created: {id} ({name})",
            id = collection.id,
            name = collection.name
        );

        Ok(collection.id)
    }

    /// List all collections with their document counts.
    pub async fn list_collections(&self) -> Result<Vec<Collection>, AppError> {
        tracing::debug!("Listing all collections");

        let rows = sqlx::query_as::<_, (String, String, Option<String>, String)>(
            "SELECT id, name, description, created_at FROM collections ORDER BY created_at DESC",
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let mut collections = Vec::with_capacity(rows.len());
        for row in rows {
            let id = Uuid::parse_str(&row.0).unwrap_or_default();
            let count = self.get_document_count(id).await.unwrap_or(0);

            collections.push(Collection {
                id,
                name: row.1,
                description: row.2,
                created_at: row.3.parse().unwrap_or_else(|_| chrono::Utc::now()),
                document_count: count,
            });
        }

        tracing::debug!("Found {} collections", collections.len());
        Ok(collections)
    }

    /// Retrieve a single collection by ID.
    pub async fn get_collection(&self, id: Uuid) -> Result<Collection, AppError> {
        tracing::debug!("Fetching collection: {id}");

        let row = sqlx::query_as::<_, (String, String, Option<String>, String)>(
            "SELECT id, name, description, created_at FROM collections WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        .ok_or_else(|| AppError::NotFound(format!("Collection {id} not found")))?;

        let count = self.get_document_count(id).await.unwrap_or(0);

        Ok(Collection {
            id: Uuid::parse_str(&row.0).unwrap_or(id),
            name: row.1,
            description: row.2,
            created_at: row.3.parse().unwrap_or_else(|_| chrono::Utc::now()),
            document_count: count,
        })
    }

    /// Delete a collection and its associated documents and chunks.
    pub async fn delete_collection(&self, id: Uuid) -> Result<(), AppError> {
        tracing::debug!("Deleting collection: {id}");

        // Delete chunks for documents in this collection
        sqlx::query(
            "DELETE FROM chunks WHERE document_id IN (SELECT id FROM documents WHERE collection_id = ?)",
        )
        .bind(id.to_string())
        .execute(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to delete chunks: {e}")))?;

        // Delete documents in this collection
        sqlx::query("DELETE FROM documents WHERE collection_id = ?")
            .bind(id.to_string())
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete documents: {e}")))?;

        // Update sessions referencing this collection to set collection_id to NULL
        sqlx::query("UPDATE sessions SET collection_id = NULL WHERE collection_id = ?")
            .bind(id.to_string())
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to update sessions: {e}")))?;

        // Delete the collection itself
        let affected = sqlx::query("DELETE FROM collections WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete collection: {e}")))?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Collection {id} not found")));
        }

        tracing::info!("Collection deleted: {id}");
        Ok(())
    }

    /// Count documents belonging to a collection.
    pub async fn get_document_count(&self, id: Uuid) -> Result<i64, AppError> {
        let row =
            sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM documents WHERE collection_id = ?")
                .bind(id.to_string())
                .fetch_one(&self.db)
                .await
                .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        Ok(row.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::error::AppError;
    use sqlx::sqlite::SqlitePoolOptions;

    /// Create an in-memory SQLite pool with collections table for testing.
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await
            .expect("Failed to create test database");

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
        .expect("Failed to create collections table");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                file_type TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                uploaded_at TEXT NOT NULL,
                collection_id TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create documents table");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chunks (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                text TEXT NOT NULL,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create chunks table");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT 'New Chat',
                collection_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE SET NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create sessions table");

        pool
    }

    #[tokio::test]
    async fn test_create_cyrillic_collection_in_sqlite() {
        // Regression: verify that SQLite does NOT impose Chroma's ASCII-only
        // naming constraint. The display name is stored in SQLite as-is while
        // Chroma gets the UUID.
        let db = setup_test_db().await;
        let repo = CollectionRepository::new(db);

        let id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let collection = Collection {
            id,
            name: "Техническая документация".to_string(),
            description: Some("Описание".to_string()),
            created_at: now,
            document_count: 0,
        };

        let result = repo.create_collection(&collection).await;
        assert!(
            result.is_ok(),
            "SQLite should accept Cyrillic collection names"
        );
        assert_eq!(result.unwrap(), id);

        // Verify we can read it back with the Cyrillic name intact
        let fetched = repo.get_collection(id).await.unwrap();
        assert_eq!(fetched.name, "Техническая документация");
        assert_eq!(fetched.description, Some("Описание".to_string()));

        // Verify it appears in list
        let list = repo.list_collections().await.unwrap();
        assert!(list.iter().any(|c| c.name == "Техническая документация"));
    }

    #[tokio::test]
    async fn test_create_then_delete_collection() {
        let db = setup_test_db().await;
        let repo = CollectionRepository::new(db);

        let id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let collection = Collection {
            id,
            name: "test-collection".to_string(),
            description: None,
            created_at: now,
            document_count: 0,
        };

        // Create
        repo.create_collection(&collection).await.unwrap();

        // Verify listed
        let list = repo.list_collections().await.unwrap();
        assert_eq!(list.len(), 1);
        assert!(!list[0].name.is_empty());

        // Delete
        repo.delete_collection(id).await.unwrap();

        // Verify gone
        let list = repo.list_collections().await.unwrap();
        assert_eq!(list.len(), 0);

        // Verify not found
        let err = repo.get_collection(id).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_create_duplicate_collection_fails() {
        let db = setup_test_db().await;
        let repo = CollectionRepository::new(db);
        let now = chrono::Utc::now();

        let collection = Collection {
            id: Uuid::new_v4(),
            name: "same-name".to_string(),
            description: None,
            created_at: now,
            document_count: 0,
        };

        repo.create_collection(&collection).await.unwrap();

        let dup = Collection {
            id: Uuid::new_v4(),
            name: "same-name".to_string(),
            description: None,
            created_at: now,
            document_count: 0,
        };

        let err = repo.create_collection(&dup).await.unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }
}
