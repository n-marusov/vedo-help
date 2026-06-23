use std::convert::Infallible;
use std::sync::Arc;

use futures::stream::{self, Stream};
use futures::StreamExt;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Mutex;
use uuid::Uuid;

use crate::modules::conversations::models::Message;
use crate::modules::conversations::repository::ConversationRepository;
use crate::modules::query::models::{QueryRequest, SourceRef, StreamEvent};
use crate::modules::query::repository::QueryRepository;
use crate::shared::embedding_client::EmbeddingClient;
use crate::shared::error::AppError;
use crate::shared::llm::{LlmClient, Message as LlmMessage};

/// Service for processing RAG queries with streaming responses.
#[derive(Clone, Debug)]
pub struct QueryService {
    repo: QueryRepository,
    llm_client: LlmClient,
    embedding_client: EmbeddingClient,
    conversation_repo: ConversationRepository,
    max_history_messages: usize,
    context_token_budget: usize,
}

impl QueryService {
    /// Create a new QueryService.
    pub fn new(
        db: PgPool,
        chroma_url: &str,
        llm_client: LlmClient,
        embedding_service_url: &str,
        max_history_messages: usize,
        context_token_budget: usize,
    ) -> Self {
        let repo = QueryRepository::new(db.clone(), chroma_url);
        let embedding_client = EmbeddingClient::new(embedding_service_url);
        let conversation_repo = ConversationRepository::new(db);
        tracing::debug!("QueryService initialized");
        Self {
            repo,
            llm_client,
            embedding_client,
            conversation_repo,
            max_history_messages,
            context_token_budget,
        }
    }

    /// Process a query and return a stream of SSE events.
    ///
    /// Steps:
    /// 1. Embed the query via the embedding service
    /// 2. Search Chroma for the top-5 most similar chunks
    /// 3. Fetch full chunk data from PostgreSQL
    /// 4. Load conversation history (if session_id is provided)
    /// 5. Persist user message (if session_id is provided)
    /// 6. Stream the LLM response, yielding events: "chunk", "sources", "done"
    /// 7. Persist assistant message on done (if session_id is provided)
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
        let mut chroma_results = self
            .repo
            .query_chroma(&collection_name, &embedding, 5)
            .await
            .map_err(|e| {
                tracing::error!("Chroma search failed: {e}");
                e
            })?;

