use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use chrono::Utc;
use serde_json::json;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::modules::documents::models::{Chunk, Document};
use crate::modules::documents::repository::DocumentRepository;
use crate::modules::web_crawl::crawler::WebCrawler;
use crate::modules::web_crawl::models::{
    CrawlConfig, CrawlJob, CrawlJobDetailResponse, CrawlJobSummary, CrawlPage, CrawlProgress,
    CrawlStatusResponse, CreateCrawlJobRequest,
};
use crate::modules::web_crawl::repository::WebCrawlRepository;
use crate::shared::chroma_client::ChromaClient;
use crate::shared::chunking::chunk_document_default;
use crate::shared::embedding_client::{EmbeddingClient, DEFAULT_EMBEDDING_MODEL};
use crate::shared::error::AppError;

#[derive(Clone)]
pub struct WebCrawlService {
    pub repo: WebCrawlRepository,
    pub doc_repo: DocumentRepository,
    pub chroma_url: String,
    pub embedding_client: EmbeddingClient,
    pub crawler: WebCrawler,
    pub crawl_progress: Arc<RwLock<HashMap<Uuid, CrawlProgress>>>,
    pub cancel_signals: Arc<RwLock<HashMap<Uuid, broadcast::Sender<()>>>>,
}

impl WebCrawlService {
    pub fn new(
        repo: WebCrawlRepository,
        doc_repo: DocumentRepository,
        chroma_url: String,
        embedding_client: EmbeddingClient,
    ) -> Self {
        Self {
            repo,
            doc_repo,
            chroma_url,
            embedding_client,
            crawler: WebCrawler::new(),
            crawl_progress: Arc::new(RwLock::new(HashMap::new())),
            cancel_signals: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ── Progress helpers ──

    pub fn set_crawl_progress(&self, job_id: Uuid, progress: CrawlProgress) {
        if let Ok(mut map) = self.crawl_progress.write() {
            map.insert(job_id, progress);
        }
    }

    pub fn get_crawl_progress(&self, job_id: Uuid) -> Option<CrawlProgress> {
        self.crawl_progress
            .read()
            .ok()
            .and_then(|m| m.get(&job_id).cloned())
    }

    pub fn clear_crawl_progress(&self, job_id: Uuid) {
        if let Ok(mut map) = self.crawl_progress.write() {
            map.remove(&job_id);
        }
    }

    // ── Start crawl ──

    /// Start crawling in the background for the given job.
    ///
    /// Spawns a `tokio::spawn` task that runs the BFS crawler, then for each
    /// crawled page: saves document + chunks to PG, embeds via RouterAI,
    /// and indexes into Chroma.
    pub async fn start_crawl(
        &self,
        job_id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<(), AppError> {
        let job = self
            .repo
            .get_job_for_user(job_id, user_id, is_admin)
            .await?;

        let acquired = self.repo.try_acquire_crawl_lock(job_id).await?;
        if !acquired {
            tracing::warn!(
                component = "web_crawl/service",
                crawl_job_id = %job_id,
                "start_crawl.concurrent_crawl_attempted"
            );
            return Err(AppError::BadRequest(
                "Crawl already in progress for this job".to_string(),
            ));
        }

        // Create cancel channel
        let (cancel_tx, _) = broadcast::channel::<()>(1);
        if let Ok(mut map) = self.cancel_signals.write() {
            map.insert(job_id, cancel_tx.clone());
        }

        self.repo
            .update_job_status(job_id, "crawling", 0, 0, None)
            .await?;

        let progress = CrawlProgress {
            pages_found: 0,
            pages_indexed: 0,
            current_url: String::new(),
            phase: "crawling".to_string(),
        };
        self.set_crawl_progress(job_id, progress);

        let svc = self.clone();
        tokio::spawn(async move {
            let cancel_rx = cancel_tx.subscribe();
            let (progress_tx, _) = broadcast::channel::<CrawlProgress>(32);

            let config: CrawlConfig =
                serde_json::from_value(job.config.clone()).unwrap_or_default();

            let result = svc
                .crawler
                .crawl(
                    &job.entry_url,
                    &config,
                    progress_tx,
                    cancel_rx,
                    job.collection_id,
                )
                .await;

            match result {
                Ok(pages) => {
                    let total = pages.len() as i32;
                    svc.set_crawl_progress(
                        job_id,
                        CrawlProgress {
                            pages_found: total,
                            pages_indexed: 0,
                            current_url: String::new(),
                            phase: "indexing".to_string(),
                        },
                    );

                    let mut indexed = 0i32;

                    for page in &pages {
                        // Create a CrawlPage record so the job detail endpoint can show discovered pages.
                        let crawl_page = CrawlPage {
                            id: Uuid::new_v4(),
                            job_id,
                            url: page.url.clone(),
                            depth: page.depth as i32,
                            status: "pending".to_string(),
                            http_status: page.http_status,
                            title: page.title.clone(),
                            created_at: Utc::now(),
                        };
                        if let Err(e) = svc.repo.create_page(&crawl_page).await {
                            tracing::error!(
                                component = "web_crawl/service",
                                error = %e,
                                url = %page.url,
                                "start_crawl.create_page_failed"
                            );
                        }

                        // Determine if the page fetch was successful (2xx HTTP status).
                        let fetch_ok = page
                            .http_status
                            .map(|s| (200..300).contains(&s))
                            .unwrap_or(false);

                        if !fetch_ok {
                            // Page was not fetched successfully — mark as failed and skip indexing.
                            if let Err(e) = svc
                                .repo
                                .update_page_status(crawl_page.id, "failed", page.http_status)
                                .await
                            {
                                tracing::error!(
                                    component = "web_crawl/service",
                                    error = %e,
                                    url = %page.url,
                                    "start_crawl.update_page_status_failed"
                                );
                            }
                            svc.set_crawl_progress(
                                job_id,
                                CrawlProgress {
                                    pages_found: total,
                                    pages_indexed: indexed,
                                    current_url: page.url.clone(),
                                    phase: "indexing".to_string(),
                                },
                            );
                            continue;
                        }

                        // Create document for this page
                        let doc_name = page.title.clone().unwrap_or_else(|| page.url.clone());

                        let doc = Document {
                            id: Uuid::new_v4(),
                            name: doc_name,
                            file_type: "text/html".to_string(),
                            file_size: page.text.len() as i64,
                            uploaded_at: Utc::now(),
                            collection_id: job.collection_id,
                            is_active: true,
                            source: "web".to_string(),
                            user_id: job.user_id.clone(),
                        };

                        match svc.doc_repo.save_document(&doc).await {
                            Ok(doc_id) => {
                                // Chunk the text
                                let chunk_data = chunk_document_default(&page.text);
                                let chunks: Vec<Chunk> = chunk_data
                                    .iter()
                                    .enumerate()
                                    .map(|(i, cd)| Chunk {
                                        id: Uuid::new_v4(),
                                        document_id: doc_id,
                                        index: i,
                                        text: cd.text.clone(),
                                        is_active: true,
                                    })
                                    .collect();

                                // Save chunks individually
                                let mut chunk_save_ok = true;
                                for chunk in &chunks {
                                    if let Err(e) = svc.doc_repo.save_chunk(chunk).await {
                                        tracing::error!(
                                            component = "web_crawl/service",
                                            error = %e,
                                            url = %page.url,
                                            "start_crawl.save_chunk_failed"
                                        );
                                        chunk_save_ok = false;
                                        break;
                                    }
                                }

                                let mut index_ok = false;
                                if chunk_save_ok && !chunks.is_empty() {
                                    // Embed and index into Chroma
                                    let texts: Vec<String> =
                                        chunks.iter().map(|c| c.text.clone()).collect();
                                    match svc
                                        .embedding_client
                                        .embed(DEFAULT_EMBEDDING_MODEL, texts.clone())
                                        .await
                                    {
                                        Ok(embeddings) => {
                                            let chroma = ChromaClient::new(&svc.chroma_url);
                                            let ids: Vec<String> =
                                                chunks.iter().map(|c| c.id.to_string()).collect();
                                            let metadatas: Vec<serde_json::Value> = chunks
                                                .iter()
                                                .map(|c| {
                                                    json!({
                                                        "document_id": doc_id.to_string(),
                                                        "chunk_index": c.index,
                                                        "source": "web",
                                                    })
                                                })
                                                .collect();

                                            match chroma
                                                .add_embeddings(
                                                    &job.collection_id.to_string(),
                                                    &ids,
                                                    &embeddings,
                                                    &metadatas,
                                                    &texts,
                                                )
                                                .await
                                            {
                                                Ok(()) => {
                                                    index_ok = true;
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        component = "web_crawl/service",
                                                        error = %e,
                                                        url = %page.url,
                                                        "start_crawl.chroma_index_failed"
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                component = "web_crawl/service",
                                                error = %e,
                                                url = %page.url,
                                                "start_crawl.embedding_failed"
                                            );
                                        }
                                    }
                                }

                                // Only count as indexed if embedding + Chroma indexing succeeded.
                                if index_ok {
                                    indexed += 1;
                                    if let Err(e) = svc
                                        .repo
                                        .update_page_status(
                                            crawl_page.id,
                                            "indexed",
                                            page.http_status,
                                        )
                                        .await
                                    {
                                        tracing::error!(
                                            component = "web_crawl/service",
                                            error = %e,
                                            url = %page.url,
                                            "start_crawl.update_page_status_indexed_failed"
                                        );
                                    }
                                } else {
                                    if let Err(e) = svc
                                        .repo
                                        .update_page_status(
                                            crawl_page.id,
                                            "failed",
                                            page.http_status,
                                        )
                                        .await
                                    {
                                        tracing::error!(
                                            component = "web_crawl/service",
                                            error = %e,
                                            url = %page.url,
                                            "start_crawl.update_page_status_failed"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    component = "web_crawl/service",
                                    error = %e,
                                    url = %page.url,
                                    "start_crawl.save_document_failed"
                                );
                                if let Err(e) = svc
                                    .repo
                                    .update_page_status(crawl_page.id, "failed", page.http_status)
                                    .await
                                {
                                    tracing::error!(
                                        component = "web_crawl/service",
                                        error = %e,
                                        url = %page.url,
                                        "start_crawl.update_page_status_failed"
                                    );
                                }
                            }
                        }

                        svc.set_crawl_progress(
                            job_id,
                            CrawlProgress {
                                pages_found: total,
                                pages_indexed: indexed,
                                current_url: page.url.clone(),
                                phase: "indexing".to_string(),
                            },
                        );
                    }

                    // Update job as completed
                    if let Err(e) = svc
                        .repo
                        .update_job_status(job_id, "completed", total, indexed, None)
                        .await
                    {
                        tracing::error!(
                            component = "web_crawl/service",
                            error = %e,
                            crawl_job_id = %job_id,
                            "start_crawl.update_completed_failed"
                        );
                    }

                    svc.clear_crawl_progress(job_id);
                    if let Ok(mut map) = svc.cancel_signals.write() {
                        map.remove(&job_id);
                    }

                    tracing::info!(
                        component = "web_crawl/service",
                        crawl_job_id = %job_id,
                        pages_crawled = total,
                        pages_indexed = indexed,
                        "start_crawl.completed"
                    );
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    tracing::error!(
                        component = "web_crawl/service",
                        crawl_job_id = %job_id,
                        error = %error_msg,
                        "start_crawl.failed"
                    );

                    let _ = svc
                        .repo
                        .update_job_status(job_id, "error", 0, 0, Some(&error_msg))
                        .await;
                    tracing::info!(
                        component = "web_crawl/service",
                        crawl_job_id = %job_id,
                        "start_crawl.error_detail"
                    );
                    svc.clear_crawl_progress(job_id);
                    if let Ok(mut map) = svc.cancel_signals.write() {
                        map.remove(&job_id);
                    }
                }
            }
        });

        Ok(())
    }

    /// Cancel a crawl job with cancel signal + DB update.
    pub async fn cancel_job(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<CrawlJobSummary, AppError> {
        // Verify ownership
        let _job = self.repo.get_job_for_user(id, user_id, is_admin).await?;

        // Send cancel signal
        if let Ok(map) = self.cancel_signals.read() {
            if let Some(tx) = map.get(&id) {
                let _ = tx.send(());
            }
        }

        let cancelled = self.repo.cancel_job(id).await?;
        if !cancelled {
            return Err(AppError::BadRequest(
                "Job cannot be cancelled — current status does not allow cancellation".to_string(),
            ));
        }

        self.clear_crawl_progress(id);
        if let Ok(mut map) = self.cancel_signals.write() {
            map.remove(&id);
        }

        let summary = self
            .repo
            .get_job_summary_with_collection_name(id, user_id, is_admin)
            .await?;

        tracing::info!(
            component = "web_crawl/service",
            crawl_job_id = %id,
            "cancel_job.completed"
        );

        Ok(summary)
    }

    /// Retry failed pages — list pages, reset only `failed` pages to `pending`,
    /// then restart the crawl pipeline for that job.
    pub async fn retry_failed_pages(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<CrawlJobSummary, AppError> {
        let job = self.repo.get_job_for_user(id, user_id, is_admin).await?;

        let pages = self.repo.list_pages_by_job(id).await?;
        let failed_count = pages.iter().filter(|p| p.status == "failed").count();

        if failed_count == 0 {
            return Err(AppError::BadRequest("No failed pages to retry".to_string()));
        }

        // Reset only failed pages to pending (don't touch successfully indexed pages).
        self.repo
            .update_pages_status_by_job(id, "pending", None, Some("failed"))
            .await?;

        tracing::info!(
            component = "web_crawl/service",
            crawl_job_id = %id,
            failed_pages = failed_count,
            "retry_failed_pages.reset"
        );

        // Reset job status to 'idle' so try_acquire_crawl_lock can proceed.
        // Preserve the current pages_found/pages_indexed counters — they will be
        // overwritten by start_crawl when the new crawl completes.
        self.repo
            .update_job_status(id, "idle", job.pages_found, job.pages_indexed, None)
            .await?;

        // Restart the crawl pipeline for the failed pages.
        // The crawl will re-fetch the entry URL and re-discover pages; pages already
        // indexed will have a document in PG and embeddings in Chroma. Re-crawling
        // will create new documents for the previously failed pages.
        if let Err(e) = self.start_crawl(id, user_id, is_admin).await {
            tracing::error!(
                component = "web_crawl/service",
                crawl_job_id = %id,
                error = %e,
                "retry_failed_pages.start_crawl_failed"
            );
            // Return the error — the retry itself failed.
            return Err(e);
        }

        let summary = self
            .repo
            .get_job_summary_with_collection_name(id, user_id, is_admin)
            .await?;

        tracing::info!(
            component = "web_crawl/service",
            crawl_job_id = %id,
            status = %summary.status,
            "retry_failed_pages.completed"
        );

        Ok(summary)
    }

    /// Get combined status (DB + in-memory progress).
    pub async fn get_crawl_status(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<CrawlStatusResponse, AppError> {
        let job = self.repo.get_job_for_user(id, user_id, is_admin).await?;
        let progress = self.get_crawl_progress(id);

        Ok(CrawlStatusResponse {
            job_id: job.id,
            status: job.status,
            pages_found: job.pages_found,
            pages_indexed: job.pages_indexed,
            error: job.error_message,
            progress,
        })
    }

    // ── Existing CRUD methods ──

    pub async fn create_job(
        &self,
        req: CreateCrawlJobRequest,
        user_id: &str,
        is_admin: bool,
    ) -> Result<CrawlJobSummary, AppError> {
        // Verify the user owns the target collection before creating a job against it.
        // Admins can create jobs against any collection.
        if !is_admin {
            let owner_ok: Option<(String,)> =
                sqlx::query_as("SELECT user_id FROM collections WHERE id = $1")
                    .bind(req.collection_id)
                    .fetch_optional(self.repo.pool())
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            component = "web_crawl/service",
                            error = %e,
                            collection_id = %req.collection_id,
                            "create_job.check_owner.sql_error"
                        );
                        AppError::InternalError(format!("Database error: {e}"))
                    })?;
            match owner_ok {
                None => {
                    tracing::warn!(
                        component = "web_crawl/service",
                        collection_id = %req.collection_id,
                        user_id = %user_id,
                        "create_job.collection_not_found"
                    );
                    return Err(AppError::NotFound(format!(
                        "Collection {} not found",
                        req.collection_id
                    )));
                }
                Some((owner_uid,)) if owner_uid != user_id => {
                    tracing::warn!(
                        component = "web_crawl/service",
                        collection_id = %req.collection_id,
                        user_id = %user_id,
                        owner = %owner_uid,
                        "create_job.not_owner"
                    );
                    return Err(AppError::Forbidden(
                        "You do not own the target collection".to_string(),
                    ));
                }
                _ => {}
            }
        }

        let config = req.config.unwrap_or_default();
        let config_json = serde_json::to_value(&config).map_err(|e| {
            tracing::error!(
                component = "web_crawl/service",
                error = %e,
                "create_job.serialize_config_error"
            );
            AppError::InternalError(format!("Failed to serialize config: {e}"))
        })?;

        let now = Utc::now();
        let job = CrawlJob {
            id: Uuid::new_v4(),
            entry_url: req.entry_url,
            config: config_json,
            status: "idle".to_string(),
            pages_found: 0,
            pages_indexed: 0,
            collection_id: req.collection_id,
            user_id: user_id.to_string(),
            error_message: None,
            created_at: now,
            updated_at: now,
        };

        self.repo.create_job(&job).await?;

        let summary = self
            .repo
            .get_job_summary_with_collection_name(job.id, user_id, false)
            .await?;

        tracing::info!(
            component = "web_crawl/service",
            crawl_job_id = %job.id,
            entry_url = %job.entry_url,
            collection_id = %job.collection_id,
            "create_job.completed"
        );

        Ok(summary)
    }

    pub async fn list_jobs(
        &self,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<CrawlJobSummary>, AppError> {
        self.repo.list_jobs_by_user(user_id, is_admin).await
    }

    pub async fn get_job_with_pages(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<CrawlJobDetailResponse, AppError> {
        let job = self.repo.get_job_for_user(id, user_id, is_admin).await?;
        let pages = self.repo.list_pages_by_job(id).await?;

        let collection_name = self
            .get_collection_name(job.collection_id)
            .await
            .unwrap_or_default();

        Ok(CrawlJobDetailResponse {
            id: job.id,
            entry_url: job.entry_url,
            config: job.config,
            status: job.status,
            pages_found: job.pages_found,
            pages_indexed: job.pages_indexed,
            collection_id: job.collection_id,
            collection_name,
            error_message: job.error_message,
            created_at: job.created_at,
            updated_at: job.updated_at,
            pages,
        })
    }

    pub async fn delete_job(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<(), AppError> {
        self.repo.delete_job_for_user(id, user_id, is_admin).await
    }

    async fn get_collection_name(&self, collection_id: Uuid) -> Result<String, AppError> {
        let result: Option<(String,)> =
            sqlx::query_as("SELECT name FROM collections WHERE id = $1")
                .bind(collection_id)
                .fetch_optional(self.repo.pool())
                .await
                .map_err(|e| {
                    tracing::error!(
                        component = "web_crawl/service",
                        error = %e,
                        collection_id = %collection_id,
                        "get_collection_name.sql_error"
                    );
                    AppError::InternalError(format!("Database error: {e}"))
                })?;

        result
            .map(|r| r.0)
            .ok_or_else(|| AppError::NotFound(format!("Collection {collection_id} not found")))
    }
}
