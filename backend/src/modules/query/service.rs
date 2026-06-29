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
use crate::modules::query::debug_models::{
    DebugData, EmbeddingSearchStep, HydeResult, HydeStep, KeywordSearchStep, MergeDedupStep,
    MergeSourceBreakdown, MultiQueryStep, RerankResult, RerankingStep, SearchResultItem,
};
use crate::modules::query::hybrid_service::merge_and_dedup;
use crate::modules::query::models::{
    PipelineMetricStep, PipelineStageEvent, QueryRequest, SourceRef, StreamEvent,
};
use crate::modules::query::query_enhancer::{generate_hyde, generate_multi_queries};
use crate::modules::query::repository::QueryRepository;
use crate::modules::query::reranker::rerank_chunks;
use crate::shared::embedding_client::EmbeddingClient;
use crate::shared::error::AppError;
use crate::shared::llm::{LlmClient, Message as LlmMessage};

/// Service for processing RAG queries with streaming responses.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct QueryService {
    // Fields: some are used now, others will be used in Phases 4-5.
    repo: QueryRepository,
    llm_client: LlmClient,
    embedding_client: EmbeddingClient,
    collection_repo: CollectionRepository,
    conversation_repo: ConversationRepository,
    max_history_messages: usize,
    context_token_budget: usize,
    advanced_rag_enabled: bool,
    rerank_top_k: usize,
    hybrid_top_k: usize,
    multi_query_count: usize,
    llm_rerank_model: String,
}

