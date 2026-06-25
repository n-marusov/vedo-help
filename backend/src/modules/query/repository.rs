use sqlx::PgPool;
use uuid::Uuid;

use crate::shared::error::AppError;
use crate::shared::llm::CrateChunkData;
use crate::shared::types::ChromaResult;
use crate::shared::ChromaClient;

/// Repository for query-related data access: Chroma vector search and
/// PostgreSQL chunk / document lookups.
#[derive(Clone, Debug)]
pub struct QueryRepository {
    db: PgPool,
    chroma: ChromaClient,
}

impl QueryRepository {
    /// Create a new QueryRepository with the given database pool and Chroma URL.
    pub fn new(db: PgPool, chroma_url: &str) -> Self {
        let chroma = ChromaClient::new(chroma_url);
        tracing::debug!(component = "query/repository", "repository.initialized");
        Self { db, chroma }
    }

    /// Query Chroma for the top-k most similar chunks to the given embedding.
    ///
    /// The `collection_name` is the UUID string of the collection.
    pub async fn query_chroma(
        &self,
        collection_name: &str,
        embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<ChromaResult>, AppError> {
        tracing::debug!(
            component = "query/repository",
            collection_name = %collection_name,
            top_k = top_k,
            embedding_dimension = embedding.len(),
            "chroma.query.start"
        );

        let results = self
            .chroma
            .query(
                collection_name,
                embedding,
                top_k,
                Some(serde_json::json!({"is_active": true})),
            )
            .await?;

        tracing::info!(
            component = "query/repository",
            result_count = results.len(),
            collection_name = %collection_name,
            "chroma.query.found"
        );
        Ok(results)
    }

    /// Fetch chunks from PostgreSQL by their IDs, joined with document names.
    ///
    /// Returns `CrateChunkData` with text, index, and document_name populated.
    /// Maintains the order of the input `ids` slice.
    pub async fn get_chunks_by_ids(&self, ids: &[String]) -> Result<Vec<CrateChunkData>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        tracing::debug!(
            component = "query/repository",
            request_count = ids.len(),
            "chunks.fetch_by_ids.start"
        );

        // Build placeholders for the IN clause (PostgreSQL numbered params)
        let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("${i}")).collect();
        let query_str = format!(
            "SELECT c.id, c.index, c.text, d.name AS document_name \
             FROM chunks c \
             JOIN documents d ON c.document_id = d.id \
             WHERE c.id IN ({}) AND c.is_active = TRUE",
            placeholders.join(", ")
        );

        let mut query = sqlx::query_as::<_, (uuid::Uuid, i32, String, String)>(&query_str);
        for id in ids {
            let uuid = Uuid::parse_str(id).map_err(|e| {
                AppError::InternalError(format!("Invalid chunk UUID from Chroma: {e}"))
            })?;
            query = query.bind(uuid);
        }

        let rows = query
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to fetch chunks: {e}")))?;

        // Build a lookup map keyed by chunk UUID for ordering
        let mut by_id: std::collections::HashMap<Uuid, CrateChunkData> =
            std::collections::HashMap::new();
        for (chunk_id, index, text, document_name) in rows {
            tracing::trace!(
                component = "query/repository",
                chunk_id = %chunk_id,
                document_name = %document_name,
                chunk_index = index,
                "chunk.fetched"
            );
            by_id.insert(
                chunk_id,
                CrateChunkData {
                    text,
                    index: index as usize,
                    document_name,
                },
            );
        }

        // Return in the order of the input ids
        let mut chunks = Vec::with_capacity(ids.len());
        for id_str in ids {
            let id = Uuid::parse_str(id_str).map_err(|e| {
                AppError::InternalError(format!("Invalid chunk UUID from Chroma: {e}"))
            })?;
            if let Some(chunk) = by_id.remove(&id) {
                chunks.push(chunk);
            } else {
                tracing::warn!(
                    component = "query/repository",
                    chunk_id = %id,
                    "chunk.not_found_in_pg"
                );
            }
        }

        tracing::debug!(
            component = "query/repository",
            result_count = chunks.len(),
            "chunks.fetch_by_ids.found"
        );
        Ok(chunks)
    }

    /// Access the underlying PostgreSQL pool (for conversation history, etc.).
    pub fn db(&self) -> &PgPool {
        &self.db
    }
}
