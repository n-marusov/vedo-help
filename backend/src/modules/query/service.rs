use std::convert::Infallible;
use std::sync::Arc;

use futures::stream::{self, Stream};
use futures::StreamExt;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Mutex;
use uuid::Uuid;

use crate::modules::collections::repository::CollectionRepository;
use crate::modules::conversations::models::Message;
use crate::modules::conversations::repository::ConversationRepository;
use crate::modules::query::models::{QueryRequest, SourceRef, StreamEvent};
use crate::modules::query::repository::QueryRepository;
use crate::modules::settings::service::SettingsService;
use crate::shared::chunk_search;
use crate::shared::embedding_client::EmbeddingClient;
use crate::shared::error::AppError;
use crate::shared::llm::{LlmClient, Message as LlmMessage};

/// Service for processing RAG queries with streaming responses.
#[derive(Clone, Debug)]
pub struct QueryService {
    repo: QueryRepository,
    llm_client: LlmClient,
    embedding_client: EmbeddingClient,
    collection_repo: CollectionRepository,
    conversation_repo: ConversationRepository,
    max_history_messages: usize,
    context_token_budget: usize,
    pub config: crate::config::AppConfig,
    settings_service: Option<SettingsService>,
}

impl QueryService {
    /// Create a new QueryService.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: PgPool,
        chroma_url: &str,
        llm_client: LlmClient,
        embedding_client: EmbeddingClient,
        collection_repo: CollectionRepository,
        max_history_messages: usize,
        context_token_budget: usize,
        config: crate::config::AppConfig,
        settings_service: Option<SettingsService>,
    ) -> Self {
        let repo = QueryRepository::new(db.clone(), chroma_url);
        let conversation_repo = ConversationRepository::new(db);
        tracing::debug!(component = "query/service", "service.initialized");
        Self {
            repo,
            llm_client,
            embedding_client,
            collection_repo,
            conversation_repo,
            max_history_messages,
            context_token_budget,
            config,
            settings_service,
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
        user_id: &str,
        is_admin: bool,
    ) -> Result<impl Stream<Item = Result<StreamEvent, Infallible>>, AppError> {
        tracing::info!(
            component = "query/service",
            collection_id = %request.collection_id,
            session_id = ?request.session_id,
            query_length = request.query.len(),
            "query.process.start"
        );

        // 0. Verify collection ownership
        self.collection_repo
            .get_collection_for_user(request.collection_id, user_id, is_admin)
            .await?;

        // 1. Embed the query
        tracing::debug!(component = "query/service", "query.embed.start");
        let embeddings = self
            .embedding_client
            .embed(&self.config.embedding_model, vec![request.query.clone()])
            .await
            .map_err(|e| {
                tracing::error!(component = "query/service", error = %e, "query.embed.failed");
                e
            })?;

        let embedding = embeddings.into_iter().next().ok_or_else(|| {
            let err =
                AppError::EmbeddingError("Embedding service returned empty result".to_string());
            tracing::error!(component = "query/service", "query.embed.empty_result");
            err
        })?;

        tracing::debug!(
            component = "query/service",
            embedding_dimension = embedding.len(),
            "query.embedded"
        );

        // 2. Load effective RAG settings (DB overrides with env fallback)
        let rag_settings = if let Some(ref svc) = self.settings_service {
            svc.get_rag_settings().await.unwrap_or_else(|_| {
                tracing::warn!(
                    component = "query/service",
                    "settings.load.failed, using env defaults"
                );
                crate::modules::settings::models::RagSettings::default()
                    .with_env_overrides(&self.config)
            })
        } else {
            crate::modules::settings::models::RagSettings::default()
                .with_env_overrides(&self.config)
        };
        let advanced_rag_enabled = rag_settings.advanced_rag_enabled;
        let multi_query_enabled = rag_settings.multi_query_enabled;
        let hyde_enabled = rag_settings.hyde_enabled;
        let bm25_enabled = rag_settings.bm25_enabled;
        let reranking_enabled = rag_settings.reranking_enabled;
        let eff_rerank_top_k = rag_settings.rerank_top_k;
        let eff_hybrid_top_k = rag_settings.hybrid_top_k;
        let eff_multi_query_count = rag_settings.multi_query_count;

        use crate::modules::query::debug_models::*;
        let mut debug_data = DebugData::new(&request.query);

        // 3. Advanced RAG logic
        let final_chunks: Vec<crate::shared::chunk_search::ChunkSearchResult>;

        if !advanced_rag_enabled {
            // Standard semantic search fallback
            let start = tokio::time::Instant::now();
            let chunk_search_results = chunk_search::search_chunks_semantic(
                self.repo.chroma(),
                &self.embedding_client,
                self.repo.db(),
                request.collection_id,
                &request.query,
                None,
                eff_rerank_top_k,
                &self.config.embedding_model,
            )
            .await
            .map_err(|e| {
                tracing::error!(component = "query/service", error = %e, "query.chunk_search_failed");
                e
            })?;

            debug_data.embedding_search = Some(EmbeddingSearchStep {
                query_snippet: request.query.clone(),
                embedding_dimension: 384,
                latency_ms: start.elapsed().as_millis() as u64,
                collection_name: request.collection_id.to_string(),
                top_k: eff_rerank_top_k,
                result_count: chunk_search_results.len(),
                retries: 0,
                results: chunk_search_results
                    .iter()
                    .map(|r| SearchResultItem {
                        chunk_id: r.chunk_id.to_string(),
                        document_name: r.document_name.clone(),
                        chunk_index: r.chunk_index,
                        score: r.score.unwrap_or(0.0),
                        text_snippet: r.text.chars().take(200).collect(),
                    })
                    .collect(),
            });

            final_chunks = chunk_search_results;
        } else {
            // ── Composed Pipeline with Per-Stage Gates ──

            // Build the list of queries to search:
            //   - If Multi-Query is enabled, generate LLM variants
            //   - Otherwise, use only the original query
            let mut queries: Vec<String> = vec![request.query.clone()];

            if multi_query_enabled {
                let mq_start = tokio::time::Instant::now();
                let mq_system = "You are an AI language model assistant. Your task is to generate 3 different versions of the given user question to retrieve relevant documents from a vector database. By generating multiple perspectives on the user question, your goal is to help the user overcome some of the limitations of the distance-based similarity search. Provide these alternative questions separated by newlines.";

                let variants_text = self
                    .llm_client
                    .query_single(mq_system, &request.query)
                    .await
                    .unwrap_or_default();
                let mut variants: Vec<String> = variants_text
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if variants.is_empty() {
                    variants.push(request.query.clone());
                }
                if variants.len() > eff_multi_query_count {
                    variants.truncate(eff_multi_query_count);
                }

                queries.extend(variants);

                debug_data.multi_query = Some(MultiQueryStep {
                    original_query: request.query.clone(),
                    variants: queries[1..].to_vec(),
                    latency_ms: mq_start.elapsed().as_millis() as u64,
                });
            }

            // HyDE: for each query, optionally generate a hypothetical document,
            // then search semantically
            let embed_start = tokio::time::Instant::now();
            let mut all_semantic_chunks = Vec::new();
            let mut hyde_results = Vec::new();

            for q in &queries {
                let hyde_doc = if hyde_enabled {
                    let doc_start = tokio::time::Instant::now();
                    let hyde_system = "Please write a short hypothetical document that answers the question. The document should be factual and concise.";
                    let doc = self
                        .llm_client
                        .query_single(hyde_system, q)
                        .await
                        .unwrap_or_default();
                    hyde_results.push(HydeResult {
                        query: q.clone(),
                        hypothetical_doc: doc.clone(),
                        latency_ms: doc_start.elapsed().as_millis() as u64,
                    });
                    format!("{} {}", q, doc)
                } else {
                    q.clone()
                };

                let batch_results = chunk_search::search_chunks_semantic(
                    self.repo.chroma(),
                    &self.embedding_client,
                    self.repo.db(),
                    request.collection_id,
                    &hyde_doc,
                    None,
                    eff_hybrid_top_k,
                    &self.config.embedding_model,
                )
                .await
                .unwrap_or_default();

                all_semantic_chunks.extend(batch_results);
            }

            if hyde_enabled {
                debug_data.hyde = Some(HydeStep {
                    per_query: hyde_results,
                });
            }

            debug_data.embedding_search = Some(EmbeddingSearchStep {
                query_snippet: if multi_query_enabled {
                    "Multi-query with HyDE".to_string()
                } else {
                    "Single query".to_string()
                },
                embedding_dimension: 384,
                latency_ms: embed_start.elapsed().as_millis() as u64,
                collection_name: request.collection_id.to_string(),
                top_k: eff_hybrid_top_k,
                result_count: all_semantic_chunks.len(),
                retries: 0,
                results: all_semantic_chunks
                    .iter()
                    .take(5)
                    .map(|r| SearchResultItem {
                        chunk_id: r.chunk_id.to_string(),
                        document_name: r.document_name.clone(),
                        chunk_index: r.chunk_index,
                        score: r.score.unwrap_or(0.0),
                        text_snippet: r.text.chars().take(200).collect(),
                    })
                    .collect(),
            });

            // BM25 (Keyword Search) — gated
            let bm25_chunks = if bm25_enabled {
                let bm25_start = tokio::time::Instant::now();
                let results = chunk_search::search_chunks_text(
                    self.repo.db(),
                    request.collection_id,
                    &request.query,
                    None,
                    eff_hybrid_top_k,
                    0,
                )
                .await
                .unwrap_or_default();

                debug_data.keyword_search = Some(KeywordSearchStep {
                    query_tokens: crate::shared::bm25::tokenize(&request.query),
                    total_matches: results.len(),
                    results: results
                        .iter()
                        .take(5)
                        .map(|r| SearchResultItem {
                            chunk_id: r.chunk_id.to_string(),
                            document_name: r.document_name.clone(),
                            chunk_index: r.chunk_index,
                            score: 0.0,
                            text_snippet: r.text.chars().take(200).collect(),
                        })
                        .collect(),
                    latency_ms: bm25_start.elapsed().as_millis() as u64,
                });

                results
            } else {
                Vec::new()
            };

            // Merge & Dedup
            let initial_count = all_semantic_chunks.len() + bm25_chunks.len();
            let mut unique_chunks = std::collections::HashMap::new();

            for c in all_semantic_chunks.clone() {
                unique_chunks.insert(c.chunk_id.to_string(), c);
            }
            for c in bm25_chunks.clone() {
                unique_chunks.insert(c.chunk_id.to_string(), c);
            }

            let merged: Vec<_> = unique_chunks.into_values().collect();

            debug_data.merge_dedup = Some(MergeDedupStep {
                input_chunks: initial_count,
                after_dedup: merged.len(),
                source_breakdown: MergeSourceBreakdown {
                    vector_chunks: all_semantic_chunks.len(),
                    keyword_chunks: bm25_chunks.len(),
                },
            });

            // LLM Reranking — gated
            let final_merged = if reranking_enabled {
                let rerank_system = "You are an expert relevance ranker. Given a user question and a document chunk, evaluate if the chunk contains information that helps answer the question. If it is relevant and should be kept, respond with the exact word 'брать'. If it is completely irrelevant, respond with 'пропустить'. Do not provide any other text or explanation. Question: ";

                let mut accepted_chunks = Vec::new();
                let mut rerank_results = Vec::new();

                for chunk in merged {
                    let prompt = format!("{}\n\nChunk: {}", request.query, chunk.text);
                    let verdict = self
                        .llm_client
                        .query_single(rerank_system, &prompt)
                        .await
                        .unwrap_or_else(|_| "брать".to_string());
                    let keep = verdict.to_lowercase().contains("брать");

                    rerank_results.push(RerankResult {
                        chunk_id: chunk.chunk_id.to_string(),
                        score: if keep { 1.0 } else { 0.0 },
                        verdict: if keep {
                            "брать".to_string()
                        } else {
                            "пропустить".to_string()
                        },
                        comment: verdict,
                    });

                    if keep {
                        accepted_chunks.push(chunk);
                    }
                }

                let accepted_count = accepted_chunks.len();
                let rejected_count = rerank_results.len() - accepted_count;

                if accepted_chunks.len() > eff_rerank_top_k {
                    accepted_chunks.truncate(eff_rerank_top_k);
                }

                debug_data.reranking = Some(RerankingStep {
                    input_count: rerank_results.len(),
                    accepted: accepted_count,
                    rejected: rejected_count,
                    results: rerank_results,
                });

                accepted_chunks
            } else {
                let mut result = merged;
                if result.len() > eff_rerank_top_k {
                    result.truncate(eff_rerank_top_k);
                }
                result
            };

            final_chunks = final_merged;
        }

        // Build chunks for LLM context (CrateChunkData) and SourceRefs from search results
        let chunks: Vec<crate::shared::llm::CrateChunkData> = final_chunks
            .iter()
            .map(|r| crate::shared::llm::CrateChunkData {
                text: r.text.clone(),
                index: r.chunk_index,
                document_name: r.document_name.clone(),
            })
            .collect();

        let source_refs: Vec<SourceRef> = final_chunks
            .iter()
            .map(|r| SourceRef {
                document_id: r.document_id,
                document_name: r.document_name.clone(),
                chunk_index: r.chunk_index,
                text: r.text.clone(),
                relevance: r.score.unwrap_or(0.0),
                stage: None,
                rerank_score: None,
                rerank_verdict: None,
            })
            .collect();

        let chunk_ids: Vec<String> = final_chunks
            .iter()
            .map(|r| r.chunk_id.to_string())
            .collect();

        let debug_data_json = serde_json::to_string(&debug_data).unwrap_or_default();

        // 4. Load conversation history if session is present
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
                debug_data: None,
            };
            self.conversation_repo.add_message(&msg).await?;
            tracing::info!(component = "query/service", user_message_id = %user_msg_id, "query.user_message_persisted");

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
                        component = "query/service",
                        session_id = %session_id,
                        auto_title = %auto_title,
                        "query.session.auto_named"
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
            debug_data_json,
        );

        Ok(stream)
    }

    /// Build the final event stream:
    ///   1. LLM text chunks → "chunk" events
    ///   2. Sources metadata → "sources" event
    ///   3. Completion signal → "done" event with message IDs
    #[allow(clippy::too_many_arguments)]
    fn build_event_stream(
        llm_stream: impl Stream<Item = Result<String, AppError>> + 'static,
        sources: Vec<SourceRef>,
        session_id: Option<Uuid>,
        _chunk_ids: Vec<String>,
        user_message_id: Option<Uuid>,
        assistant_message_id: Option<Uuid>,
        conversation_repo: ConversationRepository,
        debug_data_json: String,
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
                        sources: Some(serde_json::to_string(&sources).unwrap_or_default()),
                        created_at: chrono::Utc::now(),
                        edited_at: None,
                        original_content: None,
                        deleted_at: None,
                        debug_data: Some(debug_data_json),
                    };
                    if let Err(e) = repo.add_message(&msg).await {
                        tracing::error!(
                            component = "query/service",
                            error = %e,
                            "query.assistant_message_persist_failed"
                        );
                    } else {
                        tracing::info!(
                            component = "query/service",
                            assistant_message_id = %asst_id_val,
                            "query.assistant_message_persisted"
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
                    tracing::error!(component = "query/service", error = %e, "llm.stream_error");
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
            component = "query/service",
            session_id = %session_id,
            history_count = count,
            trimmed_count = trimmed.len(),
            dropped_count = dropped,
            kept_tokens = kept_tokens,
            token_budget = self.context_token_budget,
            max_messages = self.max_history_messages,
            "query.trim_history.result"
        );

        Ok(trimmed)
    }
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    role: String,
    content: String,
}

