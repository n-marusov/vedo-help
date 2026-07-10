use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use futures::stream::{self, StreamExt};
use futures::Stream;
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio_stream::wrappers::ReceiverStream;
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
use crate::shared::llm::{LlmClient, Message as LlmMessage, MAX_RETRIES};

/// Status of an active RAG pipeline job, used by the SSE recovery endpoint.
#[derive(Debug, Clone)]
pub enum JobStatus {
    Running { stage: Option<String> },
    Done,
    Failed(String),
}

type ActiveJobMap = HashMap<uuid::Uuid, (uuid::Uuid, watch::Sender<JobStatus>)>;

/// Registry of in-flight RAG pipeline jobs indexed by session ID.
/// Each entry holds a `watch::Sender` so that the SSE subscribe handler
/// can wait for job completion without polling the database.
#[derive(Clone, Debug)]
pub struct ActiveJobRegistry {
    inner: Arc<Mutex<ActiveJobMap>>,
}

impl ActiveJobRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new pipeline job and return its ID plus a receiver that will be notified
    /// when the job completes.
    pub fn register(&self, session_id: uuid::Uuid) -> (uuid::Uuid, watch::Receiver<JobStatus>) {
        let job_id = uuid::Uuid::new_v4();
        let (tx, rx) = watch::channel(JobStatus::Running { stage: None });
        self.inner.lock().unwrap().insert(session_id, (job_id, tx));
        tracing::debug!(
            component = "query/service",
            session_id = %session_id,
            job_id = %job_id,
            "active_job.registered"
        );
        (job_id, rx)
    }

    fn current_sender(
        &self,
        session_id: uuid::Uuid,
        job_id: uuid::Uuid,
    ) -> Option<watch::Sender<JobStatus>> {
        self.inner
            .lock()
            .unwrap()
            .get(&session_id)
            .and_then(|(current_job_id, tx)| {
                if *current_job_id == job_id {
                    Some(tx.clone())
                } else {
                    None
                }
            })
    }

    /// Return whether the given job is still the latest pipeline for this session.
    pub fn is_current(&self, session_id: uuid::Uuid, job_id: uuid::Uuid) -> bool {
        self.inner
            .lock()
            .unwrap()
            .get(&session_id)
            .is_some_and(|(current_job_id, _)| *current_job_id == job_id)
    }

    /// Publish the latest visible pipeline stage for recovery subscribers.
    pub fn update_stage(&self, session_id: uuid::Uuid, job_id: uuid::Uuid, stage: String) {
        if let Some(tx) = self.current_sender(session_id, job_id) {
            let _ = tx.send(JobStatus::Running {
                stage: Some(stage.clone()),
            });
            tracing::debug!(
                component = "query/service",
                session_id = %session_id,
                job_id = %job_id,
                stage = stage,
                "active_job.stage_updated"
            );
        }
    }

    /// Mark a job as completed successfully.
    pub fn complete(&self, session_id: uuid::Uuid, job_id: uuid::Uuid) {
        if let Some(tx) = self.current_sender(session_id, job_id) {
            let _ = tx.send(JobStatus::Done);
            tracing::debug!(
                component = "query/service",
                session_id = %session_id,
                job_id = %job_id,
                "active_job.completed"
            );
        }
    }

    /// Mark a job as failed.
    pub fn fail(&self, session_id: uuid::Uuid, job_id: uuid::Uuid, error: String) {
        if let Some(tx) = self.current_sender(session_id, job_id) {
            let _ = tx.send(JobStatus::Failed(error));
            tracing::debug!(
                component = "query/service",
                session_id = %session_id,
                job_id = %job_id,
                "active_job.failed"
            );
        }
    }

    /// Get a receiver for an existing job. Returns `None` if no job is registered
    /// for the given session ID.
    pub fn get_receiver(&self, session_id: uuid::Uuid) -> Option<watch::Receiver<JobStatus>> {
        self.inner
            .lock()
            .unwrap()
            .get(&session_id)
            .map(|(_, tx)| tx.subscribe())
    }
}

