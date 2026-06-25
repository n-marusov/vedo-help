use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use uuid::Uuid;

use crate::modules::auth::models::UserContext;
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
    user_ctx: UserContext,
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
        component = "git_sync/handlers",
        repo_url = %req.url,
        collection_id = %req.collection_id,
        "create_repo.creating"
    );
    tracing::debug!(
        component = "git_sync/handlers",
        branch = %req.branch.as_deref().unwrap_or("none"),
        has_token = req.access_token.is_some(),
        "create_repo.params"
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
        user_id: user_ctx.user_id.clone(),
        created_at: now,
        updated_at: now,
    };

    svc.repo.create_repo(&repo).await.map_err(|e| {
        tracing::error!(component = "git_sync/handlers", git_repo_id = %id, error = %e, "create_repo.failed");
        e
    })?;

    let summary = svc
        .repo
        .get_repo_with_collection_name(id)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "git_sync/handlers",
                git_repo_id = %id,
                error = %e,
                "create_repo.fetch_summary.failed"
            );
            e
        })?;

    tracing::info!(component = "git_sync/handlers", git_repo_id = %id, "create_repo.created");
    Ok(Json(summary))
}

/// List all registered Git repositories with their collection names.
///
/// Endpoint: `GET /api/git-sync/repos`
pub async fn list_repos(
    user_ctx: UserContext,
    State(svc): State<GitSyncService>,
) -> Result<Json<Vec<GitRepoSummary>>, AppError> {
    tracing::info!(component = "git_sync/handlers", user_id = %user_ctx.user_id, "list_repos");

    let repos = svc
        .repo
        .list_repos_with_collection_names_for_user(&user_ctx.user_id, user_ctx.is_admin())
        .await
        .map_err(|e| {
            tracing::error!(component = "git_sync/handlers", error = %e, "list_repos.failed");
            e
        })?;

    tracing::debug!(
        component = "git_sync/handlers",
        count = repos.len(),
        "list_repos.result"
    );
    Ok(Json(repos))
}

