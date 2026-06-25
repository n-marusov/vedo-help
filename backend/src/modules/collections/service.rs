use uuid::Uuid;

use crate::modules::collections::models::{Collection, CollectionSummary, CreateCollectionRequest};
use crate::modules::collections::repository::CollectionRepository;
use crate::shared::chroma_client::ChromaClient;
use crate::shared::error::AppError;

/// Service for collection management operations.
#[derive(Clone, Debug)]
pub struct CollectionService {
    repo: CollectionRepository,
    chroma_url: String,
}

impl CollectionService {
    /// Create a new CollectionService.
    pub fn new(repo: CollectionRepository, chroma_url: String) -> Self {
        Self { repo, chroma_url }
    }

    /// Create a new collection. Validates name uniqueness and creates in both
    /// PostgreSQL and Chroma.
    pub async fn create(
        &self,
        req: CreateCollectionRequest,
    ) -> Result<CollectionSummary, AppError> {
        tracing::info!(component = "collections/service", collection_name = %req.name, "collection.create");

        let name = req.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::BadRequest(
                "Collection name cannot be empty".to_string(),
            ));
        }

        let id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let collection = Collection {
            id,
            name: name.clone(),
            description: req.description,
            created_at: now,
            document_count: 0,
        };

        // Create in PostgreSQL first
        self.repo.create_collection(&collection).await?;

        // Create in Chroma — use UUID as Chroma collection name (Chroma only
        // accepts ASCII alphanumeric, underscores and hyphens). The display name
        // is stored in PostgreSQL and shown in the UI.
        let chroma = ChromaClient::new(&self.chroma_url);
        if let Err(e) = chroma.create_collection(&id.to_string()).await {
            // Rollback PostgreSQL creation if Chroma fails
            tracing::error!(component = "collections/service", collection_name = %name, error = %e, "collection.create.chroma_error");
            let _ = self.repo.delete_collection(id).await;
            return Err(e);
        }

        tracing::info!(component = "collections/service", collection_id = %id, collection_name = %name, "collection.created");

        Ok(CollectionSummary {
            id,
            name,
            document_count: 0,
            created_at: now,
        })
    }

    /// List all collections.
    pub async fn list(&self) -> Result<Vec<CollectionSummary>, AppError> {
        tracing::debug!(component = "collections/service", "collection.list");

        let collections = self.repo.list_collections().await?;
        let summaries: Vec<CollectionSummary> = collections
            .into_iter()
            .map(|c| CollectionSummary {
                id: c.id,
                name: c.name,
                document_count: c.document_count,
                created_at: c.created_at,
            })
            .collect();

        tracing::debug!(
            component = "collections/service",
            count = summaries.len(),
            "collection.list.return"
        );
        Ok(summaries)
    }

    /// Get a single collection by ID.
    pub async fn get(&self, id: Uuid) -> Result<Collection, AppError> {
        tracing::debug!(component = "collections/service", collection_id = %id, "collection.get");
        self.repo.get_collection(id).await
    }

    /// Delete a collection. Removes from PostgreSQL and drops the Chroma collection.
    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        tracing::info!(component = "collections/service", collection_id = %id, "collection.delete");

        // Get the collection first to know the name (for Chroma deletion)
        let collection = self.repo.get_collection(id).await?;

        // Delete from PostgreSQL first (includes cascade to documents/chunks)
        self.repo.delete_collection(id).await?;

        // Delete from Chroma — use UUID as collection name (re-derived from id)
        let chroma = ChromaClient::new(&self.chroma_url);
        if let Err(e) = chroma.delete_collection(&id.to_string()).await {
            // Log but don't fail — the PostgreSQL data is already cleaned up
            tracing::warn!(component = "collections/service", collection_name = %collection.name, error = %e, "collection.delete.chroma_warning");
        }

        tracing::info!(component = "collections/service", collection_id = %id, "collection.deleted");
        Ok(())
    }
}
