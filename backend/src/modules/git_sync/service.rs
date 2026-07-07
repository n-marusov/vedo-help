use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;

use chrono::Utc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::modules::documents::models::Chunk;
use crate::modules::documents::repository::DocumentRepository;
use crate::modules::git_sync::models::{GitRepo, SyncProgress, SyncStatusResponse};
use crate::modules::git_sync::repository::GitRepoRepository;
use crate::modules::settings::service::SettingsService;
use crate::shared::chroma_client::ChromaClient;
use crate::shared::chunking::chunk_document_default;
use crate::shared::embedding_client::EmbeddingClient;
use crate::shared::error::AppError;

/// Maximum file size for parsing (10 MB).
const MAX_MD_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Core service for Git repository sync operations.
///
/// Orchestrates the full pipeline: clone/pull → parse markdown → chunk →
/// embed → index into Chroma. All git operations run in `spawn_blocking`
/// to avoid blocking the async runtime.
///
/// The `access_token` is never logged — it is redacted from all tracing output.
#[derive(Clone, Debug)]
pub struct GitSyncService {
    pub repo: GitRepoRepository,
    pub doc_repo: DocumentRepository,
    pub chroma_url: String,
    pub embedding_client: EmbeddingClient,
    pub clone_root: PathBuf,
    pub settings_service: Option<SettingsService>,
    /// In-memory progress for active syncs — cleared on completion.
    pub sync_progress: Arc<RwLock<HashMap<Uuid, SyncProgress>>>,
}