/// Get a single Git repository by ID.
///
/// Endpoint: `GET /api/git-sync/repos/{id}`
pub async fn get_repo(
    user_ctx: UserContext,
    State(svc): State<GitSyncService>,
    Path(id): Path<Uuid>,
) -> Result<Json<GitRepoSummary>, AppError> {
    tracing::info!(component = "git_sync/handlers", git_repo_id = %id, user_id = %user_ctx.user_id, "get_repo");

    let summary = svc
        .repo
        .get_repo_with_collection_name_for_user(id, &user_ctx.user_id, user_ctx.is_admin())
        .await
        .map_err(|e| {
            tracing::error!(component = "git_sync/handlers", git_repo_id = %id, error = %e, "get_repo.failed");
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
    user_ctx: UserContext,
    State(svc): State<GitSyncService>,
    Path(id): Path<Uuid>,
) -> Result<Json<SyncStatusResponse>, AppError> {
    tracing::info!(component = "git_sync/handlers", git_repo_id = %id, user_id = %user_ctx.user_id, "trigger_sync");

    let response = svc.sync_repo(id, &user_ctx.user_id, user_ctx.is_admin()).await.map_err(|e| {
        tracing::error!(component = "git_sync/handlers", git_repo_id = %id, error = %e, "trigger_sync.failed");
        e
    })?;

    tracing::info!(
        component = "git_sync/handlers",
        git_repo_id = %id,
        status = %response.status,
        files_indexed = response.files_indexed,
        chunks_total = response.chunks_total,
        "trigger_sync.completed"
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
    user_ctx: UserContext,
    State(svc): State<GitSyncService>,
    Path(id): Path<Uuid>,
) -> Result<Json<SyncStatusResponse>, AppError> {
    tracing::info!(component = "git_sync/handlers", git_repo_id = %id, user_id = %user_ctx.user_id, "get_sync_status");

    let repo = svc.repo.get_repo_for_user(id, &user_ctx.user_id, user_ctx.is_admin()).await.map_err(|e| {
        tracing::error!(component = "git_sync/handlers", git_repo_id = %id, error = %e, "get_sync_status.failed");
        e
    })?;

    tracing::debug!(
        component = "git_sync/handlers",
        git_repo_id = %id,
        status = %repo.status,
        commit = %repo.last_commit_hash.as_deref().unwrap_or("none"),
        "get_sync_status.result"
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
/// PostgreSQL record.
pub async fn delete_repo(
    user_ctx: UserContext,
    State(svc): State<GitSyncService>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(component = "git_sync/handlers", git_repo_id = %id, user_id = %user_ctx.user_id, "delete_repo");

    svc.delete_repo_and_cleanup(id, &user_ctx.user_id, user_ctx.is_admin()).await.map_err(|e| {
        tracing::error!(component = "git_sync/handlers", git_repo_id = %id, error = %e, "delete_repo.failed");
        e
    })?;

    tracing::info!(component = "git_sync/handlers", git_repo_id = %id, "delete_repo.deleted");
    Ok(Json(json!({"status": "deleted", "id": id})))
}

/// Handle incoming webhook events from Git providers.
///
/// Endpoint: `POST /api/git-sync/webhook`
///
/// Supports GitHub (X-Hub-Signature-256) and GitLab (X-Gitlab-Token) webhook
/// authentication. The webhook secret is stored per-repo in the database.
///
/// Returns `202 Accepted` immediately after spawning an async sync task.
pub async fn webhook(
    State(svc): State<GitSyncService>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(component = "git_sync/handlers", "webhook.received");

    // Parse the raw body as a generic JSON value first to extract repo_id
    let payload: serde_json::Value = serde_json::from_slice(&body).map_err(|e| {
        tracing::warn!(component = "git_sync/handlers", error = %e, "webhook.invalid_json");
        AppError::BadRequest(format!("Invalid JSON body: {e}"))
    })?;

    // Extract repo_id from the payload
    let repo_id_str = payload
        .get("repo_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            tracing::warn!(component = "git_sync/handlers", "webhook.missing_repo_id");
            AppError::BadRequest("Missing repo_id in payload".to_string())
        })?;

    let repo_id: Uuid = repo_id_str.parse().map_err(|e| {
        tracing::warn!(component = "git_sync/handlers", error = %e, "webhook.invalid_repo_id");
        AppError::BadRequest(format!("Invalid repo_id: {e}"))
    })?;

    // Get the repo from DB
    let repo = match svc.repo.get_repo(repo_id).await {
        Ok(r) => r,
        Err(AppError::NotFound(_)) => {
            tracing::warn!(component = "git_sync/handlers", git_repo_id = %repo_id, "webhook.repo_not_found");
            return Err(AppError::NotFound(format!(
                "Git repository {repo_id} not found"
            )));
        }
        Err(e) => return Err(e),
    };

    // Determine event type
    let gh_event = headers
        .get("X-GitHub-Event")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let gl_event = headers
        .get("X-Gitlab-Event")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let event_source = if gh_event.is_some() {
        "github"
    } else if gl_event.is_some() {
        "gitlab"
    } else {
        "unknown"
    };

    // Only respond to push events
    let is_push = match &gh_event {
        Some(e) if e == "push" => true,
        _ => matches!(&gl_event, Some(e) if e == "Push Hook"),
    };

    if !is_push {
        tracing::debug!(
            component = "git_sync/handlers",
            gh_event = %gh_event.as_deref().unwrap_or("none"),
            gl_event = %gl_event.as_deref().unwrap_or("none"),
            "webhook.non_push_event"
        );
        return Ok(Json(
            json!({"status": "skipped", "reason": "non-push event"}),
        ));
    }

    // Validate signature/token
    if let Some(ref secret) = repo.webhook_secret {
        match event_source {
            "github" => {
                let signature = headers
                    .get("X-Hub-Signature-256")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                match signature {
                    Some(sig) if !verify_hmac_signature(&body, secret, &sig) => {
                        tracing::warn!(
                            component = "git_sync/handlers",
                            git_repo_id = %repo_id,
                            "webhook.github_signature_mismatch"
                        );
                        return Err(AppError::Unauthorized(
                            "Invalid webhook signature".to_string(),
                        ));
                    }
                    Some(_) => {
                        tracing::debug!(
                            component = "git_sync/handlers",
                            git_repo_id = %repo_id,
                            "webhook.github_signature_valid"
                        );
                    }
                    None => {
                        tracing::warn!(
                            component = "git_sync/handlers",
                            git_repo_id = %repo_id,
                            "webhook.missing_github_signature"
                        );
                        return Err(AppError::Unauthorized(
                            "Missing webhook signature".to_string(),
                        ));
                    }
                }
            }
            "gitlab" => {
                let token = headers
                    .get("X-Gitlab-Token")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                match token {
                    Some(t) if t != *secret => {
                        tracing::warn!(
                            component = "git_sync/handlers",
                            git_repo_id = %repo_id,
                            "webhook.gitlab_token_mismatch"
                        );
                        return Err(AppError::Unauthorized("Invalid webhook token".to_string()));
                    }
                    Some(_) => {
                        tracing::debug!(component = "git_sync/handlers", git_repo_id = %repo_id, "webhook.gitlab_token_valid");
                    }
                    None => {
                        tracing::warn!(
                            component = "git_sync/handlers",
                            git_repo_id = %repo_id,
                            "webhook.missing_gitlab_token"
                        );
                        return Err(AppError::Unauthorized("Missing webhook token".to_string()));
                    }
                }
            }
            _ => {
                tracing::warn!(
                    component = "git_sync/handlers",
                    git_repo_id = %repo_id,
                    "webhook.unknown_event_source"
                );
            }
        }
    } else {
        tracing::warn!(
            component = "git_sync/handlers",
            git_repo_id = %repo_id,
            "webhook.no_secret"
        );
    }

    // Extract ref_name from the payload
    let ref_name = payload
        .get("ref")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Strip refs/heads/ prefix to get the branch name
    let ref_branch = ref_name.strip_prefix("refs/heads/").unwrap_or(&ref_name);

    // Check if the branch matches the repo's configured branch
    if !ref_branch.is_empty() && ref_branch != repo.branch {
        tracing::info!(
            component = "git_sync/handlers",
            git_repo_id = %repo_id,
            branch = %ref_branch,
            expected_branch = %repo.branch,
            "webhook.branch_mismatch"
        );
        return Ok(Json(json!({
            "status": "skipped",
            "reason": "branch mismatch"
        })));
    }

    tracing::info!(
        component = "git_sync/handlers",
        git_repo_id = %repo_id,
        ref_name = %ref_name,
        source = %event_source,
        "webhook.triggering_sync"
    );

    // Spawn async sync task and return 202 Accepted
    let svc_clone = svc.clone();
    tokio::spawn(async move {
        if let Err(e) = svc_clone.sync_repo_internal(repo_id).await {
            tracing::error!(component = "git_sync/handlers", git_repo_id = %repo_id, error = %e, "webhook.sync_failed");
        }
    });

    Ok(Json(json!({"status": "accepted", "repo_id": repo_id})))
}

/// Verify an HMAC-SHA256 signature against the request body and secret.
///
/// The signature from the `X-Hub-Signature-256` header has the format
/// `sha256=<hex_digest>`.
fn verify_hmac_signature(body: &[u8], secret: &str, signature_header: &str) -> bool {
    // Extract the hex digest from the header value (strip "sha256=" prefix)
    let expected_digest = signature_header.strip_prefix("sha256=").unwrap_or("");
    if expected_digest.is_empty() {
        tracing::warn!(
            component = "git_sync/handlers",
            "webhook.verify_hmac.invalid_header_format"
        );
        return false;
    }

    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(component = "git_sync/handlers", error = %e, "webhook.verify_hmac.create_failed");
            return false;
        }
    };

    mac.update(body);

    // Compute the expected hex digest (used for logging if needed)
    let _computed = hex::encode(mac.finalize().into_bytes());

    // Constant-time comparison to prevent timing attacks
    // Use the HMAC library's verify_slice for constant-time comparison
    let expected_bytes = match hex::decode(expected_digest) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(component = "git_sync/handlers", error = %e, "webhook.verify_hmac.invalid_hex_digest");
            return false;
        }
    };

    // Use constant-time verification
    let result = Hmac::<Sha256>::new_from_slice(secret.as_bytes());
    match result {
        Ok(mut mac) => {
            mac.update(body);
            mac.verify_slice(&expected_bytes).is_ok()
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_hmac_signature_valid() {
        let body = b"{\"test\": true}";
        let secret = "my-webhook-secret";

        // Compute a valid signature
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let digest = hex::encode(mac.finalize().into_bytes());
        let header = format!("sha256={digest}");

        assert!(verify_hmac_signature(body, secret, &header));
    }

    #[test]
    fn test_verify_hmac_signature_invalid() {
        let body = b"{\"test\": true}";
        let secret = "my-webhook-secret";
        let bad_header = "sha256=0000000000000000000000000000000000000000000000000000000000000000";

        assert!(!verify_hmac_signature(body, secret, bad_header));
    }

    #[test]
    fn test_verify_hmac_signature_bad_prefix() {
        let body = b"test";
        let secret = "secret";

        assert!(!verify_hmac_signature(body, secret, ""));
        assert!(!verify_hmac_signature(body, secret, "sha256="));
        assert!(!verify_hmac_signature(body, secret, "invalid"));
    }

    #[test]
    fn test_verify_hmac_signature_wrong_secret() {
        let body = b"{\"test\": true}";
        let good_secret = "my-webhook-secret";
        let wrong_secret = "wrong-secret";

        // Compute a signature with the good secret
        let mut mac = Hmac::<Sha256>::new_from_slice(good_secret.as_bytes()).unwrap();
        mac.update(body);
        let digest = hex::encode(mac.finalize().into_bytes());
        let header = format!("sha256={digest}");

        // Verify with the wrong secret — should fail
        assert!(!verify_hmac_signature(body, wrong_secret, &header));
    }
}