impl Default for ActiveJobRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Service for processing RAG queries with streaming responses.
#[derive(Clone, Debug)]
pub struct QueryService {
    repo: QueryRepository,
    llm_client: LlmClient,
    embedding_client: EmbeddingClient,
    collection_repo: CollectionRepository,
    pub conversation_repo: ConversationRepository,
    max_history_messages: usize,
    context_token_budget: usize,
    pub config: crate::config::AppConfig,
    settings_service: Option<SettingsService>,
    pub active_jobs: ActiveJobRegistry,
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
            active_jobs: ActiveJobRegistry::new(),
        }
    }

    /// Process a query and return a stream of SSE events.
    ///
    /// Emits pipeline_stage events as each RAG pipeline step completes,
    /// then streams the LLM response as chunk/sources/done events.
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

        // 1. Load effective RAG settings
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

        // Create channel and spawn pipeline in background task.
        // The SSE response starts immediately — stage events arrive as each
        // pipeline step completes, giving the frontend real-time progress.
        let (tx, rx) = mpsc::channel::<Result<StreamEvent, Infallible>>(64);
        let svc = self.clone();
        let owned_user_id = user_id.to_string();

        // Register in active jobs registry so the SSE subscribe endpoint
        // can be notified when the pipeline completes (recovery after reload).
        let request_session_id = request.session_id;
        let request_job_id = request_session_id.map(|sid| svc.active_jobs.register(sid).0);

        tokio::spawn(async move {
            let pipeline_result = Self::run_pipeline(
                tx.clone(),
                svc.clone(),
                request,
                owned_user_id,
                is_admin,
                request_job_id,
                rag_settings,
            )
            .await;

            match pipeline_result {
                Ok(()) => {
                    if let (Some(sid), Some(job_id)) = (request_session_id, request_job_id) {
                        svc.active_jobs.complete(sid, job_id);
                    }
                }
                Err(e) => {
                    tracing::error!(
                        component = "query/service",
                        error = %e,
                        "query.pipeline.failed"
                    );
                    if let (Some(sid), Some(job_id)) = (request_session_id, request_job_id) {
                        svc.active_jobs.fail(sid, job_id, e.to_string());
                    }
                    tx.send(Ok(StreamEvent {
                        event_type: "error".to_string(),
                        data: json!({"text": e.to_string()}),
                    }))
                    .await
                    .ok();
                }
            }
        });

        Ok(ReceiverStream::new(rx))
    }

    /// Execute the full RAG pipeline and send events through the channel.
    ///
    /// Stage events are sent in real-time as each step completes, allowing the
    /// frontend to display progress before the LLM response begins streaming.
    #[allow(clippy::too_many_arguments)]
    async fn run_pipeline(
        tx: mpsc::Sender<Result<StreamEvent, Infallible>>,
        svc: QueryService,
        request: QueryRequest,
        _user_id: String,
        _is_admin: bool,
        job_id: Option<Uuid>,
        rag_settings: crate::modules::settings::models::RagSettings,
    ) -> Result<(), AppError> {
        use crate::modules::query::debug_models::*;

        let pipeline_start = tokio::time::Instant::now();
        let advanced_rag_enabled = rag_settings.advanced_rag_enabled;
        let multi_query_enabled = rag_settings.multi_query_enabled;
        let hyde_enabled = rag_settings.hyde_enabled;
        let bm25_enabled = rag_settings.bm25_enabled;
        let reranking_enabled = rag_settings.reranking_enabled;
        let eff_rerank_top_k = rag_settings.rerank_top_k;
        let eff_hybrid_top_k = rag_settings.hybrid_top_k;
        let eff_multi_query_count = rag_settings.multi_query_count;
        let eff_embedding_model = rag_settings.embedding_model.clone();

        // Helper to send a stage event through the channel and publish it to
        // the active job registry for browser-reload SSE recovery subscribers.
        let send_stage = |name: String| {
            let tx = tx.clone();
            let active_jobs = svc.active_jobs.clone();
            let session_id = request.session_id;
            async move {
                if let (Some(session_id), Some(job_id)) = (session_id, job_id) {
                    active_jobs.update_stage(session_id, job_id, name.clone());
                }
                tx.send(Ok(StreamEvent {
                    event_type: "pipeline_stage".to_string(),
                    data: json!({"stage_name": name}),
                }))
                .await
                .ok();
                tracing::debug!(
                    component = "query/service",
                    stage = name,
                    "query.pipeline_stage"
                );
            }
        };

        // Load previous conversation history before persisting the current query.
        // The current user query is passed to the LLM separately; loading history
        // after early persistence would duplicate the same user message in the prompt.
        let history = if let Some(session_id) = request.session_id {
            svc.load_conversation_history(session_id).await?
        } else {
            Vec::new()
        };

        // Persist the user-visible conversation state before any slow RAG work.
        // This is intentionally before embedding, multi-query, search, and LLM calls:
        // if the browser reloads while those steps are running, recovery and the
        // sessions list must already show the user message and a non-default title.
        let user_message_id =
            Self::persist_user_message_and_autoname_session(&svc, &request).await?;

        // Step 1: Embed the query
        send_stage("embedding".to_string()).await;
        let embeddings = svc
            .embedding_client
            .embed(&eff_embedding_model, vec![request.query.clone()])
            .await?;
        let _embedding = embeddings.into_iter().next().ok_or_else(|| {
            AppError::EmbeddingError("Embedding service returned empty result".to_string())
        })?;

        let mut debug_data = DebugData::new(&request.query);
        let mut rerank_lookup: std::collections::HashMap<String, (f64, String)> =
            std::collections::HashMap::new();

        // Step 2: Advanced RAG logic
        let final_chunks: Vec<crate::shared::chunk_search::ChunkSearchResult>;

        if !advanced_rag_enabled {
            send_stage("searching".to_string()).await;
            let start = tokio::time::Instant::now();
            let chunk_search_results = chunk_search::search_chunks_semantic(
                svc.repo.chroma(),
                &svc.embedding_client,
                svc.repo.db(),
                request.collection_id,
                &request.query,
                None,
                eff_rerank_top_k,
                &eff_embedding_model,
            )
            .await?;

            debug_data.embedding_search = Some(EmbeddingSearchStep {
                query_snippet: request.query.clone(),
                embedding_dimension: _embedding.len(),
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
            let mut queries: Vec<String> = vec![request.query.clone()];

            if multi_query_enabled {
                send_stage("multi_query".to_string()).await;
                let mq_start = tokio::time::Instant::now();
                let mq_system = "You are an AI language model assistant. Your task is to generate 3 different versions of the given user question to retrieve relevant documents from a vector database. By generating multiple perspectives on the user question, your goal is to help the user overcome some of the limitations of the distance-based similarity search. Provide these alternative questions separated by newlines.";
                let variants_text = svc
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

            if hyde_enabled {
                send_stage("hyde".to_string()).await;
            }
            send_stage("searching".to_string()).await;
            let embed_start = tokio::time::Instant::now();
            let mut all_semantic_chunks = Vec::new();
            let mut hyde_results = Vec::new();
            let mut effective_query_first = request.query.clone();

            for (i, q) in queries.iter().enumerate() {
                let hyde_doc = if hyde_enabled {
                    let doc_start = tokio::time::Instant::now();
                    let hyde_system = "Please write a short hypothetical document that answers the question. The document should be factual and concise.";
                    let doc = svc
                        .llm_client
                        .query_single(hyde_system, q)
                        .await
                        .unwrap_or_default();
                    hyde_results.push(HydeResult {
                        query: q.clone(),
                        hypothetical_doc: doc.clone(),
                        latency_ms: doc_start.elapsed().as_millis() as u64,
                    });
                    format!("{}, {}", q, doc)
                } else {
                    q.clone()
                };
                if hyde_enabled && i == 0 {
                    effective_query_first = hyde_doc.clone();
                }
                let batch_results = chunk_search::search_chunks_semantic(
                    svc.repo.chroma(),
                    &svc.embedding_client,
                    svc.repo.db(),
                    request.collection_id,
                    &hyde_doc,
                    None,
                    eff_hybrid_top_k,
                    &eff_embedding_model,
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
                query_snippet: effective_query_first.clone(),
                embedding_dimension: _embedding.len(),
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

            let bm25_chunks = if bm25_enabled {
                send_stage("keyword_search".to_string()).await;
                let bm25_start = tokio::time::Instant::now();
                let results = chunk_search::search_bm25(
                    svc.repo.db(),
                    request.collection_id,
                    &request.query,
                    eff_hybrid_top_k,
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
                            score: r.score.unwrap_or(0.0),
                            text_snippet: r.text.chars().take(200).collect(),
                        })
                        .collect(),
                    latency_ms: bm25_start.elapsed().as_millis() as u64,
                });
                results
            } else {
                Vec::new()
            };

            let initial_count = all_semantic_chunks.len() + bm25_chunks.len();

            // RRF (Reciprocal Rank Fusion) — score by position in each result list
            // A chunk at rank 3 in vector search gets 1/(60+3) contribution, plus
            // 1/(60+7) from BM25 if it also appeared there. This fairly combines
            // semantic and lexical relevance without comparing raw score magnitudes.
            let rrf_k = 60.0_f64;
            let mut rrf_scores: std::collections::HashMap<String, f64> =
                std::collections::HashMap::new();

            for (i, chunk) in all_semantic_chunks.iter().enumerate() {
                let rank = (i + 1) as f64;
                *rrf_scores.entry(chunk.chunk_id.to_string()).or_insert(0.0) +=
                    1.0 / (rrf_k + rank);
            }
            for (i, chunk) in bm25_chunks.iter().enumerate() {
                let rank = (i + 1) as f64;
                *rrf_scores.entry(chunk.chunk_id.to_string()).or_insert(0.0) +=
                    1.0 / (rrf_k + rank);
            }

            // Track deduplicated chunks (found by BOTH search methods)
            let bm25_ids: std::collections::HashSet<String> =
                bm25_chunks.iter().map(|c| c.chunk_id.to_string()).collect();
            let vector_ids: std::collections::HashSet<String> = all_semantic_chunks
                .iter()
                .map(|c| c.chunk_id.to_string())
                .collect();
            let deduped_ids: Vec<String> = vector_ids.intersection(&bm25_ids).cloned().collect();

            // Dedup by chunk_id
            let mut unique_chunks = std::collections::HashMap::new();
            for c in all_semantic_chunks.clone() {
                unique_chunks.insert(c.chunk_id.to_string(), c);
            }
            for c in bm25_chunks.clone() {
                unique_chunks.insert(c.chunk_id.to_string(), c);
            }

            // Merge and sort by RRF score (descending) so the highest-ranked
            // chunks across both search methods appear first
            let mut merged: Vec<_> = unique_chunks.into_values().collect();
            merged.sort_by(|a, b| {
                let a_score = rrf_scores
                    .get(&a.chunk_id.to_string())
                    .copied()
                    .unwrap_or(0.0);
                let b_score = rrf_scores
                    .get(&b.chunk_id.to_string())
                    .copied()
                    .unwrap_or(0.0);
                b_score
                    .partial_cmp(&a_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let merge_dedup_results: Vec<SearchResultItem> = merged
                .iter()
                .map(|r| SearchResultItem {
                    chunk_id: r.chunk_id.to_string(),
                    document_name: r.document_name.clone(),
                    chunk_index: r.chunk_index,
                    score: r.score.unwrap_or(0.0),
                    text_snippet: r.text.chars().take(200).collect(),
                })
                .collect();

            debug_data.merge_dedup = Some(MergeDedupStep {
                input_chunks: initial_count,
                after_dedup: merged.len(),
                source_breakdown: MergeSourceBreakdown {
                    vector_chunks: all_semantic_chunks.len(),
                    keyword_chunks: bm25_chunks.len(),
                },
                results: merge_dedup_results,
                deduped_ids,
            });

            let final_merged = if reranking_enabled {
                send_stage("reranking".to_string()).await;
                let (rerank_system, batch_prompt) =
                    format_batch_rerank_prompt(&request.query, &merged);
                let response = svc
                    .llm_client
                    .query_single(&rerank_system, &batch_prompt)
                    .await;
                let verdicts: Vec<(usize, bool)> = match response {
                    Ok(text) => {
                        tracing::debug!(
                            component = "query/service",
                            response_len = text.len(),
                            "batch_rerank.response_received"
                        );
                        let parsed = parse_batch_verdicts(&text, merged.len());
                        if parsed.is_empty() && !merged.is_empty() {
                            tracing::warn!(
                                component = "query/service",
                                raw_snippet = %text.chars().take(200).collect::<String>(),
                                "batch_rerank.parse_error"
                            );
                            merged.iter().enumerate().map(|(i, _)| (i, true)).collect()
                        } else {
                            parsed
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            component = "query/service",
                            error = %e,
                            "batch_rerank.fallback"
                        );
                        merged.iter().enumerate().map(|(i, _)| (i, true)).collect()
                    }
                };
                let accepted_set: std::collections::HashSet<usize> = verdicts
                    .iter()
                    .filter(|(_, keep)| *keep)
                    .map(|(i, _)| *i)
                    .collect();
                let mut accepted_chunks = Vec::new();
                let mut rerank_results = Vec::new();
                for (i, chunk) in merged.into_iter().enumerate() {
                    let keep = accepted_set.contains(&i);
                    let rerank_score = if keep { 1.0 } else { 0.0 };
                    let rerank_verdict = if keep {
                        "брать".to_string()
                    } else {
                        "пропустить".to_string()
                    };
                    rerank_lookup.insert(
                        chunk.chunk_id.to_string(),
                        (rerank_score, rerank_verdict.clone()),
                    );
                    rerank_results.push(RerankResult {
                        chunk_id: chunk.chunk_id.to_string(),
                        document_name: chunk.document_name.clone(),
                        chunk_index: chunk.chunk_index,
                        text_snippet: chunk.text.chars().take(200).collect(),
                        score: rerank_score,
                        verdict: rerank_verdict,
                        comment: String::new(),
                    });
                    if keep {
                        accepted_chunks.push(chunk);
                    }
                }
                let accepted_count = accepted_chunks.len();
                let rejected_count = rerank_results.len() - accepted_count;
                let rerank_results_count = rerank_results.len();
                // accepted_chunks are already in RRF order (merged was sorted by RRF)
                if accepted_chunks.len() > eff_rerank_top_k {
                    accepted_chunks.truncate(eff_rerank_top_k);
                }
                debug_data.reranking = Some(RerankingStep {
                    input_count: rerank_results_count,
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

        send_stage("building_context".to_string()).await;

        // Build LLM context
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
            .map(|r| {
                let entry = rerank_lookup.get(&r.chunk_id.to_string());
                let (rerank_score, rerank_verdict) = entry
                    .map(|(s, v)| (*s, v.clone()))
                    .unwrap_or((0.0, String::new()));
                SourceRef {
                    document_id: r.document_id,
                    document_name: r.document_name.clone(),
                    chunk_index: r.chunk_index,
                    text: r.text.clone(),
                    relevance: r.score.unwrap_or(0.0),
                    stage: if advanced_rag_enabled && reranking_enabled {
                        Some("reranked".to_string())
                    } else {
                        Some("vector".to_string())
                    },
                    rerank_score: if advanced_rag_enabled && reranking_enabled {
                        Some(rerank_score)
                    } else {
                        None
                    },
                    rerank_verdict: if advanced_rag_enabled
                        && reranking_enabled
                        && !rerank_verdict.is_empty()
                    {
                        Some(rerank_verdict)
                    } else {
                        None
                    },
                }
            })
            .collect();

        let chunk_ids: Vec<String> = final_chunks
            .iter()
            .map(|r| r.chunk_id.to_string())
            .collect();

        // Use the previous history captured before early user-message persistence.

        // Populate debug data final answer step
        {
            let context_parts: Vec<String> = chunks
                .iter()
                .map(|c| {
                    format!(
                        "[Source: {} (chunk {})]\n{}",
                        c.document_name, c.index, c.text
                    )
                })
                .collect();
            let context_str = context_parts.join("\n\n");
            let messages = svc
                .llm_client
                .build_messages(&context_str, &request.query, &history);
            let prompt_preview = serde_json::to_string_pretty(&messages).unwrap_or_default();

            debug_data.final_answer = Some(FinalAnswerStep {
                model: rag_settings.llm_model.clone(),
                max_retries: MAX_RETRIES,
                chunks_in_context: chunks.len(),
                history_message_count: history.len(),
                history_token_estimate: 0,
                token_budget: rag_settings.llm_context_token_budget,
                total_tokens_estimate: 0,
                latency_ms: pipeline_start.elapsed().as_millis() as u64,
                prompt_preview,
            });
        }

        let debug_data_json = serde_json::to_string(&debug_data).unwrap_or_default();

        let assistant_message_id = request.session_id.map(|_| Uuid::new_v4());

        // Stream LLM response
        send_stage("generating".to_string()).await;
        let llm_stream = svc
            .llm_client
            .query_stream(&request.query, &chunks, &history)
            .await?;

        // Forward LLM events through the channel
        let llm_event_stream = QueryService::build_event_stream(
            llm_stream,
            source_refs,
            request.session_id,
            chunk_ids,
            user_message_id,
            assistant_message_id,
            svc.conversation_repo.clone(),
            svc.active_jobs.clone(),
            job_id,
            debug_data_json,
        );

        let mut llm_event_stream = Box::pin(llm_event_stream);
        let mut client_connected = true;
        while let Some(event) = llm_event_stream.next().await {
            if client_connected && tx.send(event).await.is_err() {
                client_connected = false;
                tracing::warn!(
                    component = "query/service",
                    session_id = request
                        .session_id
                        .map(|id| id.to_string())
                        .unwrap_or_default(),
                    "query.client_disconnected.continuing_pipeline"
                );
            }
        }

        Ok(())
    }

    async fn persist_user_message_and_autoname_session(
        svc: &QueryService,
        request: &QueryRequest,
    ) -> Result<Option<Uuid>, AppError> {
        let Some(session_id) = request.session_id else {
            return Ok(None);
        };

        let user_message_id = Uuid::new_v4();
        let msg = Message {
            id: user_message_id,
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

        svc.conversation_repo.add_message(&msg).await?;
        tracing::info!(
            component = "query/service",
            session_id = %session_id,
            user_message_id = %user_message_id,
            "[FIX] query.user_message_persisted_before_pipeline"
        );

        let session = svc.conversation_repo.get_session(session_id).await?;
        if matches!(session.title.as_str(), "New Chat" | "New Session") {
            if let Some(auto_title) = Self::query_auto_title(&request.query) {
                svc.conversation_repo
                    .update_session(session_id, Some(auto_title.clone()), None)
                    .await?;
                tracing::info!(
                    component = "query/service",
                    session_id = %session_id,
                    session_title = %auto_title,
                    "[FIX] query.session_autonamed_before_pipeline"
                );
            }
        }

        Ok(Some(user_message_id))
    }

    fn query_auto_title(query: &str) -> Option<String> {
        let title = query.trim().chars().take(50).collect::<String>();
        if title.is_empty() {
            None
        } else {
            Some(title)
        }
    }

    /// Create a pipeline_stage event for SSE streaming.
    #[allow(dead_code)]
    fn make_stage_event(stage_name: &str) -> StreamEvent {
        tracing::debug!(
            component = "query/service",
            stage = stage_name,
            "query.pipeline_stage"
        );
        StreamEvent {
            event_type: "pipeline_stage".to_string(),
            data: json!({"stage_name": stage_name}),
        }
    }

    /// Build the final event stream:
    ///   1. Debug data → "debug" event (RAG pipeline internals)
    ///   2. LLM text chunks → "chunk" events
    ///   3. Sources metadata → "sources" event
    ///   4. Completion signal → "done" event with message IDs
    #[allow(clippy::too_many_arguments)]
    fn build_event_stream(
        llm_stream: impl Stream<Item = Result<String, AppError>> + 'static,
        sources: Vec<SourceRef>,
        session_id: Option<Uuid>,
        _chunk_ids: Vec<String>,
        user_message_id: Option<Uuid>,
        assistant_message_id: Option<Uuid>,
        conversation_repo: ConversationRepository,
        active_jobs: ActiveJobRegistry,
        job_id: Option<Uuid>,
        debug_data_json: String,
    ) -> impl Stream<Item = Result<StreamEvent, Infallible>> {
        let sources_event = StreamEvent {
            event_type: "sources".to_string(),
            data: json!({"sources": sources}),
        };

        // Parse debug data into a JSON value for the debug event
        let debug_value: serde_json::Value =
            serde_json::from_str(&debug_data_json).unwrap_or_default();

        // Debug event: sent before chunks so the frontend can show pipeline internals
        let debug_event = stream::once(async move {
            Ok(StreamEvent {
                event_type: "debug".to_string(),
                data: json!({"debug": debug_value}),
            })
        });

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

            let active_jobs = active_jobs;

            stream::once(async move {
                // Persist the assistant message only for the latest pipeline in the session.
                // A query edited while streaming starts a replacement pipeline; the old
                // background task may still finish, but must not resurrect stale history.
                if let (Some(session_id), Some(asst_id_val)) = (sid, asst_id) {
                    if let Some(job_id) = job_id {
                        if !active_jobs.is_current(session_id, job_id) {
                            tracing::warn!(
                                component = "query/service",
                                session_id = %session_id,
                                job_id = %job_id,
                                assistant_message_id = %asst_id_val,
                                "[FIX] query.stale_assistant_persist_skipped"
                            );
                            return Ok(StreamEvent {
                                event_type: "done".to_string(),
                                data: json!({
                                    "user_message_id": user_message_id,
                                    "assistant_message_id": serde_json::Value::Null,
                                }),
                            });
                        }
                    }
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

        debug_event
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

/// Format a batch rerank prompt containing all chunks as a numbered list.
/// Returns (system_prompt, user_prompt).
fn format_batch_rerank_prompt(
    question: &str,
    chunks: &[crate::shared::chunk_search::ChunkSearchResult],
) -> (String, String) {
    tracing::debug!(
        component = "query/service",
        chunk_count = chunks.len(),
        total_text_len = chunks.iter().map(|c| c.text.len()).sum::<usize>(),
        "batch_rerank.start"
    );

    let system_prompt = "You are an expert relevance ranker. Given a user question and a \
                numbered list of document chunks, evaluate each chunk independently.\n\n\
                For each chunk, determine if it contains information that helps answer \
                the user's question.\n\n\
                Respond ONLY with a valid JSON array of objects. Each object must have:\n\
                  - \"index\": the chunk number (1-based)\n\
                  - \"verdict\": \"брать\" if the chunk is relevant, \"пропустить\" if \
                completely irrelevant\n\n\
                Example: [{\"index\": 1, \"verdict\": \"брать\"}, {\"index\": 2, \"verdict\": \"пропустить\"}]".to_string();

    let mut user_prompt = String::from("Question: ");
    user_prompt.push_str(question);
    user_prompt.push_str("\n\nChunks to evaluate:\n");
    for (i, chunk) in chunks.iter().enumerate() {
        user_prompt.push_str(&format!("--- Chunk {} ---\n{}\n\n", i + 1, chunk.text));
    }

    tracing::debug!(
        component = "query/service",
        prompt_len = user_prompt.len(),
        "batch_rerank.prompt_ready"
    );

    (system_prompt, user_prompt)
}

/// Parse batch rerank JSON response into (index, keep) verdicts.
/// Falls back gracefully on malformed input.
fn parse_batch_verdicts(response: &str, chunk_count: usize) -> Vec<(usize, bool)> {
    let root: serde_json::Value = match serde_json::from_str(response) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                component = "query/service",
                error = %e,
                "batch_rerank.parse_error.invalid_json"
            );
            return Vec::new();
        }
    };

    let items = match root.as_array() {
        Some(arr) => arr.clone(),
        None => {
            tracing::warn!(
                component = "query/service",
                "batch_rerank.parse_error.not_an_array"
            );
            return Vec::new();
        }
    };

    let mut verdicts = Vec::new();
    for item in &items {
        let idx = match item.get("index").and_then(|v| v.as_u64()) {
            Some(i) if i >= 1 && (i as usize) <= chunk_count => (i as usize) - 1,
            Some(i) if i > 0 => {
                tracing::warn!(
                    component = "query/service",
                    index = i,
                    chunk_count = chunk_count,
                    "batch_rerank.unexpected_indices"
                );
                continue;
            }
            _ => continue,
        };
        let verdict = item.get("verdict").and_then(|v| v.as_str()).unwrap_or("");
        let keep = verdict.to_lowercase().contains("брать");
        verdicts.push((idx, keep));
    }

    // Fill in missing indices as accepted (safe default)
    let present: std::collections::HashSet<usize> = verdicts.iter().map(|(i, _)| *i).collect();
    for i in 0..chunk_count {
        if !present.contains(&i) {
            verdicts.push((i, true));
        }
    }

    // Sort by index to preserve original order
    verdicts.sort_by_key(|a| a.0);

    tracing::info!(
        component = "query/service",
        accepted = verdicts.iter().filter(|(_, k)| *k).count(),
        rejected = verdicts.iter().filter(|(_, k)| !*k).count(),
        total = chunk_count,
        "batch_rerank.parsed"
    );

    verdicts
}

#[cfg(test)]
mod tests {
    use super::{
        parse_batch_verdicts, ActiveJobRegistry, AppError, Infallible, JobStatus, QueryService,
        StreamEvent,
    };
    use crate::modules::conversations::repository::ConversationRepository;
    use futures::stream;
    use futures::StreamExt;
    use serde_json::json;
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
            ActiveJobRegistry::new(),
            None,
            "null".to_string(),
        ));

        let events: Vec<StreamEvent> = stream
            .filter_map(|r| futures::future::ready(r.ok()))
            .collect()
            .await;

        // Expected: debug, chunk "Hello ", chunk "world", sources, done
        assert_eq!(
            events.len(),
            5,
            "should yield 5 events (debug + chunks + sources + done)"
        );
        assert_eq!(events[0].event_type, "debug");
        assert_eq!(events[1].event_type, "chunk");
        assert_eq!(events[1].data["text"], "Hello ");
        assert_eq!(events[2].event_type, "chunk");
        assert_eq!(events[2].data["text"], "world");
        assert_eq!(events[3].event_type, "sources");
        assert_eq!(events[4].event_type, "done");
        assert_eq!(events[4].data["user_message_id"], serde_json::Value::Null);
        assert_eq!(
            events[4].data["assistant_message_id"],
            serde_json::Value::Null
        );
    }

    /// Test that `make_stage_event` creates a correctly structured pipeline_stage event.
    #[test]
    fn make_stage_event_creates_correct_event() {
        let event = QueryService::make_stage_event("embedding");

        assert_eq!(event.event_type, "pipeline_stage");
        assert_eq!(event.data["stage_name"], "embedding");
    }

    #[test]
    fn active_job_registry_publishes_recovery_stage_updates() {
        let registry = ActiveJobRegistry::new();
        let session_id = uuid::Uuid::new_v4();
        let (job_id, rx) = registry.register(session_id);

        assert!(matches!(
            rx.borrow().clone(),
            JobStatus::Running { stage: None }
        ));

        registry.update_stage(session_id, job_id, "reranking".to_string());

        assert!(matches!(
            rx.borrow().clone(),
            JobStatus::Running { stage: Some(stage) } if stage == "reranking"
        ));
    }

    #[test]
    fn query_auto_title_trims_and_truncates_to_fifty_chars() {
        let title = QueryService::query_auto_title(
            "  12345678901234567890123456789012345678901234567890extra  ",
        );

        assert_eq!(
            title.as_deref(),
            Some("12345678901234567890123456789012345678901234567890")
        );
    }

    #[test]
    fn query_auto_title_returns_none_for_whitespace_only_query() {
        assert_eq!(QueryService::query_auto_title("   \n\t  "), None);
    }

    /// Test that stage events chained before LLM events appear in the correct order.
    #[tokio::test]
    async fn stage_events_appear_before_chunk_events_when_chained() {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost/nonexistent")
            .expect("lazy pool creation should not require a running DB");

        // Build stage events (simulating what process_query would collect)
        let stage_names = ["embedding", "searching", "building_context", "generating"];
        let stage_events: Vec<StreamEvent> = stage_names
            .iter()
            .map(|name| QueryService::make_stage_event(name))
            .collect();

        let stage_stream =
            futures::stream::iter(stage_events.into_iter().map(Ok::<StreamEvent, Infallible>));

        // Build an LLM stream with a single chunk
        let llm_stream = Box::pin(stream::iter(vec![Ok::<String, AppError>(
            "test response".to_string(),
        )]));

        let repo = ConversationRepository::new(pool);

        let llm_event_stream = Box::pin(QueryService::build_event_stream(
            llm_stream,
            vec![],
            None,
            vec![],
            None,
            None,
            repo,
            ActiveJobRegistry::new(),
            None,
            "null".to_string(),
        ));

        let full_stream = stage_stream.chain(llm_event_stream);

        let events: Vec<StreamEvent> = full_stream
            .filter_map(|r| futures::future::ready(r.ok()))
            .collect()
            .await;

        // Expected order: 4 stage events, then debug, chunk, sources, done
        assert_eq!(
            events.len(),
            8,
            "should yield 8 events: 4 stages + debug + chunk + sources + done"
        );

        // First 4 events must be pipeline_stage events in order
        assert_eq!(events[0].event_type, "pipeline_stage");
        assert_eq!(events[0].data["stage_name"], "embedding");
        assert_eq!(events[1].event_type, "pipeline_stage");
        assert_eq!(events[1].data["stage_name"], "searching");
        assert_eq!(events[2].event_type, "pipeline_stage");
        assert_eq!(events[2].data["stage_name"], "building_context");
        assert_eq!(events[3].event_type, "pipeline_stage");
        assert_eq!(events[3].data["stage_name"], "generating");

        // Then debug event (from build_event_stream)
        assert_eq!(events[4].event_type, "debug");

        // Then LLM chunk, sources, done
        assert_eq!(events[5].event_type, "chunk");
        assert_eq!(events[5].data["text"], "test response");
        assert_eq!(events[6].event_type, "sources");
        assert_eq!(events[7].event_type, "done");
    }

    /// Regression: channel-based stage stream yields events in correct order, just like
    /// `run_pipeline` sends events through `tokio::sync::mpsc`.
    #[tokio::test]
    async fn channel_stage_events_yielded_in_order() {
        use futures::StreamExt;
        use tokio::sync::mpsc;
        use tokio_stream::wrappers::ReceiverStream;

        let (tx, rx) = mpsc::channel::<Result<StreamEvent, Infallible>>(16);

        // Spawn a task that sends stage events through the channel
        tokio::spawn(async move {
            let events = vec![
                QueryService::make_stage_event("embedding"),
                QueryService::make_stage_event("searching"),
                QueryService::make_stage_event("building_context"),
                QueryService::make_stage_event("generating"),
            ];
            for event in events {
                tx.send(Ok(event)).await.unwrap();
            }
        });

        let stream = ReceiverStream::new(rx);
        let collected: Vec<StreamEvent> = stream
            .filter_map(|r| futures::future::ready(r.ok()))
            .collect()
            .await;

        assert_eq!(collected.len(), 4, "should receive exactly 4 stage events");
        assert_eq!(collected[0].event_type, "pipeline_stage");
        assert_eq!(collected[0].data["stage_name"], "embedding");
        assert_eq!(collected[1].event_type, "pipeline_stage");
        assert_eq!(collected[1].data["stage_name"], "searching");
        assert_eq!(collected[2].event_type, "pipeline_stage");
        assert_eq!(collected[2].data["stage_name"], "building_context");
        assert_eq!(collected[3].event_type, "pipeline_stage");
        assert_eq!(collected[3].data["stage_name"], "generating");
    }

    /// Regression: channel stream can interleave stage events with LLM forwarding.
    /// This simulates the final step of `run_pipeline` where stage events precede
    /// forwarded LLM chunk events through the same channel.
    #[tokio::test]
    async fn channel_stage_then_llm_events_yielded_in_order() {
        use futures::StreamExt;
        use tokio::sync::mpsc;
        use tokio_stream::wrappers::ReceiverStream;

        let (tx, rx) = mpsc::channel::<Result<StreamEvent, Infallible>>(32);

        tokio::spawn(async move {
            // Send stage events
            tx.send(Ok(QueryService::make_stage_event("embedding")))
                .await
                .unwrap();
            tx.send(Ok(QueryService::make_stage_event("searching")))
                .await
                .unwrap();
            tx.send(Ok(QueryService::make_stage_event("generating")))
                .await
                .unwrap();

            // Simulate forwarded LLM events (chunks, sources, done)
            tx.send(Ok(StreamEvent {
                event_type: "chunk".to_string(),
                data: json!({"text": "Hello"}),
            }))
            .await
            .unwrap();
            tx.send(Ok(StreamEvent {
                event_type: "chunk".to_string(),
                data: json!({"text": " world"}),
            }))
            .await
            .unwrap();
            tx.send(Ok(StreamEvent {
                event_type: "sources".to_string(),
                data: json!({"sources": []}),
            }))
            .await
            .unwrap();
            tx.send(Ok(StreamEvent {
                event_type: "done".to_string(),
                data: json!({"user_message_id": null}),
            }))
            .await
            .unwrap();
        });

        let stream = ReceiverStream::new(rx);
        let collected: Vec<StreamEvent> = stream
            .filter_map(|r| futures::future::ready(r.ok()))
            .collect()
            .await;

        assert_eq!(collected.len(), 7, "should receive all 7 events");

        // Stage events come first
        assert_eq!(collected[0].event_type, "pipeline_stage");
        assert_eq!(collected[0].data["stage_name"], "embedding");
        assert_eq!(collected[1].event_type, "pipeline_stage");
        assert_eq!(collected[1].data["stage_name"], "searching");
        assert_eq!(collected[2].event_type, "pipeline_stage");
        assert_eq!(collected[2].data["stage_name"], "generating");

        // Then LLM chunks, sources, done
        assert_eq!(collected[3].event_type, "chunk");
        assert_eq!(collected[3].data["text"], "Hello");
        assert_eq!(collected[4].event_type, "chunk");
        assert_eq!(collected[4].data["text"], " world");
        assert_eq!(collected[5].event_type, "sources");
        assert_eq!(collected[6].event_type, "done");
    }

    // ── Batch Reranking Tests ──

    #[test]
    fn test_parse_batch_verdicts_valid_json() {
        let response = r#"[
            {"index": 1, "verdict": "брать"},
            {"index": 2, "verdict": "пропустить"},
            {"index": 3, "verdict": "брать"}
        ]"#;
        let verdicts = parse_batch_verdicts(response, 3);
        assert_eq!(verdicts.len(), 3);
        assert!(verdicts[0].1); // index 0 → accept
        assert!(!verdicts[1].1); // index 1 → reject
        assert!(verdicts[2].1); // index 2 → accept
    }

    #[test]
    fn test_parse_batch_verdicts_all_accept() {
        let response = r#"[
            {"index": 1, "verdict": "брать"},
            {"index": 2, "verdict": "брать"},
            {"index": 3, "verdict": "брать"}
        ]"#;
        let verdicts = parse_batch_verdicts(response, 3);
        assert_eq!(verdicts.len(), 3);
        assert!(verdicts.iter().all(|(_, k)| *k));
    }

    #[test]
    fn test_parse_batch_verdicts_all_reject() {
        let response = r#"[
            {"index": 1, "verdict": "пропустить"},
            {"index": 2, "verdict": "пропустить"}
        ]"#;
        let verdicts = parse_batch_verdicts(response, 2);
        assert_eq!(verdicts.len(), 2);
        assert!(verdicts.iter().all(|(_, k)| !*k));
    }

    #[test]
    fn test_parse_batch_verdicts_partial() {
        let response = r#"[
            {"index": 1, "verdict": "брать"},
            {"index": 2, "verdict": "пропустить"},
            {"index": 3, "verdict": "пропустить"},
            {"index": 4, "verdict": "брать"}
        ]"#;
        let verdicts = parse_batch_verdicts(response, 4);
        assert_eq!(verdicts.len(), 4);
        assert!(verdicts[0].1); // accept
        assert!(!verdicts[1].1); // reject
        assert!(!verdicts[2].1); // reject
        assert!(verdicts[3].1); // accept
    }

    #[test]
    fn test_parse_batch_verdicts_invalid_json_fallback() {
        let response = "not valid json at all";
        let verdicts = parse_batch_verdicts(response, 5);
        // Invalid JSON returns empty Vec → caller falls back to accept all
        assert!(verdicts.is_empty());
    }

    #[test]
    fn test_parse_batch_verdicts_extra_indices() {
        // Response has more entries than chunks → extra entries are ignored
        let response = r#"[
            {"index": 1, "verdict": "брать"},
            {"index": 2, "verdict": "пропустить"},
            {"index": 999, "verdict": "брать"}
        ]"#;
        let verdicts = parse_batch_verdicts(response, 2);
        // Should have 2 entries (indices 0 and 1)
        assert_eq!(verdicts.len(), 2);
        assert!(verdicts[0].1);
        assert!(!verdicts[1].1);
    }

    #[test]
    fn test_parse_batch_verdicts_missing_indices() {
        // Response has fewer entries → missing entries are accepted
        let response = r#"[
            {"index": 2, "verdict": "пропустить"}
        ]"#;
        let verdicts = parse_batch_verdicts(response, 3);
        // Should have 3 entries: index 0 missing → accepted, index 1 specified as reject, index 2 missing → accepted
        assert_eq!(verdicts.len(), 3);
        assert!(verdicts[0].1); // index 0 missing → accepted
        assert!(!verdicts[1].1); // index 1 → specified as "пропустить" → rejected
        assert!(verdicts[2].1); // index 2 missing → accepted
    }

    #[test]
    fn test_batch_rerank_replaces_sequential_loop() {
        // Verify that parse_batch_verdicts handles a realistic multi-chunk response
        let response = format!(
            r#"[
                {{"index": 1, "verdict": "брать"}},
                {{"index": 2, "verdict": "брать"}},
                {{"index": 3, "verdict": "пропустить"}},
                {{"index": 4, "verdict": "брать"}},
                {{"index": 5, "verdict": "пропустить"}}
            ]"#
        );
        let verdicts = parse_batch_verdicts(&response, 5);
        assert_eq!(verdicts.len(), 5);
        let accepted: Vec<usize> = verdicts
            .iter()
            .filter(|(_, k)| *k)
            .map(|(i, _)| *i)
            .collect();
        assert_eq!(accepted, vec![0, 1, 3]);
        let rejected: Vec<usize> = verdicts
            .iter()
            .filter(|(_, k)| !*k)
            .map(|(i, _)| *i)
            .collect();
        assert_eq!(rejected, vec![2, 4]);
    }
}
