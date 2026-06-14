use std::convert::Infallible;

use futures::stream::{self, Stream};
use futures::StreamExt;
use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::modules::query::models::{QueryRequest, SourceRef, StreamEvent};
use crate::modules::query::repository::QueryRepository;
use crate::shared::embedding_client::EmbeddingClient;
use crate::shared::error::AppError;
use crate::shared::llm::{Message, OpenRouterClient};

/// Service for processing RAG queries with streaming responses.
#[derive(Clone, Debug)]
pub struct QueryService {
    repo: QueryRepository,
    llm_client: OpenRouterClient,
    embedding_client: EmbeddingClient,
}

impl QueryService {
    /// Create a new QueryService.
    pub fn new(
        db: SqlitePool,
        chroma_url: &str,
        llm_client: OpenRouterClient,
        embedding_service_url: &str,
    ) -> Self {
        let repo = QueryRepository::new(db, chroma_url);
        let embedding_client = EmbeddingClient::new(embedding_service_url);
        tracing::debug!("QueryService initialized");
        Self {
            repo,
            llm_client,
            embedding_client,
        }
    }

    /// Process a query and return a stream of SSE events.
    ///
    /// Steps:
    /// 1. Embed the query via the embedding service
    /// 2. Search Chroma for the top-5 most similar chunks
    /// 3. Fetch full chunk data from SQLite
    /// 4. Load conversation history (if session_id is provided)
    /// 5. Stream the LLM response, yielding events: "chunk", "sources", "done"
    pub async fn process_query(
        &self,
        request: QueryRequest,
    ) -> Result<impl Stream<Item = Result<StreamEvent, Infallible>>, AppError> {
        tracing::info!(
            "Processing query: collection={}, session={:?}, query_len={}",
            request.collection_id,
            request.session_id,
            request.query.len()
        );

        // 1. Embed the query
        tracing::debug!("Embedding query text");
        let embeddings = self
            .embedding_client
            .embed(vec![request.query.clone()])
            .await
            .map_err(|e| {
                tracing::error!("Embedding failed: {e}");
                e
            })?;

        let embedding = embeddings.into_iter().next().ok_or_else(|| {
            let err =
                AppError::EmbeddingError("Embedding service returned empty result".to_string());
            tracing::error!("{err}");
            err
        })?;

        tracing::debug!("Query embedded: dim={}", embedding.len());

        // 2. Search Chroma for similar chunks
        let collection_name = request.collection_id.to_string();
        let chroma_results = self
            .repo
            .query_chroma(&collection_name, &embedding, 5)
            .await
            .map_err(|e| {
                tracing::error!("Chroma search failed: {e}");
                e
            })?;

        if chroma_results.is_empty() {
            tracing::warn!(
                "No results found for query in collection {}",
                request.collection_id
            );
        }

        // 3. Fetch chunk text + document names from SQLite
        let chunk_ids: Vec<String> = chroma_results.iter().map(|r| r.id.clone()).collect();
        let chunks = self.repo.get_chunks_by_ids(&chunk_ids).await?;

        // Build SourceRefs for the sources SSE event
        let source_refs: Vec<SourceRef> = chroma_results
            .iter()
            .map(|r| {
                let doc_id = Uuid::parse_str(&r.document_id).unwrap_or_default();
                let doc_name = chunks
                    .iter()
                    .find(|c| c.index == r.chunk_index)
                    .map(|c| c.document_name.clone())
                    .unwrap_or_default();
                SourceRef {
                    document_id: doc_id,
                    document_name: doc_name,
                    chunk_index: r.chunk_index,
                    text: r.text.clone(),
                    relevance: r.score,
                }
            })
            .collect();

        // 4. Load conversation history if session is present
        let history = if let Some(session_id) = request.session_id {
            self.load_conversation_history(session_id).await?
        } else {
            Vec::new()
        };

        // 5. Stream LLM response
        let llm_stream = self
            .llm_client
            .query_stream(&request.query, &chunks, &history)
            .await?;

        let stream =
            Self::build_event_stream(llm_stream, source_refs, request.session_id, chunk_ids);

        Ok(stream)
    }

    /// Build the final event stream:
    ///   1. LLM text chunks → "chunk" events
    ///   2. Sources metadata → "sources" event
    ///   3. Completion signal → "done" event
    fn build_event_stream(
        llm_stream: impl Stream<Item = Result<String, AppError>> + 'static,
        sources: Vec<SourceRef>,
        _session_id: Option<Uuid>,
        _chunk_ids: Vec<String>,
    ) -> impl Stream<Item = Result<StreamEvent, Infallible>> {
        let sources_event = StreamEvent {
            event_type: "sources".to_string(),
            data: json!({"sources": sources}),
        };

        llm_stream
            .map(|result| match result {
                Ok(text) => Ok(StreamEvent {
                    event_type: "chunk".to_string(),
                    data: json!({"text": text}),
                }),
                Err(e) => {
                    tracing::error!("LLM stream error: {e}");
                    Ok(StreamEvent {
                        event_type: "error".to_string(),
                        data: json!({"text": e.to_string()}),
                    })
                }
            })
            .chain(stream::once(async move { Ok(sources_event) }))
            .chain(stream::once(async move {
                Ok(StreamEvent {
                    event_type: "done".to_string(),
                    data: json!({}),
                })
            }))
    }

    /// Load conversation history for a session from the `messages` table.
    async fn load_conversation_history(&self, session_id: Uuid) -> Result<Vec<Message>, AppError> {
        #[derive(sqlx::FromRow)]
        struct MessageRow {
            role: String,
            content: String,
        }

        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT role, content FROM messages \
             WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(session_id.to_string())
        .fetch_all(self.repo.db())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to load history: {e}")))?;

        tracing::debug!("Loaded {} messages for session {session_id}", rows.len());

        Ok(rows
            .into_iter()
            .map(|r| Message {
                role: r.role,
                content: r.content,
            })
            .collect())
    }
}
