use sqlx::SqlitePool;

use crate::shared::error::AppError;
use crate::shared::llm::CrateChunkData;
use crate::shared::types::ChromaResult;
use crate::shared::ChromaClient;

/// Repository for query-related data access: Chroma vector search and
/// SQLite chunk / document lookups.
#[derive(Clone, Debug)]
pub struct QueryRepository {
    db: SqlitePool,
    chroma: ChromaClient,
}

impl QueryRepository {
    /// Create a new QueryRepository with the given database pool and Chroma URL.
    pub fn new(db: SqlitePool, chroma_url: &str) -> Self {
        let chroma = ChromaClient::new(chroma_url);
        tracing::debug!("QueryRepository initialized");
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
            "Querying Chroma: collection={collection_name}, top_k={top_k}, embedding_dim={}",
            embedding.len()
        );

        let results = self.chroma.query(collection_name, embedding, top_k).await?;

        tracing::info!(
            "Chroma returned {} results for collection={collection_name}",
            results.len()
        );
        Ok(results)
    }

    /// Fetch chunks from SQLite by their IDs, joined with document names.
    ///
    /// Returns `CrateChunkData` with text, index, and document_name populated.
    /// Maintains the order of the input `ids` slice.
    pub async fn get_chunks_by_ids(&self, ids: &[String]) -> Result<Vec<CrateChunkData>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        tracing::debug!("Fetching {} chunks from SQLite", ids.len());

        // Build placeholders for the IN clause
        let placeholders: Vec<String> = (0..ids.len()).map(|_| "?".to_string()).collect();
        let query_str = format!(
            "SELECT c.id, c.index, c.text, d.name AS document_name \
             FROM chunks c \
             JOIN documents d ON c.document_id = d.id \
             WHERE c.id IN ({})",
            placeholders.join(", ")
        );

        let mut query = sqlx::query_as::<_, (String, i64, String, String)>(&query_str);
        for id in ids {
            query = query.bind(id);
        }

        let rows = query
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to fetch chunks: {e}")))?;

        // Build a lookup map keyed by chunk UUID for ordering
        let mut by_id: std::collections::HashMap<String, CrateChunkData> =
            std::collections::HashMap::new();
        for (chunk_id, index, text, document_name) in rows {
            tracing::trace!(
                "Fetched chunk: id={chunk_id}, document={document_name}, index={index}"
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
        for id in ids {
            if let Some(chunk) = by_id.remove(id) {
                chunks.push(chunk);
            } else {
                tracing::warn!("Chunk {id} not found in SQLite — Chroma result may be stale");
            }
        }

        tracing::debug!("Found {} chunks in SQLite", chunks.len());
        Ok(chunks)
    }

    /// Access the underlying SQLite pool (for conversation history, etc.).
    pub fn db(&self) -> &SqlitePool {
        &self.db
    }
}
