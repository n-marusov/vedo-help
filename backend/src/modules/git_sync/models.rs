use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A registered Git repository tracked for document syncing.
///
/// Stores connection details (URL, token), sync state, and the linked
/// Chroma collection. Sensitive fields (`access_token`, `webhook_secret`)
/// are never serialized in API responses — use `GitRepoSummary` for that.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GitRepo {
    pub id: Uuid,
    pub url: String,
    pub branch: String,
    /// HTTPS personal access token — NEVER logged, NEVER serialized in responses.
    pub access_token: Option<String>,
    /// Absolute path to the local clone on disk.
    pub local_path: String,
    pub last_commit_hash: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub collection_id: Uuid,
    /// One of `"idle"`, `"syncing"`, `"error"`.
    pub status: String,
    pub webhook_secret: Option<String>,
    /// The KeyCloak user `sub` that owns this repo.
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request payload for registering a new Git repository.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRepoRequest {
    pub url: String,
    pub branch: Option<String>,
    pub access_token: Option<String>,
    pub collection_id: Uuid,
}

/// Public summary of a Git repository, safe for API responses.
///
/// Excludes sensitive fields (`access_token`, `webhook_secret`).
/// Includes the resolved `collection_name` via a JOIN query.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GitRepoSummary {
    pub id: Uuid,
    pub url: String,
    pub branch: String,
    /// Internal filesystem path — excluded from API responses.
    #[serde(skip_serializing)]
    pub local_path: String,
    pub last_commit_hash: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub collection_id: Uuid,
    pub collection_name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<GitRepo> for GitRepoSummary {
    fn from(repo: GitRepo) -> Self {
        tracing::debug!(
            component = "git_sync/models",
            git_repo_id = %repo.id,
            "GitRepoSummary.strip_sensitive_fields"
        );

        Self {
            id: repo.id,
            url: repo.url,
            branch: repo.branch,
            local_path: repo.local_path,
            last_commit_hash: repo.last_commit_hash,
            last_synced_at: repo.last_synced_at,
            collection_id: repo.collection_id,
            // collection_name is resolved via JOIN — unavailable from GitRepo alone.
            // The caller must populate it separately.
            collection_name: String::new(),
            status: repo.status,
            created_at: repo.created_at,
            updated_at: repo.updated_at,
        }
    }
}

/// Tracks real-time sync progress for the frontend progress bar.
///
/// - `phase`: `"cloning"` during git clone, `"indexing"` during embedding generation.
/// - `total_files`: total documents to index — determined after cloning, set before indexing.
/// - `indexed_files`: how many have been processed so far.
/// - `current_file`: the file currently being embedded (shown in the UI).
#[derive(Debug, Clone, Serialize)]
pub struct SyncProgress {
    pub total_files: usize,
    pub indexed_files: usize,
    pub current_file: String,
    pub phase: String,
}

/// Response returned by the sync endpoint, reporting sync results.
#[derive(Debug, Clone, Serialize)]
pub struct SyncStatusResponse {
    pub repo_id: Uuid,
    pub status: String,
    pub files_indexed: usize,
    pub chunks_total: usize,
    pub last_commit: Option<String>,
    pub error: Option<String>,
    /// Real-time progress of an active sync — `None` when idle or error.
    pub progress: Option<SyncProgress>,
}

/// Payload received from a Git provider (GitHub / GitLab / Bitbucket)
/// when a push event triggers a webhook.
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookPayload {
    pub repo_id: Uuid,
    pub event: String,
    pub ref_name: String,
    pub before: Option<String>,
    pub after: String,
}
