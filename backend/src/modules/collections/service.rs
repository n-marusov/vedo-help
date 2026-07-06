use uuid::Uuid;

use crate::modules::collections::models::{
    ChunkSearchQuery, Collection, CollectionStats, CollectionSummary, CreateCollectionRequest,
};
use crate::modules::collections::repository::CollectionRepository;
use crate::shared::chroma_client::ChromaClient;
use crate::shared::error::AppError;

/// Service for collection management operations.
#[derive(Clone, Debug)]
pub struct CollectionService {
    repo: CollectionRepository,
    chroma_url: String,
    embedding_client: crate::shared::embedding_client::EmbeddingClient,
}

impl CollectionService {
    /// Create a new CollectionService.
    pub fn new(
        repo: CollectionRepository,
        chroma_url: String,
        embedding_client: crate::shared::embedding_client::EmbeddingClient,
    ) -> Self {
        Self {
            repo,
            chroma_url,
            embedding_client,
        }
    }

    /// Create a new collection. Validates name uniqueness and creates in both
    /// PostgreSQL and Chroma.
    pub async fn create(
        &self,
        req: CreateCollectionRequest,
        user_id: &str,
    ) -> Result<CollectionSummary, AppError> {
        tracing::info!(component = "collections/service", collection_name = %req.name, user_id = %user_id, "collection.create");

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
            user_id: user_id.to_string(),
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

        tracing::info!(component = "collections/service", collection_id = %id, collection_name = %name, user_id = %user_id, "collection.created");

        Ok(CollectionSummary {
            id,
            name,
            document_count: 0,
            created_at: now,
        })
    }

    /// List all collections for a user.
    /// Non-admin users see only their own collections; admin users see all.
    pub async fn list(
        &self,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<CollectionSummary>, AppError> {
        tracing::debug!(component = "collections/service", user_id = %user_id, is_admin = %is_admin, "collection.list");

        let collections = self
            .repo
            .list_collections_by_user(user_id, is_admin)
            .await?;
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

    /// Get a single collection by ID with ownership check.
    pub async fn get(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Collection, AppError> {
        tracing::debug!(component = "collections/service", collection_id = %id, user_id = %user_id, "collection.get");
        self.repo
            .get_collection_for_user(id, user_id, is_admin)
            .await
    }

    /// Get comprehensive statistics for a collection with ownership check.
    pub async fn get_stats(
        &self,
        collection_id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<CollectionStats, AppError> {
        tracing::info!(
            component = "collections/service",
            collection_id = %collection_id,
            user_id = %user_id,
            is_admin = %is_admin,
            "collection.get_stats"
        );

        // Verify ownership first
        self.repo
            .get_collection_for_user(collection_id, user_id, is_admin)
            .await?;

        self.repo.get_collection_stats(collection_id).await
    }

    /// Search chunks in a collection with ownership check.
    /// Supports text search (PostgreSQL ILIKE) and semantic search (Chroma).
    pub async fn search_chunks(
        &self,
        collection_id: Uuid,
        user_id: &str,
        is_admin: bool,
        params: &ChunkSearchQuery,
    ) -> Result<Vec<crate::shared::chunk_search::ChunkSearchResult>, AppError> {
        tracing::info!(
            component = "collections/service",
            collection_id = %collection_id,
            search_type = ?params.search_type,
            "collection.search_chunks"
        );

        // Verify ownership first
        self.repo
            .get_collection_for_user(collection_id, user_id, is_admin)
            .await?;

        let search_type = params.search_type.as_deref().unwrap_or("text");
        let source = params.source.as_deref();

        match search_type {
            "semantic" => {
                let chroma = crate::shared::chroma_client::ChromaClient::new(&self.chroma_url);
                let query = params.q.as_deref().unwrap_or("");
                let top_k = params.top_k.unwrap_or(20);

                crate::shared::chunk_search::search_chunks_semantic(
                    &chroma,
                    &self.embedding_client,
                    self.repo.db(),
                    collection_id,
                    query,
                    source,
                    top_k,
                    crate::shared::embedding_client::DEFAULT_EMBEDDING_MODEL,
                )
                .await
            }
            _ => {
                let query = params.q.as_deref().unwrap_or("");
                let limit = params.limit.unwrap_or(20);
                let offset = params.offset.unwrap_or(0);

                crate::shared::chunk_search::search_chunks_text(
                    self.repo.db(),
                    collection_id,
                    query,
                    source,
                    limit,
                    offset,
                )
                .await
            }
        }
    }

    /// Delete a collection. Removes from PostgreSQL and drops the Chroma collection.
    pub async fn delete(&self, id: Uuid, user_id: &str, is_admin: bool) -> Result<(), AppError> {
        tracing::info!(component = "collections/service", collection_id = %id, user_id = %user_id, "collection.delete");

        // Get the collection first to know the name (for Chroma deletion)
        // This also verifies ownership
        let collection = self
            .repo
            .get_collection_for_user(id, user_id, is_admin)
            .await?;

        // Delete from PostgreSQL with ownership check
        self.repo
            .delete_collection_for_user(id, user_id, is_admin)
            .await?;

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