pub async fn load_conversation_history_rows(
    db: &PgPool,
    session_id: Uuid,
) -> Result<Vec<LlmMessage>, AppError> {
    tracing::debug!(component = "query/service", session_id = %session_id, "query.load_conversation_history.start");

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
            component = "query/service",
            session_id = %session_id,
            error = %e,
            "query.load_conversation_history.failed"
        );
        AppError::InternalError(format!("Failed to load history: {e}"))
    })?;

    tracing::debug!(
        component = "query/service",
        session_id = %session_id,
        row_count = rows.len(),
        "query.load_conversation_history.found"
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
    use super::{AppError, QueryService, StreamEvent};
    use crate::modules::conversations::repository::ConversationRepository;
    use futures::stream;
    use futures::StreamExt;
    use sqlx::postgres::PgPoolOptions;

    /// Pure unit test: `build_event_stream` with all `None` IDs never touches the DB.
    /// Uses `connect_lazy` so no PostgreSQL is needed.
    #[tokio::test]
    async fn build_event_stream_does_not_panic_in_tokio_runtime() {
        // Lazy pool — never connects because the stream skips DB calls
        // when session_id, user_message_id, and assistant_message_id are all None.
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost/nonexistent")
            .expect("lazy pool creation should not require a running DB");

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
            "null".to_string(),
        ));

        let events: Vec<StreamEvent> = stream
            .filter_map(|r| futures::future::ready(r.ok()))
            .collect()
            .await;

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
