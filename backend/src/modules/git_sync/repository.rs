use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::git_sync::models::{GitRepo, GitRepoSummary};
use crate::shared::error::AppError;

#[derive(sqlx::FromRow)]
struct GitRepoRow {
    id: String,
    url: String,
    branch: String,
    access_token: Option<String>,
    local_path: String,
    last_commit_hash: Option<String>,
    last_synced_at: Option<String>,
    collection_id: String,
    status: String,
    webhook_secret: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct GitRepoSummaryRow {
    id: String,
    url: String,
    branch: String,
    local_path: String,
    last_commit_hash: Option<String>,
    last_synced_at: Option<String>,
    collection_id: String,
    collection_name: String,
    status: String,
    created_at: String,
    updated_at: String,
}

fn parse_uuid_field(value: &str, field: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|e| {
        tracing::error!(
            "[FIX:git-repo-uuid-text] invalid UUID in git_repositories.{field}: value={value} error={e}"
        );
        AppError::InternalError(format!("Invalid git repository {field}: {e}"))
    })
}

fn parse_datetime_field(value: &str, field: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            tracing::error!(
                "[FIX:git-repo-uuid-text] invalid timestamp in git_repositories.{field}: value={value} error={e}"
            );
            AppError::InternalError(format!("Invalid git repository {field}: {e}"))
        })
}

fn parse_optional_datetime_field(
    value: Option<String>,
    field: &str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    value
        .as_deref()
        .map(|value| parse_datetime_field(value, field))
        .transpose()
}

impl TryFrom<GitRepoRow> for GitRepo {
    type Error = AppError;

    fn try_from(row: GitRepoRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: parse_uuid_field(&row.id, "id")?,
            url: row.url,
            branch: row.branch,
            access_token: row.access_token,
            local_path: row.local_path,
            last_commit_hash: row.last_commit_hash,
            last_synced_at: parse_optional_datetime_field(row.last_synced_at, "last_synced_at")?,
            collection_id: parse_uuid_field(&row.collection_id, "collection_id")?,
            status: row.status,
            webhook_secret: row.webhook_secret,
            created_at: parse_datetime_field(&row.created_at, "created_at")?,
            updated_at: parse_datetime_field(&row.updated_at, "updated_at")?,
        })
    }
}

impl TryFrom<GitRepoSummaryRow> for GitRepoSummary {
    type Error = AppError;

    fn try_from(row: GitRepoSummaryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: parse_uuid_field(&row.id, "id")?,
            url: row.url,
            branch: row.branch,
            local_path: row.local_path,
            last_commit_hash: row.last_commit_hash,
            last_synced_at: parse_optional_datetime_field(row.last_synced_at, "last_synced_at")?,
            collection_id: parse_uuid_field(&row.collection_id, "collection_id")?,
            collection_name: row.collection_name,
            status: row.status,
            created_at: parse_datetime_field(&row.created_at, "created_at")?,
            updated_at: parse_datetime_field(&row.updated_at, "updated_at")?,
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

    /// Insert a new Git repository record.
    pub async fn create_repo(&self, repo: &GitRepo) -> Result<Uuid, AppError> {
        tracing::debug!(
            "[GitRepoRepository::create_repo] entry repo_id={} url={}",
            repo.id,
            repo.url
        );

        sqlx::query(
            r#"
            INSERT INTO git_repositories
                (id, url, branch, access_token, local_path, last_commit_hash,
                 last_synced_at, collection_id, status, webhook_secret, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(repo.id.to_string())
        .bind(&repo.url)
        .bind(&repo.branch)
        .bind(&repo.access_token)
        .bind(&repo.local_path)
        .bind(&repo.last_commit_hash)
        .bind(repo.last_synced_at.map(|t| t.to_rfc3339()))
        .bind(repo.collection_id.to_string())
        .bind(&repo.status)
        .bind(&repo.webhook_secret)
        .bind(repo.created_at.to_rfc3339())
        .bind(repo.updated_at.to_rfc3339())
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                "[GitRepoRepository::create_repo] SQL error: error={e}, query=INSERT git_repositories"
            );
            AppError::InternalError(format!("Failed to create git repository: {e}"))
        })?;

        tracing::debug!("[GitRepoRepository::create_repo] exit repo_id={}", repo.id);
        Ok(repo.id)
    }