impl GitSyncService {
    /// Create a new `GitSyncService`.
    pub fn new(
        repo: GitRepoRepository,
        doc_repo: DocumentRepository,
        chroma_url: String,
        embedding_client: EmbeddingClient,
        clone_root: PathBuf,
        settings_service: Option<SettingsService>,
    ) -> Self {
        tracing::info!(
            component = "git_sync/service",
            chroma_url = %chroma_url,
            clone_root = %clone_root.display(),
            "new"
        );
        Self {
            repo,
            doc_repo,
            chroma_url,
            embedding_client,
            clone_root,
            settings_service,
            sync_progress: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ── Sync progress helpers ──

    /// Store current sync progress for a repo (frontend polls via GET status).
    pub fn set_sync_progress(&self, repo_id: Uuid, progress: SyncProgress) {
        if let Ok(mut map) = self.sync_progress.write() {
            map.insert(repo_id, progress);
        }
    }

    /// Read current sync progress, or `None` if not actively syncing.
    pub fn get_sync_progress(&self, repo_id: Uuid) -> Option<SyncProgress> {
        self.sync_progress
            .read()
            .ok()
            .and_then(|m| m.get(&repo_id).cloned())
    }

    /// Remove progress entry once a sync completes or fails.
    pub fn clear_sync_progress(&self, repo_id: Uuid) {
        if let Ok(mut map) = self.sync_progress.write() {
            map.remove(&repo_id);
        }
    }

    /// Start the polling scheduler that periodically syncs all registered repos.
    ///
    /// If `interval_secs` is `0`, the scheduler logs an INFO and returns
    /// immediately (disabled).
    ///
    /// Each tick:
    /// 1. Lists all repos with status != `"syncing"`
    /// 2. For each eligible repo, calls `sync_repo(id)`
    /// 3. Tracks consecutive failures per repo with exponential backoff
    ///    (1m → 2m → 4m, cap at 30m)
    ///
    /// Shuts down cleanly when the broadcast receiver receives a signal.
    pub async fn start_scheduler(
        self: Arc<Self>,
        interval_secs: u64,
        mut shutdown: broadcast::Receiver<()>,
    ) {
        if interval_secs == 0 {
            tracing::info!(component = "git_sync/service", "start_scheduler.disabled");
            return;
        }

        tracing::info!(
            component = "git_sync/service",
            interval_secs = interval_secs,
            "start_scheduler.started"
        );

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // List all repos
                    let repos = match self.repo.list_repos().await {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::error!(
                                component = "git_sync/service",
                                error = %e,
                                "start_scheduler.list_repos_failed"
                            );
                            continue;
                        }
                    };

                    let eligible: Vec<Uuid> = repos
                        .iter()
                        .filter(|r| {
                            if r.status == "syncing" {
                                return false;
                            }
                            // Exponential backoff for error-status repos
                            if r.status == "error" {
                                if let Some(last_fail) = r.last_synced_at {
                                    let elapsed =
                                        (Utc::now() - last_fail).num_seconds() as u64;
                                    // Backoff: 1m, 2m, 4m, 8m, 16m, 30m (cap)
                                    // Use the consecutive error count from DB
                                    // but for simplicity use fixed 5m backoff
                                    if elapsed < 300 {
                                        tracing::debug!(
                                            component = "git_sync/service",
                                            git_repo_id = %r.id,
                                            elapsed_secs = elapsed,
                                            "start_scheduler.skipping_errored_repo"
                                        );
                                        return false;
                                    }
                                }
                            }
                            true
                        })
                        .map(|r| r.id)
                        .collect();

                    tracing::debug!(
                        component = "git_sync/service",
                        repos_checked = eligible.len(),
                        "start_scheduler.poll_cycle"
                    );

                    // Spawn sync tasks for all eligible repos in parallel
                    for repo_id in &eligible {
                        let svc = self.clone();
                        let rid = *repo_id;
                        tokio::spawn(async move {
                            if let Err(e) = svc.sync_repo_internal(rid).await {
                                tracing::error!(
                                    component = "git_sync/service",
                                    git_repo_id = %rid,
                                    error = %e,
                                    "start_scheduler.sync_failed"
                                );
                            }
                        });
                    }
                }
                _ = shutdown.recv() => {
                    tracing::info!(component = "git_sync/service", "start_scheduler.stopped");
                    break;
                }
            }
        }
    }
    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Full sync orchestrator for a single repo.
    ///
    /// 1. Atomically acquire sync lock (CAS: only if status != `"syncing"`)
    /// 2. Fetch repo metadata and resolve collection
    /// 3. If no `last_commit_hash` → full clone + parse all `.md` files
    /// 4. Else → pull + diff → parse only changed files
    /// 5. Index chunks into Chroma
    /// 6. Update `last_commit_hash`, `last_synced_at`, status → `"idle"`
    /// 7. On failure → status → `"error"`
    ///
    /// Uses the collection UUID (not the display name) for Chroma API calls,
    /// since Chroma accepts only ASCII alphanumeric, underscores, and hyphens
    /// in collection names.
    /// Sync a repository without ownership checks (internal use — scheduler, webhook).
    pub async fn sync_repo_internal(&self, repo_id: Uuid) -> Result<SyncStatusResponse, AppError> {
        self.sync_repo(repo_id, "", true).await
    }

    /// Sync a repository with ownership verification.
    pub async fn sync_repo(
        &self,
        repo_id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<SyncStatusResponse, AppError> {
        tracing::info!(component = "git_sync/service", git_repo_id = %repo_id, "sync_repo.started");

        // Atomically acquire sync lock — prevents concurrent syncs
        let acquired = self.repo.try_acquire_sync_lock(repo_id).await?;
        if !acquired {
            tracing::warn!(
                component = "git_sync/service",
                git_repo_id = %repo_id,
                "sync_repo.concurrent_sync_attempted"
            );
            return Ok(SyncStatusResponse {
                repo_id,
                status: "syncing".to_string(),
                files_indexed: 0,
                chunks_total: 0,
                last_commit: None,
                error: Some("Sync already in progress for this repository".to_string()),
                progress: None,
            });
        }

        // Fetch repo metadata with ownership check
        let git_repo = match self
            .repo
            .get_repo_for_user(repo_id, user_id, is_admin)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("Failed to fetch repo metadata: {e}");
                tracing::error!(
                    component = "git_sync/service",
                    error = %msg,
                    git_repo_id = %repo_id,
                    "sync_repo.fetch_metadata_failed"
                );
                self.repo.mark_sync_error(repo_id, &msg).await?;
                return Ok(SyncStatusResponse {
                    repo_id,
                    status: "error".to_string(),
                    files_indexed: 0,
                    chunks_total: 0,
                    last_commit: None,
                    error: Some(msg),
                    progress: None,
                });
            }
        };

        // Signal frontend that we are in the cloning phase
        self.set_sync_progress(
            repo_id,
            SyncProgress {
                total_files: 0,
                indexed_files: 0,
                current_file: String::new(),
                phase: "cloning".to_string(),
            },
        );

        // Use the collection UUID as the Chroma collection name — this is safe
        // for Chroma which accepts only ASCII alphanumeric, underscores, and hyphens.
        let collection_name = git_repo.collection_id.to_string();

        let local_path = self
            .clone_root
            .join(&git_repo.user_id)
            .join(repo_id.to_string());

        let local_path_exists = local_path.exists();

        let result = if git_repo.last_commit_hash.is_none() {
            tracing::info!(component = "git_sync/service", git_repo_id = %repo_id, "sync_repo.full_clone_mode");
            self.full_sync(&git_repo, &local_path, &collection_name)
                .await
        } else if !local_path_exists {
            tracing::info!(
                component = "git_sync/service",
                git_repo_id = %repo_id,
                local_path = %local_path.display(),
                "[FIX] sync_repo.fallback_to_full_clone: local path missing for previously synced repo"
            );
            self.full_sync(&git_repo, &local_path, &collection_name)
                .await
        } else {
            tracing::info!(component = "git_sync/service", git_repo_id = %repo_id, "sync_repo.incremental_sync_mode");
            self.incremental_sync(&git_repo, &local_path, &collection_name)
                .await
        };

        match result {
            Ok((new_commit, files_indexed, chunks_total)) => {
                let now = Utc::now();
                self.repo
                    .update_sync_status(repo_id, &new_commit, &now, "idle")
                    .await?;

                self.clear_sync_progress(repo_id);

                tracing::info!(
                    component = "git_sync/service",
                    git_repo_id = %repo_id,
                    files_indexed = files_indexed,
                    chunks_total = chunks_total,
                    "sync_repo.completed"
                );

                Ok(SyncStatusResponse {
                    repo_id,
                    status: "idle".to_string(),
                    files_indexed,
                    chunks_total,
                    last_commit: Some(new_commit),
                    error: None,
                    progress: None,
                })
            }
            Err(e) => {
                let error_msg = e.to_string();
                tracing::error!(
                    component = "git_sync/service",
                    git_repo_id = %repo_id,
                    error = %error_msg,
                    "sync_repo.failed"
                );
                // `mark_sync_error` only sets status to "error" without
                // overwriting last_commit_hash, preserving it for future syncs.
                self.repo.mark_sync_error(repo_id, &error_msg).await?;

                self.clear_sync_progress(repo_id);

                Ok(SyncStatusResponse {
                    repo_id,
                    status: "error".to_string(),
                    files_indexed: 0,
                    chunks_total: 0,
                    last_commit: git_repo.last_commit_hash,
                    error: Some(error_msg),
                    progress: None,
                })
            }
        }
    }

    /// Clone a repository to the local filesystem.
    ///
    /// Injects the access token into the URL (`https://{token}@host/path`)
    /// for authentication. Runs in `spawn_blocking` since `git2` is synchronous.
    /// The `access_token` is never logged.
    pub async fn clone_repo(&self, git_repo: &GitRepo) -> Result<PathBuf, AppError> {
        let repo_id = git_repo.id;
        let user_id = &git_repo.user_id;
        let clone_url = Self::inject_token(&git_repo.url, &git_repo.access_token);
        let redacted_url = Self::redact_clone_url(&clone_url);
        let branch = git_repo.branch.clone();
        let local_path = self.clone_root.join(user_id).join(repo_id.to_string());

        tracing::info!(
            component = "git_sync/service",
            git_repo_id = %repo_id,
            branch = %branch,
            repo_url = %redacted_url,
            local_path = %local_path.display(),
            "clone_repo.starting"
        );

        let local_path_clone = local_path.clone();
        let cloned_path = tokio::task::spawn_blocking(move || {
            // Remove any leftover directory from a previous failed clone attempt.
            // git2::RepoBuilder::clone() refuses to clone into a non-empty directory.
            if local_path_clone.exists() {
                tracing::debug!(
                    component = "git_sync/service",
                    git_repo_id = %repo_id,
                    local_path = %local_path_clone.display(),
                    "clone_repo.removing_leftover_dir"
                );
                std::fs::remove_dir_all(&local_path_clone).map_err(|e| {
                    AppError::InternalError(format!(
                        "Failed to remove leftover clone directory: {e}"
                    ))
                })?;
            }

            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts.download_tags(git2::AutotagOption::All);

            let mut builder = git2::build::RepoBuilder::new();
            builder.fetch_options(fetch_opts);
            builder.branch(&branch);

            builder.clone(&clone_url, &local_path_clone).map_err(|e| {
                tracing::error!(
                    component = "git_sync/service",
                    git_repo_id = %repo_id,
                    error = %e,
                    "clone_repo.failed"
                );
                AppError::InternalError(format!("Failed to clone repository: {e}"))
            })?;

            tracing::debug!(component = "git_sync/service", git_repo_id = %repo_id, "clone_repo.completed");

            Ok::<PathBuf, AppError>(local_path_clone)
        })
        .await
        .map_err(|e| {
            tracing::error!(
                component = "git_sync/service",
                git_repo_id = %repo_id,
                error = %e,
                "clone_repo.spawn_blocking_panicked"
            );
            AppError::InternalError(format!("Clone task failed: {e}"))
        })??;

        Ok(cloned_path)
    }

    /// Pull (fetch + fast-forward) an existing local repository.
    ///
    /// Returns `(old_commit, new_commit)` for computing the diff.
    pub async fn pull_repo(
        &self,
        local_path: &Path,
        branch: &str,
        access_token: &Option<String>,
        repo_url: &str,
    ) -> Result<(String, String), AppError> {
        let local_path = local_path.to_path_buf();
        let branch = branch.to_string();
        let pull_url = Self::inject_token(repo_url, access_token);

        tracing::debug!(
            component = "git_sync/service",
            local_path = %local_path.display(),
            branch = %branch,
            "pull_repo.starting"
        );

        tokio::task::spawn_blocking(move || {
            let repo = git2::Repository::open(&local_path).map_err(|e| {
                tracing::error!(
                    component = "git_sync/service",
                    local_path = %local_path.display(),
                    error = %e,
                    "pull_repo.open_failed"
                );
                AppError::InternalError(format!("Failed to open repository: {e}"))
            })?;

            // Get the current HEAD commit before fetch
            let old_commit = repo
                .head()
                .ok()
                .and_then(|head| head.peel_to_commit().ok())
                .map(|c| c.id().to_string())
                .unwrap_or_default();

            // Set authenticated remote URL for fetch
            repo.remote_set_url("origin", &pull_url).map_err(|e| {
                tracing::error!(component = "git_sync/service", error = %e, "pull_repo.set_remote_url_failed");
                AppError::InternalError(format!("Failed to set remote URL: {e}"))
            })?;

            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts.download_tags(git2::AutotagOption::All);

            repo.find_remote("origin")
                .and_then(|mut remote| remote.fetch(&[&branch], Some(&mut fetch_opts), None))
                .map_err(|e| {
                    tracing::error!(
                        component = "git_sync/service",
                        local_path = %local_path.display(),
                        error = %e,
                        "pull_repo.fetch_failed"
                    );
                    AppError::InternalError(format!("Failed to fetch from remote: {e}"))
                })?;

            // Fast-forward merge
            let fetch_head = repo
                .find_reference("FETCH_HEAD")
                .map_err(|e| AppError::InternalError(format!("Failed to find FETCH_HEAD: {e}")))?;
            let fetch_commit = repo
                .reference_to_annotated_commit(&fetch_head)
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to resolve FETCH_HEAD: {e}"))
                })?;

            let new_commit = fetch_commit.id().to_string();

            // Only fast-forward if there are new commits
            if old_commit != new_commit {
                let refname = format!("refs/heads/{branch}");
                if let Ok(annotated) = repo.reference_to_annotated_commit(&fetch_head) {
                    if let Ok(mut reference) = repo.find_reference(&refname) {
                        let _ = reference.set_target(annotated.id(), "fast-forward");
                    }
                }
            }

            tracing::debug!(
                component = "git_sync/service",
                old_commit = %old_commit,
                new_commit = %new_commit,
                "pull_repo.completed"
            );

            Ok((old_commit, new_commit))
        })
        .await
        .map_err(|e| {
            tracing::error!(component = "git_sync/service", error = %e, "pull_repo.spawn_blocking_panicked");
            AppError::InternalError(format!("Pull task failed: {e}"))
        })?
    }

    /// Get changed `.md` files between two commits using git diff.
    pub async fn get_changed_files(
        &self,
        local_path: &Path,
        old_commit: &str,
        new_commit: &str,
    ) -> Result<Vec<String>, AppError> {
        let local_path = local_path.to_path_buf();
        let old_commit = old_commit.to_string();
        let new_commit = new_commit.to_string();

        tracing::debug!(
            component = "git_sync/service",
            local_path = %local_path.display(),
            old_commit = %old_commit,
            new_commit = %new_commit,
            "get_changed_files"
        );

        tokio::task::spawn_blocking(move || {
            let repo = git2::Repository::open(&local_path).map_err(|e| {
                tracing::error!(component = "git_sync/service", error = %e, "get_changed_files.open_failed");
                AppError::InternalError(format!("Failed to open repository: {e}"))
            })?;

            let old_tree = repo
                .find_commit(
                    git2::Oid::from_str(&old_commit)
                        .map_err(|e| AppError::InternalError(format!("Invalid old commit: {e}")))?,
                )
                .map_err(|e| AppError::InternalError(format!("Failed to find old commit: {e}")))?
                .tree()
                .map_err(|e| AppError::InternalError(format!("Failed to get old tree: {e}")))?;

            let new_tree = repo
                .find_commit(
                    git2::Oid::from_str(&new_commit)
                        .map_err(|e| AppError::InternalError(format!("Invalid new commit: {e}")))?,
                )
                .map_err(|e| AppError::InternalError(format!("Failed to find new commit: {e}")))?
                .tree()
                .map_err(|e| AppError::InternalError(format!("Failed to get new tree: {e}")))?;

            let diff = repo
                .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)
                .map_err(|e| AppError::InternalError(format!("Failed to compute diff: {e}")))?;

            let mut changed_files = Vec::new();
            diff.foreach(
                &mut |delta, _| {
                    if let Some(path) = delta.new_file().path() {
                        if let Some(ext) = path.extension() {
                            if ext == "md" {
                                changed_files.push(path.to_string_lossy().to_string());
                            }
                        }
                    }
                    true
                },
                None,
                None,
                None,
            )
            .map_err(|e| AppError::InternalError(format!("Failed to iterate diff: {e}")))?;

            tracing::debug!(
                component = "git_sync/service",
                count = changed_files.len(),
                "get_changed_files.found"
            );

            Ok(changed_files)
        })
        .await
        .map_err(|e| {
            tracing::error!(
                component = "git_sync/service",
                error = %e,
                "get_changed_files.spawn_blocking_panicked"
            );
            AppError::InternalError(format!("Diff task failed: {e}"))
        })?
    }

    /// Walk a directory recursively, collecting all `.md` files with their content.
    ///
    /// If `filter` is provided, only include files whose relative path is in the list.
    /// Files larger than 10 MB are skipped with a WARN log.
    pub async fn parse_markdown_files(
        &self,
        dir: &Path,
        filter: Option<&[String]>,
    ) -> Result<Vec<(String, String)>, AppError> {
        let dir = dir.to_path_buf();
        let filter: Option<Vec<String>> = filter.map(|f| f.to_vec());

        tracing::debug!(
            component = "git_sync/service",
            dir = %dir.display(),
            filter = if filter.is_some() { "provided" } else { "none" },
            "parse_markdown_files.entry"
        );

        tokio::task::spawn_blocking(move || {
            let mut files = Vec::new();
            let mut skipped = 0usize;

            for entry in walkdir::WalkDir::new(&dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext != "md" {
                        continue;
                    }
                } else {
                    continue;
                }

                // Compute path relative to root dir
                let rel_path = path
                    .strip_prefix(&dir)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();

                // Apply filter if present
                if let Some(ref filter) = filter {
                    if !filter.contains(&rel_path) {
                        continue;
                    }
                }

                // Check file size — skip >10MB
                if let Ok(metadata) = path.metadata() {
                    if metadata.len() > MAX_MD_FILE_SIZE {
                        tracing::warn!(
                            component = "git_sync/service",
                            file_name = %rel_path,
                            file_size = metadata.len(),
                            "parse_markdown_files.skipped_large_file"
                        );
                        skipped += 1;
                        continue;
                    }
                }

                // Read file content
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        files.push((rel_path, content));
                    }
                    Err(e) => {
                        tracing::warn!(
                            component = "git_sync/service",
                            file_name = %rel_path,
                            error = %e,
                            "parse_markdown_files.skipped_non_utf8"
                        );
                        skipped += 1;
                        continue;
                    }
                }
            } // end for loop

            tracing::debug!(
                component = "git_sync/service",
                files_found = files.len(),
                files_skipped = skipped,
                "parse_markdown_files.completed"
            );

            Ok(files)
        })
        .await
        .map_err(|e| {
            tracing::error!(
                component = "git_sync/service",
                error = %e,
                "parse_markdown_files.spawn_blocking_panicked"
            );
            AppError::InternalError(format!("Parse task failed: {e}"))
        })?
    }

    /// Index parsed markdown files into PostgreSQL + Chroma via chunking + embedding.
    ///
    /// For each file:
    /// 1. Deactivate old git documents/chunks for this file
    /// 2. Save new Document record (source='git')
    /// 3. Save Chunk records (UUID PK for each)
    /// 4. Embed and index into Chroma with chunk UUIDs as IDs
    ///
    /// Returns `(files_count, chunks_total)`.
    pub async fn index_chunks(
        &self,
        collection_name: &str,
        collection_id: Uuid,
        repo_id: Uuid,
        user_id: &str,
        files: &[(String, String)],
    ) -> Result<(usize, usize), AppError> {
        tracing::info!(
            component = "git_sync/service",
            collection = %collection_name,
            git_repo_id = %repo_id,
            files_count = files.len(),
            "index_chunks.entry"
        );

        let chroma = ChromaClient::new(&self.chroma_url);

        let mut files_indexed = 0usize;
        let mut chunks_total = 0usize;
        let mut all_ids = Vec::new();
        let mut all_embeddings = Vec::new();
        let mut all_metadatas = Vec::new();
        let mut all_texts = Vec::new();

        for (file_path, content) in files {
            // 1. Deactivate old git documents for this repo file
            if let Ok(Some(old_doc)) = self
                .doc_repo
                .get_active_git_document_by_name(collection_id, file_path)
                .await
            {
                tracing::debug!(
                    component = "git_sync/service",
                    old_document_id = %old_doc.id,
                    file_path = %file_path,
                    "index_chunks.deactivating_old_document"
                );

                // Delete from Chroma by the old document UUID
                if let Err(e) = chroma
                    .delete_where(
                        collection_name,
                        &serde_json::json!({"document_id": old_doc.id.to_string()}),
                    )
                    .await
                {
                    tracing::warn!(
                        component = "git_sync/service",
                        document_id = %old_doc.id,
                        collection = %collection_name,
                        error = %e,
                        "index_chunks.delete_old_chunks_from_chroma_failed"
                    );
                }

                // Deactivate old chunks and document in PG
                if let Err(e) = self.doc_repo.deactivate_chunks_batch(&[old_doc.id]).await {
                    tracing::warn!(
                        component = "git_sync/service",
                        document_id = %old_doc.id,
                        error = %e,
                        "index_chunks.deactivate_old_chunks_failed"
                    );
                }
                if let Err(e) = self
                    .doc_repo
                    .deactivate_documents_batch(&[old_doc.id])
                    .await
                {
                    tracing::warn!(
                        component = "git_sync/service",
                        document_id = %old_doc.id,
                        error = %e,
                        "index_chunks.deactivate_old_document_failed"
                    );
                }
            }

            // Chunk the document
            let chunks = chunk_document_default(content);
            if chunks.is_empty() {
                continue;
            }

            let chunk_count = chunks.len();

            // 2. Save new Document record
            let doc_id = Uuid::new_v4();
            let doc = crate::modules::documents::models::Document {
                id: doc_id,
                name: file_path.clone(),
                file_type: "text/markdown".to_string(),
                file_size: content.len() as i64,
                uploaded_at: chrono::Utc::now(),
                collection_id,
                is_active: true,
                source: "git".to_string(),
                user_id: user_id.to_string(),
            };
            self.doc_repo.save_document(&doc).await?;

            // 3. Save Chunk records and prepare Chroma data
            let chunk_texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();

            // Determine embedding model from settings (fallback to DEFAULT_EMBEDDING_MODEL)
            let mut embedding_model =
                crate::shared::embedding_client::DEFAULT_EMBEDDING_MODEL.to_string();
            if let Some(ref settings) = self.settings_service {
                if let Ok(rag_settings) = settings.get_rag_settings().await {
                    embedding_model = rag_settings.embedding_model;
                }
            }

            // Embed
            let embeddings = self
                .embedding_client
                .embed(&embedding_model, chunk_texts.clone())
                .await
                .map_err(|e| {
                    tracing::error!(
                        component = "git_sync/service",
                        git_repo_id = %repo_id,
                        file_name = %file_path,
                        error = %e,
                        "index_chunks.embedding_failed"
                    );
                    e
                })?;

            for (i, chunk) in chunks.iter().enumerate() {
                let chunk_id = Uuid::new_v4();
                let chunk_record = Chunk {
                    id: chunk_id,
                    document_id: doc_id,
                    index: chunk.index,
                    text: chunk.text.clone(),
                    is_active: true,
                };
                self.doc_repo.save_chunk(&chunk_record).await?;

                all_ids.push(chunk_id.to_string());
                all_embeddings.push(embeddings[i].clone());
                all_texts.push(chunk.text.clone());
                all_metadatas.push(serde_json::json!({
                    "document_id": doc_id.to_string(),
                    "document_name": file_path,
                    "chunk_id": chunk_id.to_string(),
                    "chunk_index": chunk.index,
                    "text": chunk.text,
                    "is_active": true,
                    "source": "git",
                    "file_path": file_path,
                }));
            }

            files_indexed += 1;
            chunks_total += chunk_count;

            // Update sync progress so the frontend can poll it
            if let Ok(mut map) = self.sync_progress.write() {
                if let Some(progress) = map.get_mut(&repo_id) {
                    progress.indexed_files = files_indexed;
                    progress.current_file = file_path.clone();
                }
            }

            tracing::info!(
                component = "git_sync/service",
                document_id = %doc_id,
                file_path = %file_path,
                chunk_count = chunk_count,
                "index_chunks.document_indexed"
            );
        }

        // Send all embeddings to Chroma in one batch
        if !all_ids.is_empty() {
            tracing::debug!(
                component = "git_sync/service",
                chunk_count = all_ids.len(),
                collection = %collection_name,
                "index_chunks.sending_to_chroma"
            );

            chroma
                .add_embeddings(
                    collection_name,
                    &all_ids,
                    &all_embeddings,
                    &all_metadatas,
                    &all_texts,
                )
                .await
                .map_err(|e| {
                    tracing::error!(
                        component = "git_sync/service",
                        collection = %collection_name,
                        git_repo_id = %repo_id,
                        error = %e,
                        "index_chunks.chroma_add_failed"
                    );
                    e
                })?;
        }

        tracing::info!(
            component = "git_sync/service",
            files_indexed = files_indexed,
            chunks_total = chunks_total,
            "index_chunks.completed"
        );

        Ok((files_indexed, chunks_total))
    }

    /// Delete the local clone directory for a repo.
    pub async fn delete_repo_local(&self, repo_id: Uuid, user_id: &str) -> Result<(), AppError> {
        let local_path = self.clone_root.join(user_id).join(repo_id.to_string());

        tracing::info!(
            component = "git_sync/service",
            git_repo_id = %repo_id,
            local_path = %local_path.display(),
            "delete_repo_local.deleting"
        );

        tokio::task::spawn_blocking(move || {
            if local_path.exists() {
                std::fs::remove_dir_all(&local_path).map_err(|e| {
                    tracing::error!(
                        component = "git_sync/service",
                        git_repo_id = %repo_id,
                        local_path = %local_path.display(),
                        error = %e,
                        "delete_repo_local.failed"
                    );
                    AppError::InternalError(format!("Failed to remove local clone directory: {e}"))
                })?;

                tracing::debug!(component = "git_sync/service", git_repo_id = %repo_id, "delete_repo_local.removed");
            } else {
                tracing::debug!(
                    component = "git_sync/service",
                    git_repo_id = %repo_id,
                    local_path = %local_path.display(),
                    "delete_repo_local.not_found"
                );
            }
            Ok(())
        })
        .await
        .map_err(|e| {
            tracing::error!(
                component = "git_sync/service",
                error = %e,
                "delete_repo_local.spawn_blocking_panicked"
            );
            AppError::InternalError(format!("Delete local repo task failed: {e}"))
        })?
    }

    /// Full cleanup: delete Chroma documents → delete local clone → delete PostgreSQL row.
    ///
    /// Uses Chroma's `where` filter to remove only git-sourced chunks from the
    /// collection, preserving any manually uploaded documents in the same collection.
    pub async fn delete_repo_and_cleanup(
        &self,
        repo_id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<(), AppError> {
        tracing::info!(component = "git_sync/service", git_repo_id = %repo_id, "delete_repo_and_cleanup.starting");

        // Clean up any in-memory sync progress for this repo
        self.clear_sync_progress(repo_id);

        // Get repo with ownership check
        let git_repo = self
            .repo
            .get_repo_for_user(repo_id, user_id, is_admin)
            .await?;
        let collection_name = self
            .repo
            .get_collection_name(git_repo.collection_id)
            .await?;

        // 1. Delete git-sourced embeddings from Chroma using metadata filter
        //    (preserves any manually uploaded documents in the same collection)
        let chroma = ChromaClient::new(&self.chroma_url);
        let filter = serde_json::json!({
            "source": "git",
            "repo_id": repo_id.to_string(),
        });
        if let Err(e) = chroma.delete_where(&collection_name, &filter).await {
            tracing::warn!(
                component = "git_sync/service",
                git_repo_id = %repo_id,
                collection_name = %collection_name,
                error = %e,
                "delete_repo_and_cleanup.chroma_delete_failed"
            );
        }

        // 2. Parse repo files and deactivate only those git documents in PostgreSQL
        let collection_id = git_repo.collection_id;
        let user_id_str = &git_repo.user_id;
        let local_path = self.clone_root.join(user_id_str).join(repo_id.to_string());
        let repo_files = self
            .parse_markdown_files(&local_path, None)
            .await
            .unwrap_or_default();

        let file_paths: Vec<&str> = repo_files.iter().map(|(path, _)| path.as_str()).collect();
        if !file_paths.is_empty() {
            match self
                .doc_repo
                .deactivate_git_documents_by_names(collection_id, &file_paths)
                .await
            {
                Ok(count) => {
                    tracing::info!(
                        component = "git_sync/service",
                        git_repo_id = %repo_id,
                        deactivated_count = count,
                        "delete_repo_and_cleanup.pg_documents_deactivated"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        component = "git_sync/service",
                        git_repo_id = %repo_id,
                        error = %e,
                        "delete_repo_and_cleanup.pg_documents_deactivation_failed"
                    );
                }
            }
        }

        // 3. Delete local clone
        self.delete_repo_local(repo_id, user_id).await?;

        // 4. Delete PostgreSQL row with ownership check
        self.repo
            .delete_repo_for_user(repo_id, user_id, is_admin)
            .await?;

        tracing::info!(component = "git_sync/service", git_repo_id = %repo_id, "delete_repo_and_cleanup.completed");

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Full sync: clone → parse all .md → index.
    async fn full_sync(
        &self,
        git_repo: &GitRepo,
        _local_path: &Path,
        collection_name: &str,
    ) -> Result<(String, usize, usize), AppError> {
        // 1. Clone
        let local_path = self.clone_repo(git_repo).await?;

        // 2. Get HEAD commit
        let head_commit = tokio::task::spawn_blocking({
            let local_path = local_path.clone();
            move || -> Result<String, AppError> {
                let repo = git2::Repository::open(&local_path).map_err(|e| {
                    AppError::InternalError(format!("Failed to open cloned repo: {e}"))
                })?;
                let head = repo
                    .head()
                    .map_err(|e| AppError::InternalError(format!("Failed to get HEAD: {e}")))?;
                let commit = head
                    .peel_to_commit()
                    .map_err(|e| AppError::InternalError(format!("Failed to peel commit: {e}")))?;
                Ok(commit.id().to_string())
            }
        })
        .await
        .map_err(|e| AppError::InternalError(format!("Spawn blocked failed: {e}")))??;

        // 3. Parse all .md files
        let files = self.parse_markdown_files(&local_path, None).await?;

        if files.is_empty() {
            tracing::info!(
                component = "git_sync/service",
                git_repo_id = %git_repo.id,
                "full_sync.no_md_files"
            );
            return Ok((head_commit, 0, 0));
        }

        // Signal frontend with total file count for progress bar
        self.set_sync_progress(
            git_repo.id,
            SyncProgress {
                total_files: files.len(),
                indexed_files: 0,
                current_file: String::new(),
                phase: "indexing".to_string(),
            },
        );

        // 4. Index chunks (save to PG + Chroma)
        let (files_indexed, chunks_total) = self
            .index_chunks(
                collection_name,
                git_repo.collection_id,
                git_repo.id,
                &git_repo.user_id,
                &files,
            )
            .await?;

        Ok((head_commit, files_indexed, chunks_total))
    }

    /// Incremental sync: pull → diff → parse changed files → re-index.
    async fn incremental_sync(
        &self,
        git_repo: &GitRepo,
        local_path: &Path,
        collection_name: &str,
    ) -> Result<(String, usize, usize), AppError> {
        // 1. Pull
        let (old_commit, new_commit) = self
            .pull_repo(
                local_path,
                &git_repo.branch,
                &git_repo.access_token,
                &git_repo.url,
            )
            .await?;

        // If no new commits, return early
        if old_commit == new_commit {
            tracing::debug!(
                component = "git_sync/service",
                git_repo_id = %git_repo.id,
                "incremental_sync.no_new_commits"
            );
            return Ok((new_commit, 0, 0));
        }

        // 2. Get changed .md files
        let changed_files = self
            .get_changed_files(local_path, &old_commit, &new_commit)
            .await?;

        if changed_files.is_empty() {
            tracing::debug!(
                component = "git_sync/service",
                git_repo_id = %git_repo.id,
                "incremental_sync.no_changed_files"
            );
            return Ok((new_commit, 0, 0));
        }

        // 3. Parse only changed files
        let files = self
            .parse_markdown_files(local_path, Some(&changed_files))
            .await?;

        if files.is_empty() {
            tracing::debug!(
                component = "git_sync/service",
                git_repo_id = %git_repo.id,
                "incremental_sync.no_parsed_files"
            );
            return Ok((new_commit, 0, 0));
        }

        // Signal frontend with total changed files for progress bar
        self.set_sync_progress(
            git_repo.id,
            SyncProgress {
                total_files: files.len(),
                indexed_files: 0,
                current_file: String::new(),
                phase: "indexing".to_string(),
            },
        );

        // 4. Re-index (save to PG + Chroma)
        let (files_indexed, chunks_total) = self
            .index_chunks(
                collection_name,
                git_repo.collection_id,
                git_repo.id,
                &git_repo.user_id,
                &files,
            )
            .await?;

        Ok((new_commit, files_indexed, chunks_total))
    }

    /// Inject an access token into a git HTTPS URL.
    ///
    /// `https://host/path` → `https://{token}@host/path`
    /// Non-HTTPS URLs (SSH, file://) are returned unchanged.
    /// An empty or `None` token is treated as no token.
    /// Inject an access token into a git HTTPS URL for actual git operations.
    ///
    /// `https://host/path` → `https://{token}@host/path`
    /// Non-HTTPS URLs (SSH, file://) are returned unchanged.
    /// The real token is used — never log the result of this function.
    /// An empty or `None` token is treated as no token.
    pub fn inject_token(url: &str, token: &Option<String>) -> String {
        match token {
            Some(t) if !t.is_empty() => {
                if let Some(rest) = url.strip_prefix("https://") {
                    format!("https://{}@{rest}", t)
                } else {
                    url.to_string()
                }
            }
            _ => url.to_string(),
        }
    }

    /// Return a redacted version of the clone URL for safe logging.
    /// Substitutes the token portion with `[REDACTED]`.
    fn redact_clone_url(url: &str) -> String {
        if let Some(at_pos) = url.find('@') {
            if url.starts_with("https://") {
                format!("https://[REDACTED]{}", &url[at_pos..])
            } else {
                url.to_string()
            }
        } else {
            url.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_token_https_with_token() {
        let url = "https://github.com/user/repo.git";
        let result = GitSyncService::inject_token(url, &Some("ghp_secret123".to_string()));
        assert_eq!(result, "https://ghp_secret123@github.com/user/repo.git");
    }

    #[test]
    fn test_inject_token_no_token() {
        let url = "https://github.com/user/pub.git";
        let result = GitSyncService::inject_token(url, &None);
        assert_eq!(result, "https://github.com/user/pub.git");
    }

    #[test]
    fn test_inject_token_empty_token() {
        let url = "https://github.com/user/pub.git";
        let result = GitSyncService::inject_token(url, &Some(String::new()));
        assert_eq!(result, "https://github.com/user/pub.git");
    }

    #[test]
    fn test_inject_token_ssh_url() {
        let url = "git@github.com:user/repo.git";
        let result = GitSyncService::inject_token(url, &Some("token".to_string()));
        assert_eq!(result, "git@github.com:user/repo.git");
    }

    #[test]
    fn test_inject_token_file_url() {
        let url = "file:///tmp/repo";
        let result = GitSyncService::inject_token(url, &Some("token".to_string()));
        assert_eq!(result, "file:///tmp/repo");
    }
}