        if chroma_results.is_empty() {
            tracing::warn!(
                "No results found for query in collection {} — retrying up to 3 times",
                request.collection_id
            );
            for attempt in 1..=3 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                chroma_results = self
                    .repo
                    .query_chroma(&collection_name, &embedding, 5)
                    .await
                    .map_err(|e| {
                        tracing::error!("Chroma search failed on retry {attempt}: {e}");
                        e
                    })?;
                if !chroma_results.is_empty() {
                    tracing::info!(
                        "Chroma results found after retry {attempt} for collection {}",
                        request.collection_id
                    );
                    break;
                }
                tracing::warn!(
                    "Chroma still empty after retry {attempt} for collection {}",
                    request.collection_id
                );
            }
        }

        // 3. Fetch chunk text + document names from PostgreSQL
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

        // 5. Persist user message (if session is present) and get its ID
        let user_message_id = if let Some(session_id) = request.session_id {
            let user_msg_id = Uuid::new_v4();
            let msg = Message {
                id: user_msg_id,
                session_id,
                role: "user".to_string(),
                content: request.query.clone(),
                sources: None,
                created_at: chrono::Utc::now(),
                edited_at: None,
                original_content: None,
                deleted_at: None,
            };
            self.conversation_repo.add_message(&msg).await?;
            tracing::info!("[query.process_query] persisted user_message_id={user_msg_id}");

            // Auto-name session if title is still the default placeholder
            let session = self.conversation_repo.get_session(session_id).await?;
            if session.title == "New Chat" || session.title == "New Session" {
                let auto_title = request
                    .query
                    .chars()
                    .take(50)
                    .collect::<String>()
                    .trim()
                    .to_string();
                if !auto_title.is_empty() {
                    tracing::info!(
                        "[query.auto_name] session={session_id} auto-title=\"{auto_title}\""
                    );
                    self.conversation_repo
                        .update_session(session_id, Some(auto_title), None)
                        .await?;
                }
            }

            Some(user_msg_id)
        } else {
            None
        };

        // Pre-generate assistant message ID for the done event
        let assistant_message_id = request.session_id.map(|_| Uuid::new_v4());

        // 6. Stream LLM response
        let llm_stream = self
            .llm_client
            .query_stream(&request.query, &chunks, &history)
            .await?;

        let stream = Self::build_event_stream(
            llm_stream,
            source_refs,
            request.session_id,
            chunk_ids,
            user_message_id,
            assistant_message_id,
            self.conversation_repo.clone(),
        );

        Ok(stream)
    }

    /// Build the final event stream:
    ///   1. LLM text chunks → "chunk" events
    ///   2. Sources metadata → "sources" event
    ///   3. Completion signal → "done" event with message IDs
    fn build_event_stream(
        llm_stream: impl Stream<Item = Result<String, AppError>> + 'static,
        sources: Vec<SourceRef>,
        session_id: Option<Uuid>,
        _chunk_ids: Vec<String>,
        user_message_id: Option<Uuid>,
        assistant_message_id: Option<Uuid>,
        conversation_repo: ConversationRepository,
    ) -> impl Stream<Item = Result<StreamEvent, Infallible>> {
        let sources_event = StreamEvent {
            event_type: "sources".to_string(),
            data: json!({"sources": sources}),
        };

        // Track the full LLM output for persisting the assistant message
        let full_content: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
        let content_tracker = full_content.clone();

        let tracked_stream = llm_stream.map(move |result| {
            if let Ok(text) = &result {
                let mut content = content_tracker.lock().unwrap();
                content.push_str(text);
            }
            result
        });

        let done_event = {
            let repo = conversation_repo;
            let asst_id = assistant_message_id;
            let sid = session_id;
            let fc = full_content;

            stream::once(async move {
                // Persist the assistant message if we have both a session and a pre-generated ID
                if let (Some(session_id), Some(asst_id_val)) = (sid, asst_id) {
                    let content = fc.lock().unwrap().clone();
                    let msg = Message {
                        id: asst_id_val,
                        session_id,
                        role: "assistant".to_string(),
                        content,
                        sources: None,
                        created_at: chrono::Utc::now(),
                        edited_at: None,
                        original_content: None,
                        deleted_at: None,
                    };
                    if let Err(e) = repo.add_message(&msg).await {
                        tracing::error!(
                            "[query.process_query] failed to persist assistant message: {e}"
                        );
                    } else {
                        tracing::info!(
                            "[query.process_query] persisted assistant_message_id={asst_id_val}"
                        );
                    }
                }

                Ok(StreamEvent {
                    event_type: "done".to_string(),
                    data: json!({
                        "user_message_id": user_message_id,
                        "assistant_message_id": assistant_message_id,
                    }),
                })
            })
        };

        tracked_stream
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
            .chain(done_event)
    }

    /// Load conversation history for a session from the `messages` table.
    /// Excludes soft-deleted messages (deleted_at IS NOT NULL).
    /// Applies sliding-window and token-budget trimming via `context_window::trim_history`.
    async fn load_conversation_history(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<LlmMessage>, AppError> {
        let history = load_conversation_history_rows(self.repo.db(), session_id).await?;

        let count = history.len();

        // Apply context window trimming
        let (trimmed, dropped) = crate::modules::query::context_window::trim_history(
            &history,
            self.max_history_messages,
            self.context_token_budget,
        );

        let kept_tokens: usize = trimmed
            .iter()
            .map(|m| m.content.split_whitespace().count())
            .sum();

        tracing::info!(
            "[query.trim_history] session={session_id} in={count} out={} dropped={dropped} kept_tokens={kept_tokens} budget={} max_messages={}",
            trimmed.len(),
            self.context_token_budget,
            self.max_history_messages,
        );

        Ok(trimmed)
    }
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    role: String,
    content: String,
}