    /// List all registered Git repositories.
    pub async fn list_repos(&self) -> Result<Vec<GitRepo>, AppError> {
        tracing::debug!("[GitRepoRepository::list_repos] entry");

        let rows = sqlx::query_as::<_, GitRepoRow>(
            r#"
            SELECT id, url, branch, access_token, local_path, last_commit_hash,
                   last_synced_at, collection_id, status, webhook_secret, created_at, updated_at
            FROM git_repositories
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                "[GitRepoRepository::list_repos] SQL error: error={e}, query=SELECT git_repositories"
            );
            AppError::InternalError(format!("Failed to list git repositories: {e}"))
        })?;

        let repos = rows
            .into_iter()
            .map(GitRepo::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        tracing::debug!("[GitRepoRepository::list_repos] exit count={}", repos.len());
        Ok(repos)
    }

    /// Retrieve a single Git repository by ID.
    ///
    /// Returns `AppError::NotFound` if no row exists.
    pub async fn get_repo(&self, id: Uuid) -> Result<GitRepo, AppError> {
        tracing::debug!("[GitRepoRepository::get_repo] entry repo_id={id}");

        let repo = sqlx::query_as::<_, GitRepoRow>(
            r#"
            SELECT id, url, branch, access_token, local_path, last_commit_hash,
                   last_synced_at, collection_id, status, webhook_secret, created_at, updated_at
            FROM git_repositories
            WHERE id = $1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                "[GitRepoRepository::get_repo] SQL error: error={e}, query=SELECT git_repositories WHERE id"
            );
            AppError::InternalError(format!("Database error: {e}"))
        })?
        .ok_or_else(|| {
            tracing::debug!("[GitRepoRepository::get_repo] not found repo_id={id}");
            AppError::NotFound(format!("Git repository {id} not found"))
        })?;

        let repo = GitRepo::try_from(repo)?;

        tracing::debug!("[GitRepoRepository::get_repo] exit repo_id={id}");
        Ok(repo)
    }

    /// Retrieve a single repo with its resolved collection name.
    pub async fn get_repo_with_collection_name(
        &self,
        id: Uuid,
    ) -> Result<GitRepoSummary, AppError> {
        tracing::debug!("[GitRepoRepository::get_repo_with_collection_name] entry repo_id={id}");

        let summary = sqlx::query_as::<_, GitRepoSummaryRow>(
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
        .bind(id.to_string())
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                "[GitRepoRepository::get_repo_with_collection_name] SQL error: error={e}, query=SELECT JOIN"
            );
            AppError::InternalError(format!("Database error: {e}"))
        })?
        .ok_or_else(|| {
            tracing::debug!(
                "[GitRepoRepository::get_repo_with_collection_name] not found repo_id={id}"
            );
            AppError::NotFound(format!("Git repository {id} not found"))
        })?;

        let summary = GitRepoSummary::try_from(summary)?;

        tracing::debug!("[GitRepoRepository::get_repo_with_collection_name] exit repo_id={id}");
        Ok(summary)
    }

    /// List all repos with their resolved collection names.
    pub async fn list_repos_with_collection_names(&self) -> Result<Vec<GitRepoSummary>, AppError> {
        tracing::debug!("[GitRepoRepository::list_repos_with_collection_names] entry");

        let rows = sqlx::query_as::<_, GitRepoSummaryRow>(
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
                "[GitRepoRepository::list_repos_with_collection_names] SQL error: error={e}, query=SELECT JOIN all"
            );
            AppError::InternalError(format!("Database error: {e}"))
        })?;

        let summaries = rows
            .into_iter()
            .map(GitRepoSummary::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        tracing::debug!(
            "[GitRepoRepository::list_repos_with_collection_names] exit count={}",
            summaries.len()
        );
        Ok(summaries)
    }

    /// Update the sync status, commit hash, and last synced timestamp.
    pub async fn update_sync_status(
        &self,
        id: Uuid,
        commit_hash: &str,
        synced_at: &DateTime<Utc>,
        status: &str,
    ) -> Result<(), AppError> {
        tracing::debug!(
            "[GitRepoRepository::update_sync_status] entry repo_id={id} status={status}"
        );

        let now = Utc::now().to_rfc3339();
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
        .bind(synced_at.to_rfc3339())
        .bind(status)
        .bind(&now)
        .bind(id.to_string())
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                "[GitRepoRepository::update_sync_status] SQL error: error={e}, \
                 query=UPDATE git_repositories SET status"
            );
            AppError::InternalError(format!("Failed to update sync status: {e}"))
        })?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Git repository {id} not found")));
        }

        tracing::debug!(
            "[GitRepoRepository::update_sync_status] exit repo_id={id} status={status}"
        );
        Ok(())
    }

    /// Delete a Git repository record by ID.
    ///
    /// Returns `AppError::NotFound` if no row exists.
    pub async fn delete_repo(&self, id: Uuid) -> Result<(), AppError> {
        tracing::debug!("[GitRepoRepository::delete_repo] entry repo_id={id}");

        let affected = sqlx::query("DELETE FROM git_repositories WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    "[GitRepoRepository::delete_repo] SQL error: error={e}, \
                     query=DELETE git_repositories"
                );
                AppError::InternalError(format!("Failed to delete git repository: {e}"))
            })?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Git repository {id} not found")));
        }

        tracing::debug!("[GitRepoRepository::delete_repo] exit repo_id={id}");
        Ok(())
    }

    /// Look up a collection name by its ID.
    pub async fn get_collection_name(&self, collection_id: Uuid) -> Result<String, AppError> {
        tracing::debug!(
            "[GitRepoRepository::get_collection_name] entry collection_id={collection_id}"
        );

        let name: Option<String> = sqlx::query_scalar("SELECT name FROM collections WHERE id = $1")
            .bind(collection_id.to_string())
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                tracing::error!("[GitRepoRepository::get_collection_name] SQL error: error={e}");
                AppError::InternalError(format!("Database error: {e}"))
            })?;

        let name = name.ok_or_else(|| {
            tracing::debug!(
                "[GitRepoRepository::get_collection_name] not found collection_id={collection_id}"
            );
            AppError::NotFound(format!("Collection {collection_id} not found"))
        })?;

        tracing::debug!(
            "[GitRepoRepository::get_collection_name] exit collection_id={collection_id} name={name}"
        );
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    // Tests migrated to sqlx::test with PostgreSQL fixtures (Phase 3)
}
