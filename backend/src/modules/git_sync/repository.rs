use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::git_sync::models::{GitRepo, GitRepoSummary};
use crate::shared::error::AppError;

#[derive(sqlx::FromRow)]
struct GitRepoRow {
    id: Uuid,
    url: String,
    branch: String,
    access_token: Option<String>,
    local_path: String,
    last_commit_hash: Option<String>,
    last_synced_at: Option<DateTime<Utc>>,
    collection_id: Uuid,
    status: String,
    webhook_secret: Option<String>,
    user_id: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct GitRepoSummaryRow {
    id: Uuid,
    url: String,
    branch: String,
    local_path: String,
    last_commit_hash: Option<String>,
    last_synced_at: Option<DateTime<Utc>>,
    collection_id: Uuid,
    collection_name: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<GitRepoRow> for GitRepo {
    type Error = AppError;

    fn try_from(row: GitRepoRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            url: row.url,
            branch: row.branch,
            access_token: row.access_token,
            local_path: row.local_path,
            last_commit_hash: row.last_commit_hash,
            last_synced_at: row.last_synced_at,
            collection_id: row.collection_id,
            status: row.status,
            webhook_secret: row.webhook_secret,
            user_id: row.user_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl TryFrom<GitRepoSummaryRow> for GitRepoSummary {
    type Error = AppError;

    fn try_from(row: GitRepoSummaryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            url: row.url,
            branch: row.branch,
            local_path: row.local_path,
            last_commit_hash: row.last_commit_hash,
            last_synced_at: row.last_synced_at,
            collection_id: row.collection_id,
            collection_name: row.collection_name,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Data access layer for `git_repositories` PostgreSQL table.
///
/// Provides CRUD operations plus JOIN queries that resolve collection names.
/// Sensitive fields (`access_token`, `webhook_secret`) are available from
/// `GitRepo` but are excluded from `GitRepoSummary` responses.
#[derive(Clone, Debug)]
pub struct GitRepoRepository {
    db: PgPool,
}

impl GitRepoRepository {
    /// Create a new repository with the given database pool.
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Get a reference to the underlying database pool.
    pub fn pool(&self) -> &PgPool {
        &self.db
    }

    /// Insert a new Git repository record.
    pub async fn create_repo(&self, repo: &GitRepo) -> Result<Uuid, AppError> {
        tracing::debug!(
            component = "git_sync/repository",
            git_repo_id = %repo.id,
            repo_url = %repo.url,
            "create_repo.entry"
        );

        sqlx::query(
            r#"
            INSERT INTO git_repositories
                (id, url, branch, access_token, local_path, last_commit_hash,
                 last_synced_at, collection_id, status, webhook_secret, user_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(repo.id)
        .bind(&repo.url)
        .bind(&repo.branch)
        .bind(&repo.access_token)
        .bind(&repo.local_path)
        .bind(&repo.last_commit_hash)
        .bind(repo.last_synced_at)
        .bind(repo.collection_id)
        .bind(&repo.status)
        .bind(&repo.webhook_secret)
        .bind(&repo.user_id)
        .bind(repo.created_at)
        .bind(repo.updated_at)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "git_sync/repository",
                error = %e,
                query = "INSERT git_repositories",
                "create_repo.sql_error"
            );
            AppError::InternalError(format!("Failed to create git repository: {e}"))
        })?;

        tracing::debug!(component = "git_sync/repository", git_repo_id = %repo.id, "create_repo.exit");
        Ok(repo.id)
    }

    /// List all registered Git repositories.
    /// Admin users see all repos; non-admin users see only their own.
    pub async fn list_repos_by_user(
        &self,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<GitRepo>, AppError> {
        tracing::debug!(component = "git_sync/repository", "list_repos.entry");

        let rows = if is_admin {
            sqlx::query_as::<_, GitRepoRow>(
                r#"
                SELECT id, url, branch, access_token, local_path, last_commit_hash,
                       last_synced_at, collection_id, status, webhook_secret, user_id, created_at, updated_at
                FROM git_repositories
                ORDER BY created_at DESC
                "#,
            )
            .fetch_all(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "git_sync/repository",
                    error = %e,
                    query = "SELECT git_repositories",
                    "list_repos.sql_error"
                );
                AppError::InternalError(format!("Failed to list git repositories: {e}"))
            })?
        } else {
            sqlx::query_as::<_, GitRepoRow>(
                r#"
                SELECT id, url, branch, access_token, local_path, last_commit_hash,
                       last_synced_at, collection_id, status, webhook_secret, user_id, created_at, updated_at
                FROM git_repositories
                WHERE user_id = $1
                ORDER BY created_at DESC
                "#,
            )
            .bind(user_id)
            .fetch_all(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "git_sync/repository",
                    error = %e,
                    query = "SELECT git_repositories WHERE user_id",
                    "list_repos.sql_error"
                );
                AppError::InternalError(format!("Failed to list git repositories: {e}"))
            })?
        };

        let repos = rows
            .into_iter()
            .map(GitRepo::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        tracing::debug!(
            component = "git_sync/repository",
            count = repos.len(),
            "list_repos.exit"
        );
        Ok(repos)
    }

    /// List all registered Git repositories (legacy, admin-only).
    pub async fn list_repos(&self) -> Result<Vec<GitRepo>, AppError> {
        self.list_repos_by_user("", true).await
    }

    /// Retrieve a single Git repository by ID with ownership check.
    /// Non-admin users can only access their own repos.
    pub async fn get_repo_for_user(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<GitRepo, AppError> {
        tracing::debug!(component = "git_sync/repository", git_repo_id = %id, "get_repo.entry");

        let repo = if is_admin {
            sqlx::query_as::<_, GitRepoRow>(
                r#"
                SELECT id, url, branch, access_token, local_path, last_commit_hash,
                       last_synced_at, collection_id, status, webhook_secret, user_id, created_at, updated_at
                FROM git_repositories
                WHERE id = $1
                "#,
            )
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "git_sync/repository",
                    error = %e,
                    query = "SELECT git_repositories WHERE id",
                    "get_repo.sql_error"
                );
                AppError::InternalError(format!("Database error: {e}"))
            })?
            .ok_or_else(|| {
                tracing::debug!(component = "git_sync/repository", git_repo_id = %id, "get_repo.not_found");
                AppError::NotFound(format!("Git repository {id} not found"))
            })?
        } else {
            sqlx::query_as::<_, GitRepoRow>(
                r#"
                SELECT id, url, branch, access_token, local_path, last_commit_hash,
                       last_synced_at, collection_id, status, webhook_secret, user_id, created_at, updated_at
                FROM git_repositories
                WHERE id = $1 AND user_id = $2
                "#,
            )
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "git_sync/repository",
                    error = %e,
                    query = "SELECT git_repositories WHERE id AND user_id",
                    "get_repo.sql_error"
                );
                AppError::InternalError(format!("Database error: {e}"))
            })?
            .ok_or_else(|| {
                tracing::debug!(component = "git_sync/repository", git_repo_id = %id, "get_repo.not_found");
                AppError::NotFound(format!("Git repository {id} not found"))
            })?
        };

        let repo = GitRepo::try_from(repo)?;

        tracing::debug!(component = "git_sync/repository", git_repo_id = %id, "get_repo.exit");
        Ok(repo)
    }

    /// Retrieve a single Git repository by ID (legacy, admin-only).
    ///
    /// Returns `AppError::NotFound` if no row exists.
    pub async fn get_repo(&self, id: Uuid) -> Result<GitRepo, AppError> {
        self.get_repo_for_user(id, "", true).await
    }

    /// Retrieve a single repo with its resolved collection name and ownership check.
    pub async fn get_repo_with_collection_name_for_user(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<GitRepoSummary, AppError> {
        tracing::debug!(component = "git_sync/repository", git_repo_id = %id, "get_repo_with_collection_name.entry");

        let summary = if is_admin {
            sqlx::query_as::<_, GitRepoSummaryRow>(
                r#"
                SELECT
                    g.id,
                    g.url,
                    g.branch,
                    g.local_path,
                    g.last_commit_hash,
                    g.last_synced_at,
                    g.collection_id,
                    COALESCE(c.name, '') AS collection_name,
                    g.status,
                    g.created_at,
                    g.updated_at
                FROM git_repositories g
                LEFT JOIN collections c ON g.collection_id = c.id
                WHERE g.id = $1
                "#,
            )
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "git_sync/repository",
                    error = %e,
                    query = "SELECT JOIN",
                    "get_repo_with_collection_name.sql_error"
                );
                AppError::InternalError(format!("Database error: {e}"))
            })?
            .ok_or_else(|| {
                tracing::debug!(
                    component = "git_sync/repository",
                    git_repo_id = %id,
                    "get_repo_with_collection_name.not_found"
                );
                AppError::NotFound(format!("Git repository {id} not found"))
            })?
        } else {
            sqlx::query_as::<_, GitRepoSummaryRow>(
                r#"
                SELECT
                    g.id,
                    g.url,
                    g.branch,
                    g.local_path,
                    g.last_commit_hash,
                    g.last_synced_at,
                    g.collection_id,
                    COALESCE(c.name, '') AS collection_name,
                    g.status,
                    g.created_at,
                    g.updated_at
                FROM git_repositories g
                LEFT JOIN collections c ON g.collection_id = c.id
                WHERE g.id = $1 AND g.user_id = $2
                "#,
            )
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "git_sync/repository",
                    error = %e,
                    query = "SELECT JOIN WHERE user_id",
                    "get_repo_with_collection_name.sql_error"
                );
                AppError::InternalError(format!("Database error: {e}"))
            })?
            .ok_or_else(|| {
                tracing::debug!(
                    component = "git_sync/repository",
                    git_repo_id = %id,
                    "get_repo_with_collection_name.not_found"
                );
                AppError::NotFound(format!("Git repository {id} not found"))
            })?
        };

        let summary = GitRepoSummary::try_from(summary)?;

        tracing::debug!(component = "git_sync/repository", git_repo_id = %id, "get_repo_with_collection_name.exit");
        Ok(summary)
    }

    /// Retrieve a single repo with its resolved collection name (legacy, admin-only).
    pub async fn get_repo_with_collection_name(
        &self,
        id: Uuid,
    ) -> Result<GitRepoSummary, AppError> {
        self.get_repo_with_collection_name_for_user(id, "", true)
            .await
    }

    /// List all repos with their resolved collection names, scoped by user.
    pub async fn list_repos_with_collection_names_for_user(
        &self,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<GitRepoSummary>, AppError> {
        tracing::debug!(
            component = "git_sync/repository",
            "list_repos_with_collection_names.entry"
        );

        let rows = if is_admin {
            sqlx::query_as::<_, GitRepoSummaryRow>(
                r#"
                SELECT
                    g.id,
                    g.url,
                    g.branch,
                    g.local_path,
                    g.last_commit_hash,
                    g.last_synced_at,
                    g.collection_id,
                    COALESCE(c.name, '') AS collection_name,
                    g.status,
                    g.created_at,
                    g.updated_at
                FROM git_repositories g
                LEFT JOIN collections c ON g.collection_id = c.id
                ORDER BY g.created_at DESC
                "#,
            )
            .fetch_all(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "git_sync/repository",
                    error = %e,
                    query = "SELECT JOIN all",
                    "list_repos_with_collection_names.sql_error"
                );
                AppError::InternalError(format!("Database error: {e}"))
            })?
        } else {
            sqlx::query_as::<_, GitRepoSummaryRow>(
                r#"
                SELECT
                    g.id,
                    g.url,
                    g.branch,
                    g.local_path,
                    g.last_commit_hash,
                    g.last_synced_at,
                    g.collection_id,
                    COALESCE(c.name, '') AS collection_name,
                    g.status,
                    g.created_at,
                    g.updated_at
                FROM git_repositories g
                LEFT JOIN collections c ON g.collection_id = c.id
                WHERE g.user_id = $1
                ORDER BY g.created_at DESC
                "#,
            )
            .bind(user_id)
            .fetch_all(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "git_sync/repository",
                    error = %e,
                    query = "SELECT JOIN all WHERE user_id",
                    "list_repos_with_collection_names.sql_error"
                );
                AppError::InternalError(format!("Database error: {e}"))
            })?
        };

        let summaries = rows
            .into_iter()
            .map(GitRepoSummary::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        tracing::debug!(
            component = "git_sync/repository",
            count = summaries.len(),
            "list_repos_with_collection_names.exit"
        );
        Ok(summaries)
    }

    /// List all repos with their resolved collection names (legacy, admin-only).
    pub async fn list_repos_with_collection_names(&self) -> Result<Vec<GitRepoSummary>, AppError> {
        self.list_repos_with_collection_names_for_user("", true)
            .await
    }

    /// Try to acquire a sync lock by atomically setting status to `"syncing"`.
    ///
    /// Returns `true` if the lock was acquired (previous status was not `"syncing"`),
    /// `false` if the repo was already being synced by another caller.
    /// This uses a compare-and-swap pattern to prevent race conditions.
    pub async fn try_acquire_sync_lock(&self, id: Uuid) -> Result<bool, AppError> {
        tracing::debug!(component = "git_sync/repository", git_repo_id = %id, "try_acquire_sync_lock.entry");

        let now = Utc::now();
        let result = sqlx::query(
            r#"
            UPDATE git_repositories
            SET status = 'syncing',
                updated_at = $1
            WHERE id = $2 AND status != 'syncing'
            "#,
        )
        .bind(now)
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "git_sync/repository",
                error = %e,
                query = "UPDATE git_repositories SET status = syncing WHERE id",
                "try_acquire_sync_lock.sql_error"
            );
            AppError::InternalError(format!("Failed to acquire sync lock: {e}"))
        })?;

        let acquired = result.rows_affected() > 0;
        tracing::debug!(
            component = "git_sync/repository",
            git_repo_id = %id,
            acquired = acquired,
            "try_acquire_sync_lock.exit"
        );
        Ok(acquired)
    }

    /// Release the sync lock and update status, commit hash, and last synced timestamp.
    pub async fn update_sync_status(
        &self,
        id: Uuid,
        commit_hash: &str,
        synced_at: &DateTime<Utc>,
        status: &str,
    ) -> Result<(), AppError> {
        tracing::debug!(
            component = "git_sync/repository",
            git_repo_id = %id,
            status = %status,
            "update_sync_status.entry"
        );

        let now = Utc::now();
        let affected = sqlx::query(
            r#"
            UPDATE git_repositories
            SET last_commit_hash = $1,
                last_synced_at = $2,
                status = $3,
                updated_at = $4
            WHERE id = $5
            "#,
        )
        .bind(commit_hash)
        .bind(synced_at)
        .bind(status)
        .bind(now)
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "git_sync/repository",
                error = %e,
                query = "UPDATE git_repositories SET status",
                "update_sync_status.sql_error"
            );
            AppError::InternalError(format!("Failed to update sync status: {e}"))
        })?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Git repository {id} not found")));
        }

        tracing::debug!(
            component = "git_sync/repository",
            git_repo_id = %id,
            status = %status,
            "update_sync_status.exit"
        );
        Ok(())
    }

    /// Mark a sync attempt as failed, preserving the old commit hash.
    ///
    /// Sets status to `"error"` and logs the error reason for frontend display.
    pub async fn mark_sync_error(&self, id: Uuid, error_message: &str) -> Result<(), AppError> {
        tracing::debug!(
            component = "git_sync/repository",
            git_repo_id = %id,
            error = %error_message,
            "mark_sync_error.entry"
        );

        let now = Utc::now();
        let affected = sqlx::query(
            r#"
            UPDATE git_repositories
            SET status = 'error',
                updated_at = $1
            WHERE id = $2
            "#,
        )
        .bind(now)
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "git_sync/repository",
                error = %e,
                query = "UPDATE git_repositories SET status = error",
                "mark_sync_error.sql_error"
            );
            AppError::InternalError(format!("Failed to mark sync error: {e}"))
        })?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Git repository {id} not found")));
        }

        tracing::debug!(component = "git_sync/repository", git_repo_id = %id, "mark_sync_error.exit");
        Ok(())
    }

    /// Delete a Git repository record by ID with ownership check.
    ///
    /// Returns `AppError::NotFound` if no row exists.
    pub async fn delete_repo_for_user(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<(), AppError> {
        tracing::debug!(component = "git_sync/repository", git_repo_id = %id, "delete_repo.entry");

        let affected = if is_admin {
            sqlx::query("DELETE FROM git_repositories WHERE id = $1")
                .bind(id)
                .execute(&self.db)
                .await
                .map_err(|e| {
                    tracing::error!(
                        component = "git_sync/repository",
                        error = %e,
                        query = "DELETE git_repositories",
                        "delete_repo.sql_error"
                    );
                    AppError::InternalError(format!("Failed to delete git repository: {e}"))
                })?
        } else {
            sqlx::query("DELETE FROM git_repositories WHERE id = $1 AND user_id = $2")
                .bind(id)
                .bind(user_id)
                .execute(&self.db)
                .await
                .map_err(|e| {
                    tracing::error!(
                        component = "git_sync/repository",
                        error = %e,
                        query = "DELETE git_repositories WHERE id AND user_id",
                        "delete_repo.sql_error"
                    );
                    AppError::InternalError(format!("Failed to delete git repository: {e}"))
                })?
        };

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Git repository {id} not found")));
        }

        tracing::debug!(component = "git_sync/repository", git_repo_id = %id, "delete_repo.exit");
        Ok(())
    }

    /// Delete a Git repository record by ID (legacy, admin-only).
    pub async fn delete_repo(&self, id: Uuid) -> Result<(), AppError> {
        self.delete_repo_for_user(id, "", true).await
    }

    /// Look up a collection name by its ID.
    /// Reset stale sync locks left by a previous process crash or restart.
    ///
    /// On server startup, any repository with `status = 'syncing'` has a
    /// stale lock — the process holding it is no longer alive. This method
    /// resets those repos back to `idle` so they can be retried.
    pub async fn reset_stale_sync_locks(&self) -> Result<u64, AppError> {
        tracing::warn!(
            component = "git_sync/repository",
            "reset_stale_sync_locks.start"
        );

        let result = sqlx::query(
            r#"
            UPDATE git_repositories
            SET status = 'idle',
                updated_at = $1
            WHERE status = 'syncing'
            "#,
        )
        .bind(Utc::now())
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "git_sync/repository",
                error = %e,
                "reset_stale_sync_locks.sql_error"
            );
            AppError::InternalError(format!("Failed to reset stale sync locks: {e}"))
        })?;

        let count = result.rows_affected();
        if count > 0 {
            tracing::warn!(
                component = "git_sync/repository",
                stale_locks_reset = count,
                "reset_stale_sync_locks.completed"
            );
        } else {
            tracing::debug!(
                component = "git_sync/repository",
                "reset_stale_sync_locks.none_found"
            );
        }

        Ok(count)
    }

    pub async fn get_collection_name(&self, collection_id: Uuid) -> Result<String, AppError> {
        tracing::debug!(
            component = "git_sync/repository",
            collection_id = %collection_id,
            "get_collection_name.entry"
        );

        let name: Option<String> = sqlx::query_scalar("SELECT name FROM collections WHERE id = $1")
            .bind(collection_id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(component = "git_sync/repository", error = %e, "get_collection_name.sql_error");
                AppError::InternalError(format!("Database error: {e}"))
            })?;

        let name = name.ok_or_else(|| {
            tracing::debug!(
                component = "git_sync/repository",
                collection_id = %collection_id,
                "get_collection_name.not_found"
            );
            AppError::NotFound(format!("Collection {collection_id} not found"))
        })?;

        tracing::debug!(
            component = "git_sync/repository",
            collection_id = %collection_id,
            collection_name = %name,
            "get_collection_name.exit"
        );
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    // Tests migrated to sqlx::test with PostgreSQL fixtures (Phase 3)
}