pub(crate) async fn load_conversation_history_rows(
    db: &PgPool,
    session_id: Uuid,
) -> Result<Vec<LlmMessage>, AppError> {
    tracing::debug!("[FIX] Loading conversation history with UUID bind: session={session_id}");

    let rows = sqlx::query_as::<_, MessageRow>(
        "SELECT role, content FROM messages \
         WHERE session_id = $1 AND deleted_at IS NULL \
         ORDER BY created_at ASC",
    )
    .bind(session_id)
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::error!(
            "[FIX] Failed to load conversation history: session={session_id}, error={e}"
        );
        AppError::InternalError(format!("Failed to load history: {e}"))
    })?;

    tracing::debug!(
        "[FIX] Loaded conversation history: session={session_id}, rows={}",
        rows.len()
    );

    Ok(rows
        .into_iter()
        .map(|r| LlmMessage {
            role: r.role,
            content: r.content,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::load_conversation_history_rows;
    use super::{AppError, QueryService, StreamEvent};
    use crate::modules::conversations::repository::ConversationRepository;
    use futures::stream;
    use futures::StreamExt;
    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    /// Connect to the shared test database.
    /// Expects migrations to already be applied (by Docker test container).
    async fn make_pool() -> sqlx::PgPool {
        let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://vedo:test-vedo-password@localhost:15432/vedo".to_string()
        });
        PgPoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[tokio::test]
    async fn load_conversation_history_binds_session_id_as_uuid() {
        let pool = make_pool().await;

        // Clean tables for a fresh state (sequential tests only)
        sqlx::query(
            "TRUNCATE TABLE git_repositories, messages, sessions, chunks, documents, collections CASCADE",
        )
        .execute(&pool)
        .await
        .expect("Failed to truncate tables");

        let session_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        sqlx::query(
            "INSERT INTO sessions (id, title, collection_id, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(session_id)
        .bind("UUID bind regression")
        .bind(Option::<Uuid>::None)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("insert session");

        sqlx::query(
            "INSERT INTO messages \
             (id, session_id, role, content, sources, created_at, edited_at, original_content, deleted_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(message_id)
        .bind(session_id)
        .bind("user")
        .bind("hello")
        .bind(serde_json::Value::Null)
        .bind(now)
        .bind(Option::<chrono::DateTime<chrono::Utc>>::None)
        .bind(Option::<String>::None)
        .bind(Option::<chrono::DateTime<chrono::Utc>>::None)
        .execute(&pool)
        .await
        .expect("insert message");

        let history = load_conversation_history_rows(&pool, session_id)
            .await
            .expect("history loads with uuid bind");

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].role, "user");
        assert_eq!(history[0].content, "hello");
    }

    #[tokio::test]
    async fn build_event_stream_does_not_panic_in_tokio_runtime() {
        let pool = make_pool().await;

        let llm_stream = Box::pin(stream::iter(vec![
            Ok::<String, AppError>("Hello ".to_string()),
            Ok::<String, AppError>("world".to_string()),
        ]));

        let repo = ConversationRepository::new(pool);

        let stream = Box::pin(QueryService::build_event_stream(
            llm_stream,
            vec![],
            None,
            vec![],
            None,
            None,
            repo,
        ));

        let events: Vec<StreamEvent> = stream.collect().await;

        // Expected: chunk "Hello ", chunk "world", sources, done
        assert_eq!(events.len(), 4, "should yield 4 events without panicking");
        assert_eq!(events[0].event_type, "chunk");
        assert_eq!(events[0].data["text"], "Hello ");
        assert_eq!(events[1].event_type, "chunk");
        assert_eq!(events[1].data["text"], "world");
        assert_eq!(events[2].event_type, "sources");
        assert_eq!(events[3].event_type, "done");
        assert_eq!(events[3].data["user_message_id"], serde_json::Value::Null);
        assert_eq!(
            events[3].data["assistant_message_id"],
            serde_json::Value::Null
        );
    }
}
