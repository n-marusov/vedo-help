use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::modules::git_sync::models::{
    CreateRepoRequest, GitRepo, GitRepoSummary, SyncStatusResponse,
};
use crate::modules::git_sync::service::GitSyncService;
use crate::shared::error::AppError;

/// Register a new Git repository for document syncing.
///
/// Endpoint: `POST /api/git-sync/repos`
///
/// Validates that the URL starts with `https://` or `git@`, generates a
/// unique ID, and creates the local path. The `access_token` is stored but
/// never returned in responses.
pub async fn create_repo(
    State(svc): State<GitSyncService>,
    Json(req): Json<CreateRepoRequest>,
) -> Result<Json<GitRepoSummary>, AppError> {
    // Validate URL — only HTTPS and SSH URLs are supported
    if !req.url.starts_with("https://") && !req.url.starts_with("git@") {
        return Err(AppError::BadRequest(
            "URL must start with https:// or git@".to_string(),
        ));
    }

    tracing::info!(
        "[handler::create_repo] url={} collection_id={}",
        &req.url,
        req.collection_id
    );
    tracing::debug!(
        "[handler::create_repo] branch={:?} has_token={}",
        req.branch,
        req.access_token.is_some()
    );

    let id = Uuid::new_v4();
    let now = Utc::now();
    let local_path = svc
        .clone_root
        .join(id.to_string())
        .to_string_lossy()
        .to_string();

    let repo = GitRepo {
        id,
        url: req.url,
        branch: req.branch.unwrap_or_else(|| "main".to_string()),
        access_token: req.access_token,
        local_path,
        last_commit_hash: None,
        last_synced_at: None,
        collection_id: req.collection_id,
        status: "idle".to_string(),
        webhook_secret: None,
        created_at: now,
        updated_at: now,
    };

    svc.repo.create_repo(&repo).await.map_err(|e| {
        tracing::error!("[handler::create_repo] failed repo_id={id} error={e}");
        e
    })?;

    let summary = svc
        .repo
        .get_repo_with_collection_name(id)
        .await
        .map_err(|e| {
            tracing::error!(
                "[handler::create_repo] failed to fetch summary repo_id={id} error={e}"
            );
            e
        })?;

    tracing::info!("[handler::create_repo] created repo_id={id}");
    Ok(Json(summary))
}

/// List all registered Git repositories with their collection names.
///
/// Endpoint: `GET /api/git-sync/repos`
pub async fn list_repos(
    State(svc): State<GitSyncService>,
) -> Result<Json<Vec<GitRepoSummary>>, AppError> {
    tracing::info!("[handler::list_repos]");

    let repos = svc
        .repo
        .list_repos_with_collection_names()
        .await
        .map_err(|e| {
            tracing::error!("[handler::list_repos] failed error={e}");
            e
        })?;

    tracing::debug!("[handler::list_repos] count={}", repos.len());
    Ok(Json(repos))
}

/// Get a single Git repository by ID.
///
/// Endpoint: `GET /api/git-sync/repos/{id}`
pub async fn get_repo(
    State(svc): State<GitSyncService>,
    Path(id): Path<Uuid>,
) -> Result<Json<GitRepoSummary>, AppError> {
    tracing::info!("[handler::get_repo] repo_id={id}");

    let summary = svc
        .repo
        .get_repo_with_collection_name(id)
        .await
        .map_err(|e| {
            tracing::error!("[handler::get_repo] failed repo_id={id} error={e}");
            e
        })?;

    Ok(Json(summary))
}

/// Trigger a sync for a Git repository.
///
/// Endpoint: `POST /api/git-sync/repos/{id}/sync`
///
/// Runs the full clone/pull → parse → embed → index pipeline synchronously.
/// For large repositories this may take significant time; future iterations
/// may use a `202 Accepted` pattern with polling.
pub async fn trigger_sync(
    State(svc): State<GitSyncService>,
    Path(id): Path<Uuid>,
) -> Result<Json<SyncStatusResponse>, AppError> {
    tracing::info!("[handler::trigger_sync] repo_id={id}");

    let response = svc.sync_repo(id).await.map_err(|e| {
        tracing::error!("[handler::trigger_sync] failed repo_id={id} error={e}");
        e
    })?;

    tracing::info!(
        "[handler::trigger_sync] completed repo_id={id} status={} files={} chunks={}",
        response.status,
        response.files_indexed,
        response.chunks_total
    );
    Ok(Json(response))
}

/// Get the current sync status of a Git repository.
///
/// Endpoint: `GET /api/git-sync/repos/{id}/status`
///
/// Returns the persisted status from the database. During an active sync the
/// status will be `"syncing"`; only after completion does it become `"idle"`.
pub async fn get_sync_status(
    State(svc): State<GitSyncService>,
    Path(id): Path<Uuid>,
) -> Result<Json<SyncStatusResponse>, AppError> {
    tracing::info!("[handler::get_sync_status] repo_id={id}");

    let repo = svc.repo.get_repo(id).await.map_err(|e| {
        tracing::error!("[handler::get_sync_status] failed repo_id={id} error={e}");
        e
    })?;

    tracing::debug!(
        "[handler::get_sync_status] repo_id={id} status={} commit={:?}",
        repo.status,
        repo.last_commit_hash
    );

    Ok(Json(SyncStatusResponse {
        repo_id: id,
        status: repo.status,
        files_indexed: 0,
        chunks_total: 0,
        last_commit: repo.last_commit_hash,
        error: None,
    }))
}

/// Delete a Git repository and clean up all associated data.
///
/// Endpoint: `DELETE /api/git-sync/repos/{id}`
///
/// Removes the local clone, deletes the Chroma collection, and removes the
/// SQLite record.
pub async fn delete_repo(
    State(svc): State<GitSyncService>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("[handler::delete_repo] repo_id={id}");

    svc.delete_repo_and_cleanup(id).await.map_err(|e| {
        tracing::error!("[handler::delete_repo] failed repo_id={id} error={e}");
        e
    })?;

    tracing::info!("[handler::delete_repo] deleted repo_id={id}");
    Ok(Json(json!({"status": "deleted", "id": id})))
}