impl QueryService {
    /// Create a new QueryService.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: PgPool,
        chroma_url: &str,
        llm_client: LlmClient,
        embedding_service_url: &str,
        collection_repo: CollectionRepository,
        max_history_messages: usize,
        context_token_budget: usize,
        advanced_rag_enabled: bool,
        rerank_top_k: usize,
        hybrid_top_k: usize,
        multi_query_count: usize,
        llm_rerank_model: String,
    ) -> Self {
        let repo = QueryRepository::new(db.clone(), chroma_url);
        let embedding_client = EmbeddingClient::new(embedding_service_url);
        let conversation_repo = ConversationRepository::new(db);
        tracing::debug!(
            component = "query/service",
            advanced_rag_enabled,
            rerank_top_k,
            hybrid_top_k,
            multi_query_count,
            "service.initialized"
        );
        Self {
            repo,
            llm_client,
            embedding_client,
            collection_repo,
            conversation_repo,
            max_history_messages,
            context_token_budget,
            advanced_rag_enabled,
            rerank_top_k,
            hybrid_top_k,
            multi_query_count,
            llm_rerank_model,
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
            .embed(vec![request.query.clone()])
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

        // 2. Search Chroma for similar chunks
        let collection_name = request.collection_id.to_string();
        let mut chroma_results = self
            .repo
            .query_chroma(&collection_name, &embedding, 5)
            .await
            .map_err(|e| {
                tracing::error!(component = "query/service", error = %e, "query.chroma_search_failed");
                e
            })?;

        if chroma_results.is_empty() {
            tracing::warn!(
                component = "query/service",
                collection_id = %request.collection_id,
                "query.chroma.empty_results"
            );
            for attempt in 1..=3 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                chroma_results = self
                    .repo
                    .query_chroma(&collection_name, &embedding, 5)
                    .await
                    .map_err(|e| {
                        tracing::error!(component = "query/service", retry_attempt = attempt, error = %e, "query.chroma_retry_failed");
                        e
                    })?;
                if !chroma_results.is_empty() {
                    tracing::info!(
                        component = "query/service",
                        retry_attempt = attempt,
                        collection_id = %request.collection_id,
                        "query.chroma.retry_found"
                    );
                    break;
                }
                tracing::warn!(
                    component = "query/service",
                    retry_attempt = attempt,
                    collection_id = %request.collection_id,
                    "query.chroma.retry_empty"
                );
            }
        }

        // 3. Fetch chunk text + document names from PostgreSQL
        let chunk_ids: Vec<String> = chroma_results.iter().map(|r| r.id.clone()).collect();
        let chunks = self.repo.get_chunks_by_ids(&chunk_ids).await?;

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

        // 6. Run advanced pipeline or simple path
        let (pipeline_stages, source_refs, chunks, debug_data) =
            if self.advanced_rag_enabled && request.debug {
                tracing::info!(component = "query/service", "query.pipeline.advanced");
                let result = self
                    .run_advanced_pipeline(
                        &request.query,
                        &embedding,
                        &collection_name,
                        &history,
                        is_admin,
                    )
                    .await?;
                result
            } else {
                tracing::info!(component = "query/service", "query.pipeline.simple");
                // Simple path: use the existing Chroma results as-is
                let source_refs: Vec<SourceRef> = chroma_results
                    .iter()
                    .map(|r| {
                        let doc_name = chunks
                            .iter()
                            .find(|c| c.index == r.chunk_index)
                            .map(|c| c.document_name.clone())
                            .unwrap_or_default();
                        SourceRef {
                            document_id: Uuid::parse_str(&r.document_id).unwrap_or_default(),
                            document_name: doc_name,
                            chunk_index: r.chunk_index,
                            text: r.text.clone(),
                            relevance: r.score,
                            stage: Some("embedding".to_string()),
                            rerank_score: None,
                            rerank_verdict: None,
                            rerank_comment: None,
                            keyword_matches: None,
                        }
                    })
                    .collect();
                (vec![], source_refs, chunks, None)
            };

        // 7. Stream LLM response using the (possibly reranked) chunks
        let llm_stream = self
            .llm_client
            .query_stream(&request.query, &chunks, &history)
            .await?;

        // Prepend pipeline stage events to the stream
        let stream = Self::build_event_stream(
            llm_stream,
            source_refs,
            pipeline_stages,
            request.session_id,
            chunk_ids,
            user_message_id,
            assistant_message_id,
            self.conversation_repo.clone(),
            debug_data,
        );

        Ok(stream)
    }

    /// Run the full advanced RAG pipeline with multi-query, HyDE, BM25, merge, and reranking.
    ///
    /// Returns:
    /// - `pipeline_stages`: SSE events for each pipeline step
    /// - `source_refs`: final SourceRef list (from reranked or merged chunks)
    /// - `chunks`: `CrateChunkData` for the final LLM context
    /// - `debug_data`: optional DebugData for admin visualization
    #[allow(clippy::too_many_arguments)]
    async fn run_advanced_pipeline(
        &self,
        query: &str,
        _original_embedding: &[f32],
        collection_name: &str,
        _history: &[LlmMessage],
        _is_admin: bool,
    ) -> Result<
        (
            Vec<StreamEvent>,
            Vec<SourceRef>,
            Vec<crate::shared::llm::CrateChunkData>,
            Option<DebugData>,
        ),
        AppError,
    > {
        let mut pipeline_stages: Vec<StreamEvent> = Vec::new();
        let mut debug_data = DebugData::new(query);
        let pipeline_start = std::time::Instant::now();

        // ── Step 1: Multi-query expansion ──
        let mq_start = std::time::Instant::now();
        let multi_query =
            generate_multi_queries(&self.llm_client, query, self.multi_query_count).await;
        let _mq_latency = mq_start.elapsed().as_millis() as u64;
        debug_data.multi_query = Some(MultiQueryStep::new(
            query.to_string(),
            multi_query.variants.clone(),
            multi_query.latency_ms,
        ));
        pipeline_stages.push(StreamEvent {
            event_type: "pipeline_stage".to_string(),
            data: json!(PipelineStageEvent {
                stage: "expanded_questions".to_string(),
                data: json!({"variants": multi_query.variants, "count": multi_query.variants.len()}),
                latency_ms: multi_query.latency_ms,
            }),
        });

        // ── Step 2: HyDE generation for each variant ──
        let hyde_start = std::time::Instant::now();
        let mut hyde_results = Vec::new();
        for variant in &multi_query.variants {
            let result = generate_hyde(&self.llm_client, variant).await;
            hyde_results.push(HydeResult {
                query: result.query.clone(),
                hypothetical_doc: result.hypothetical_doc.clone(),
                latency_ms: result.latency_ms,
            });
        }
        debug_data.hyde = Some(HydeStep::new(hyde_results.clone()));
        let hyde_latency = hyde_start.elapsed().as_millis() as u64;
        pipeline_stages.push(StreamEvent {
            event_type: "pipeline_stage".to_string(),
            data: json!(PipelineStageEvent {
                stage: "hyde_docs".to_string(),
                data: json!({
                    "per_query": hyde_results.iter().map(|h| json!({
                        "query": h.query,
                        "doc_length": h.hypothetical_doc.len(),
                        "latency_ms": h.latency_ms,
                    })).collect::<Vec<_>>()
                }),
                latency_ms: hyde_latency,
            }),
        });

        // ── Step 3: Embed and search ──
        // Build texts to embed: original query + each HyDE hypothetical doc
        let embed_search_start = std::time::Instant::now();
        let mut all_embeddings = Vec::new();

        // Embed original query
        let original_embed = self
            .embedding_client
            .embed(vec![query.to_string()])
            .await
            .map_err(|e| {
                tracing::error!(component = "query/service", error = %e, "pipeline.embed_original_failed");
                e
            })?;
        if let Some(emb) = original_embed.into_iter().next() {
            all_embeddings.push(emb);
        }

        // Embed each HyDE doc
        for hyde in &hyde_results {
            let hyde_embed = self
                .embedding_client
                .embed(vec![hyde.hypothetical_doc.clone()])
                .await
                .map_err(|e| {
                    tracing::warn!(component = "query/service", error = %e, "pipeline.embed_hyde_failed");
                    e
                })?;
            if let Some(emb) = hyde_embed.into_iter().next() {
                all_embeddings.push(emb);
            }
        }

        // Search Chroma for each embedding and merge results
        let mut all_chroma_ids: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut all_chroma_results: Vec<crate::shared::types::ChromaResult> = Vec::new();
        for emb in &all_embeddings {
            let results = self
                .repo
                .query_chroma(collection_name, emb, 5)
                .await
                .unwrap_or_default();
            for r in results {
                if all_chroma_ids.insert(r.id.clone()) {
                    all_chroma_results.push(r);
                }
            }
        }
        let embed_search_latency = embed_search_start.elapsed().as_millis() as u64;

        debug_data.embedding_search = Some(EmbeddingSearchStep {
            query_snippet: query.chars().take(80).collect(),
            embedding_dimension: _original_embedding.len(),
            latency_ms: embed_search_latency,
            collection_name: collection_name.to_string(),
            top_k: 5,
            result_count: all_chroma_results.len(),
            retries: 0,
            results: all_chroma_results
                .iter()
                .map(|r| SearchResultItem {
                    chunk_id: r.id.clone(),
                    document_name: String::new(),
                    chunk_index: r.chunk_index,
                    score: r.score,
                    text_snippet: r.text.chars().take(120).collect(),
                })
                .collect(),
        });

        // Fetch full chunk data for all Chroma results
        let chroma_chunk_ids: Vec<String> =
            all_chroma_results.iter().map(|r| r.id.clone()).collect();
        let pg_chunks = self.repo.get_chunks_by_ids(&chroma_chunk_ids).await?;

        // ── Step 4: BM25 keyword search ──
        let bm25_start = std::time::Instant::now();
        let bm25_results = QueryRepository::bm25_search(&pg_chunks, query, self.hybrid_top_k);
        let bm25_latency = bm25_start.elapsed().as_millis() as u64;

        let query_tokens: Vec<String> = crate::shared::bm25::tokenize(query);
        let bm25_debug_results: Vec<SearchResultItem> = bm25_results
            .iter()
            .map(|r| {
                let idx: usize = r.doc_id.parse().unwrap_or(0);
                let chunk = pg_chunks.get(idx);
                SearchResultItem {
                    chunk_id: r.doc_id.clone(),
                    document_name: chunk.map(|c| c.document_name.clone()).unwrap_or_default(),
                    chunk_index: idx,
                    score: r.score,
                    text_snippet: r.text.chars().take(120).collect(),
                }
            })
            .collect();
        debug_data.keyword_search = Some(KeywordSearchStep::new(
            query_tokens.clone(),
            bm25_results.len(),
            bm25_debug_results,
            bm25_latency,
        ));
        pipeline_stages.push(StreamEvent {
            event_type: "pipeline_stage".to_string(),
            data: json!(PipelineStageEvent {
                stage: "keyword_matches".to_string(),
                data: json!({
                    "query_tokens": query_tokens,
                    "total_matches": bm25_results.len(),
                    "latency_ms": bm25_latency,
                }),
                latency_ms: bm25_latency,
            }),
        });

        // Convert Chroma results to MergedChunks for merge_and_dedup
        let chroma_for_merge: Vec<crate::shared::types::ChromaResult> = all_chroma_results;

        // ── Step 5: Merge + dedup ──
        let merge_start = std::time::Instant::now();
        let merged = merge_and_dedup(chroma_for_merge, bm25_results);
        let merge_latency = merge_start.elapsed().as_millis() as u64;

        let vector_count = merged.iter().filter(|m| m.vector_score.is_some()).count();
        let keyword_count = merged.iter().filter(|m| m.vector_score.is_none()).count();

        debug_data.merge_dedup = Some(MergeDedupStep::new(
            merged.len(),
            merged.len(),
            MergeSourceBreakdown {
                vector_chunks: vector_count,
                keyword_chunks: keyword_count,
            },
        ));
        pipeline_stages.push(StreamEvent {
            event_type: "pipeline_stage".to_string(),
            data: json!(PipelineStageEvent {
                stage: "merged_chunks".to_string(),
                data: json!({
                    "input_chunks": merged.len(),
                    "after_dedup": merged.len(),
                    "source_breakdown": {
                        "vector_chunks": vector_count,
                        "keyword_chunks": keyword_count,
                    },
                }),
                latency_ms: merge_latency,
            }),
        });

        // ── Step 6: LLM reranking ──
        let rerank_start = std::time::Instant::now();
        let reranked = rerank_chunks(
            &self.llm_client,
            &self.llm_rerank_model,
            query,
            &merged,
            self.rerank_top_k,
        )
        .await
        .unwrap_or_default();
        let rerank_latency = rerank_start.elapsed().as_millis() as u64;

        let accepted = reranked.len();
        let rejected = merged.len().saturating_sub(accepted);
        debug_data.reranking = Some(RerankingStep::new(
            merged.len(),
            accepted,
            rejected,
            reranked
                .iter()
                .map(|r| {
                    RerankResult::new(
                        r.chunk.chunk_id.clone(),
                        r.verdict.score,
                        r.verdict.verdict.clone(),
                        r.verdict.comment.clone(),
                    )
                })
                .collect(),
        ));
        pipeline_stages.push(StreamEvent {
            event_type: "pipeline_stage".to_string(),
            data: json!(PipelineStageEvent {
                stage: "reranked_chunks".to_string(),
                data: json!({
                    "input_count": merged.len(),
                    "accepted": accepted,
                    "rejected": rejected,
                }),
                latency_ms: rerank_latency,
            }),
        });

        // ── Build final context from reranked chunks ──
        // Accepted reranked chunks become the final context
        let reranked_chunk_ids: Vec<String> =
            reranked.iter().map(|r| r.chunk.chunk_id.clone()).collect();

        // Fetch full chunk data for reranked chunks
        let final_chunks = if !reranked_chunk_ids.is_empty() {
            self.repo.get_chunks_by_ids(&reranked_chunk_ids).await?
        } else {
            // Fallback: use PG chunks from Chroma results
            pg_chunks
        };

        // Build SourceRefs with reranking metadata
        let reranked_refs: Vec<SourceRef> = reranked
            .iter()
            .map(|r| {
                let doc_id = Uuid::parse_str(&r.chunk.document_id).unwrap_or_default();
                SourceRef {
                    document_id: doc_id,
                    document_name: r.chunk.document_name.clone(),
                    chunk_index: r.chunk.chunk_index,
                    text: r.chunk.text.clone(),
                    relevance: r.verdict.score as f64,
                    stage: Some("reranked".to_string()),
                    rerank_score: Some(r.verdict.score as f64),
                    rerank_verdict: Some(r.verdict.verdict.clone()),
                    rerank_comment: Some(r.verdict.comment.clone()),
                    keyword_matches: None,
                }
            })
            .collect();

        // Also include BM25 keyword sources
        let bm25_refs: Vec<SourceRef> = if !reranked_refs.is_empty() {
            vec![]
        } else {
            merged
                .iter()
                .filter(|m| m.vector_score.is_none())
                .map(|m| SourceRef {
                    document_id: Uuid::nil(),
                    document_name: m.document_name.clone(),
                    chunk_index: m.chunk_index,
                    text: m.text.clone(),
                    relevance: m.keyword_score.unwrap_or(0.0),
                    stage: Some("keyword".to_string()),
                    rerank_score: None,
                    rerank_verdict: None,
                    rerank_comment: None,
                    keyword_matches: Some(query_tokens.clone()),
                })
                .collect()
        };

        let final_source_refs = if !reranked_refs.is_empty() {
            reranked_refs
        } else {
            bm25_refs
        };

        // ── Pipeline metric event ──
        let total_ms = pipeline_start.elapsed().as_millis() as u64;
        let metric = PipelineMetricStep {
            total_ms,
            multi_query_ms: multi_query.latency_ms,
            hyde_ms: hyde_latency,
            embedding_search_ms: embed_search_latency,
            keyword_search_ms: bm25_latency,
            merge_dedup_ms: merge_latency,
            reranking_ms: rerank_latency,
            final_answer_ms: 0, // filled after LLM answer
        };
        pipeline_stages.push(StreamEvent {
            event_type: "pipeline_stage".to_string(),
            data: json!(PipelineStageEvent {
                stage: "pipeline_metric".to_string(),
                data: json!(metric),
                latency_ms: total_ms,
            }),
        });

        tracing::info!(
            component = "query/service",
            total_ms,
            multi_query_ms = multi_query.latency_ms,
            hyde_ms = hyde_latency,
            embed_search_ms = embed_search_latency,
            bm25_ms = bm25_latency,
            merge_ms = merge_latency,
            rerank_ms = rerank_latency,
            source_count = final_source_refs.len(),
            "pipeline.complete"
        );

        Ok((
            pipeline_stages,
            final_source_refs,
            final_chunks,
            Some(debug_data),
        ))
    }

    /// Build the final event stream:
    ///   1. Pipeline stage events (advanced RAG)
    ///   2. LLM text chunks → "chunk" events
    ///   3. Sources metadata → "sources" event
    ///   4. Completion signal → "done" event with message IDs and debug data
    #[allow(clippy::too_many_arguments)]
    fn build_event_stream(
        llm_stream: impl Stream<Item = Result<String, AppError>> + 'static,
        sources: Vec<SourceRef>,
        pipeline_stages: Vec<StreamEvent>,
        session_id: Option<Uuid>,
        _chunk_ids: Vec<String>,
        user_message_id: Option<Uuid>,
        assistant_message_id: Option<Uuid>,
        conversation_repo: ConversationRepository,
        debug_data: Option<DebugData>,
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
            let dd = debug_data.clone();

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
                        debug_data: dd
                            .as_ref()
                            .map(|d| serde_json::to_string(d).unwrap_or_default()),
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

                let mut done_payload = json!({
                    "user_message_id": user_message_id,
                    "assistant_message_id": assistant_message_id,
                });
                if let Some(ref dd) = dd {
                    done_payload["debug_data"] = serde_json::to_value(dd).unwrap_or_default();
                }

                Ok(StreamEvent {
                    event_type: "done".to_string(),
                    data: done_payload,
                })
            })
        };

        // Pipeline events → tracked stream → sources → done
        let pipeline_stream = stream::iter(pipeline_stages.into_iter().map(Ok));

        pipeline_stream
            .chain(tracked_stream.map(|result| match result {
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
            }))
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
            vec![], // pipeline_stages (empty for simple path)
            None,
            vec![],
            None,
            None,
            repo,
            None, // debug_data
        ));

        let events: Vec<StreamEvent> = stream
            .filter_map(|r| futures::future::ready(r.ok()))
            .collect()
            .await;

        // Expected: chunk "Hello ", chunk "world", sources, done
        // (pipeline_stages is empty so it doesn't add extra events)
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
