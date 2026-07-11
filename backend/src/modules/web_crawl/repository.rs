use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::web_crawl::models::{CrawlJob, CrawlJobSummary, CrawlPage};
use crate::shared::error::AppError;

#[derive(sqlx::FromRow)]
struct CrawlJobRow {
    id: Uuid,
    entry_url: String,
    config: serde_json::Value,
    status: String,
    pages_found: i32,
    pages_indexed: i32,
    collection_id: Uuid,
    user_id: String,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct CrawlJobSummaryRow {
    id: Uuid,
    entry_url: String,
    config: serde_json::Value,
    status: String,
    pages_found: i32,
    pages_indexed: i32,
    collection_id: Uuid,
    collection_name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<CrawlJobRow> for CrawlJob {
    type Error = AppError;

    fn try_from(row: CrawlJobRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            entry_url: row.entry_url,
            config: row.config,
            status: row.status,
            pages_found: row.pages_found,
            pages_indexed: row.pages_indexed,
            collection_id: row.collection_id,
            user_id: row.user_id,
            error_message: row.error_message,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl TryFrom<CrawlJobSummaryRow> for CrawlJobSummary {
    type Error = AppError;

    fn try_from(row: CrawlJobSummaryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            entry_url: row.entry_url,
            config: row.config,
            status: row.status,
            pages_found: row.pages_found,
            pages_indexed: row.pages_indexed,
            collection_id: row.collection_id,
            collection_name: row.collection_name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Clone)]
pub struct WebCrawlRepository {
    db: PgPool,
}

impl WebCrawlRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Get a reference to the underlying database pool.
    pub fn pool(&self) -> &PgPool {
        &self.db
    }

    /// Insert a new crawl job.
    pub async fn create_job(&self, job: &CrawlJob) -> Result<Uuid, AppError> {
        tracing::debug!(
            component = "web_crawl/repository",
            crawl_job_id = %job.id,
            entry_url = %job.entry_url,
            "create_job.entry"
        );

        sqlx::query(
            r#"
            INSERT INTO web_crawl_jobs
                (id, entry_url, config, status, pages_found, pages_indexed,
                 collection_id, user_id, error_message, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(job.id)
        .bind(&job.entry_url)
        .bind(&job.config)
        .bind(&job.status)
        .bind(job.pages_found)
        .bind(job.pages_indexed)
        .bind(job.collection_id)
        .bind(&job.user_id)
        .bind(&job.error_message)
        .bind(job.created_at)
        .bind(job.updated_at)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "web_crawl/repository",
                error = %e,
                query = "INSERT web_crawl_jobs",
                "create_job.sql_error"
            );
            AppError::InternalError(format!("Failed to create crawl job: {e}"))
        })?;

        tracing::debug!(component = "web_crawl/repository", crawl_job_id = %job.id, "create_job.exit");
        Ok(job.id)
    }

    /// List crawl jobs for a user. Admin sees all jobs.
    pub async fn list_jobs_by_user(
        &self,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<CrawlJobSummary>, AppError> {
        tracing::debug!(component = "web_crawl/repository", "list_jobs.entry");

        let rows = if is_admin {
            sqlx::query_as::<_, CrawlJobSummaryRow>(
                r#"
                SELECT j.id, j.entry_url, j.config, j.status, j.pages_found,
                       j.pages_indexed, j.collection_id, c.name AS collection_name,
                       j.created_at, j.updated_at
                FROM web_crawl_jobs j
                JOIN collections c ON c.id = j.collection_id
                ORDER BY j.created_at DESC
                "#,
            )
            .fetch_all(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "web_crawl/repository",
                    error = %e,
                    query = "SELECT web_crawl_jobs (admin)",
                    "list_jobs.sql_error"
                );
                AppError::InternalError(format!("Failed to list crawl jobs: {e}"))
            })?
        } else {
            sqlx::query_as::<_, CrawlJobSummaryRow>(
                r#"
                SELECT j.id, j.entry_url, j.config, j.status, j.pages_found,
                       j.pages_indexed, j.collection_id, c.name AS collection_name,
                       j.created_at, j.updated_at
                FROM web_crawl_jobs j
                JOIN collections c ON c.id = j.collection_id
                WHERE j.user_id = $1
                ORDER BY j.created_at DESC
                "#,
            )
            .bind(user_id)
            .fetch_all(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "web_crawl/repository",
                    error = %e,
                    query = "SELECT web_crawl_jobs WHERE user_id",
                    "list_jobs.sql_error"
                );
                AppError::InternalError(format!("Failed to list crawl jobs: {e}"))
            })?
        };

        let summaries = rows
            .into_iter()
            .map(CrawlJobSummary::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        tracing::debug!(
            component = "web_crawl/repository",
            count = summaries.len(),
            "list_jobs.exit"
        );
        Ok(summaries)
    }

    /// Get a single crawl job by ID with ownership check.
    pub async fn get_job_for_user(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<CrawlJob, AppError> {
        tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "get_job.entry");

        let job = if is_admin {
            sqlx::query_as::<_, CrawlJobRow>(
                r#"
                SELECT id, entry_url, config, status, pages_found, pages_indexed,
                       collection_id, user_id, error_message, created_at, updated_at
                FROM web_crawl_jobs
                WHERE id = $1
                "#,
            )
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "web_crawl/repository",
                    error = %e,
                    query = "SELECT web_crawl_jobs WHERE id",
                    "get_job.sql_error"
                );
                AppError::InternalError(format!("Database error: {e}"))
            })?
            .ok_or_else(|| {
                tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "get_job.not_found");
                AppError::NotFound(format!("Crawl job {id} not found"))
            })?
        } else {
            sqlx::query_as::<_, CrawlJobRow>(
                r#"
                SELECT id, entry_url, config, status, pages_found, pages_indexed,
                       collection_id, user_id, error_message, created_at, updated_at
                FROM web_crawl_jobs
                WHERE id = $1 AND user_id = $2
                "#,
            )
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "web_crawl/repository",
                    error = %e,
                    query = "SELECT web_crawl_jobs WHERE id AND user_id",
                    "get_job.sql_error"
                );
                AppError::InternalError(format!("Database error: {e}"))
            })?
            .ok_or_else(|| {
                tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "get_job.not_found");
                AppError::NotFound(format!("Crawl job {id} not found"))
            })?
        };

        let job = CrawlJob::try_from(job)?;
        tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "get_job.exit");
        Ok(job)
    }

    /// Get a crawl job summary with collection name for user-facing responses.
    pub async fn get_job_summary_with_collection_name(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<CrawlJobSummary, AppError> {
        tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "get_job_summary.entry");

        let row = if is_admin {
            sqlx::query_as::<_, CrawlJobSummaryRow>(
                r#"
                SELECT j.id, j.entry_url, j.config, j.status, j.pages_found,
                       j.pages_indexed, j.collection_id, c.name AS collection_name,
                       j.created_at, j.updated_at
                FROM web_crawl_jobs j
                JOIN collections c ON c.id = j.collection_id
                WHERE j.id = $1
                "#,
            )
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "web_crawl/repository",
                    error = %e,
                    query = "SELECT web_crawl_jobs JOIN collections (admin)",
                    "get_job_summary.sql_error"
                );
                AppError::InternalError(format!("Database error: {e}"))
            })?
            .ok_or_else(|| {
                tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "get_job_summary.not_found");
                AppError::NotFound(format!("Crawl job {id} not found"))
            })?
        } else {
            sqlx::query_as::<_, CrawlJobSummaryRow>(
                r#"
                SELECT j.id, j.entry_url, j.config, j.status, j.pages_found,
                       j.pages_indexed, j.collection_id, c.name AS collection_name,
                       j.created_at, j.updated_at
                FROM web_crawl_jobs j
                JOIN collections c ON c.id = j.collection_id
                WHERE j.id = $1 AND j.user_id = $2
                "#,
            )
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    component = "web_crawl/repository",
                    error = %e,
                    query = "SELECT web_crawl_jobs JOIN collections WHERE id AND user_id",
                    "get_job_summary.sql_error"
                );
                AppError::InternalError(format!("Database error: {e}"))
            })?
            .ok_or_else(|| {
                tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "get_job_summary.not_found");
                AppError::NotFound(format!("Crawl job {id} not found"))
            })?
        };

        let summary = CrawlJobSummary::try_from(row)?;
        tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "get_job_summary.exit");
        Ok(summary)
    }

    /// Delete a crawl job by ID with ownership check.
    /// Pages are cascaded by the DB (ON DELETE CASCADE).
    pub async fn delete_job_for_user(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<(), AppError> {
        tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "delete_job.entry");

        let affected = if is_admin {
            sqlx::query("DELETE FROM web_crawl_jobs WHERE id = $1")
                .bind(id)
                .execute(&self.db)
                .await
                .map_err(|e| {
                    tracing::error!(
                        component = "web_crawl/repository",
                        error = %e,
                        query = "DELETE web_crawl_jobs",
                        "delete_job.sql_error"
                    );
                    AppError::InternalError(format!("Failed to delete crawl job: {e}"))
                })?
        } else {
            sqlx::query("DELETE FROM web_crawl_jobs WHERE id = $1 AND user_id = $2")
                .bind(id)
                .bind(user_id)
                .execute(&self.db)
                .await
                .map_err(|e| {
                    tracing::error!(
                        component = "web_crawl/repository",
                        error = %e,
                        query = "DELETE web_crawl_jobs WHERE id AND user_id",
                        "delete_job.sql_error"
                    );
                    AppError::InternalError(format!("Failed to delete crawl job: {e}"))
                })?
        };

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Crawl job {id} not found")));
        }

        tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "delete_job.exit");
        Ok(())
    }

    /// Update job status, counters, and optional error message.
    pub async fn update_job_status(
        &self,
        id: Uuid,
        status: &str,
        pages_found: i32,
        pages_indexed: i32,
        error_message: Option<&str>,
    ) -> Result<(), AppError> {
        tracing::debug!(
            component = "web_crawl/repository",
            crawl_job_id = %id,
            status = %status,
            "update_job_status.entry"
        );

        let now = Utc::now();
        let affected = sqlx::query(
            r#"
            UPDATE web_crawl_jobs
            SET status = $1, pages_found = $2, pages_indexed = $3,
                error_message = $4, updated_at = $5
            WHERE id = $6
            "#,
        )
        .bind(status)
        .bind(pages_found)
        .bind(pages_indexed)
        .bind(error_message)
        .bind(now)
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "web_crawl/repository",
                error = %e,
                query = "UPDATE web_crawl_jobs SET status",
                "update_job_status.sql_error"
            );
            AppError::InternalError(format!("Failed to update crawl job: {e}"))
        })?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Crawl job {id} not found")));
        }

        tracing::debug!(
            component = "web_crawl/repository",
            crawl_job_id = %id,
            status = %status,
            "update_job_status.exit"
        );
        Ok(())
    }

    /// Try to acquire a crawl lock using compare-and-swap.
    /// Only succeeds if the job's current status is 'idle'.
    pub async fn try_acquire_crawl_lock(&self, id: Uuid) -> Result<bool, AppError> {
        tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "try_acquire_crawl_lock.entry");

        let now = Utc::now();
        let result = sqlx::query(
            r#"
            UPDATE web_crawl_jobs
            SET status = 'crawling',
                updated_at = $1
            WHERE id = $2 AND status = 'idle'
            "#,
        )
        .bind(now)
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "web_crawl/repository",
                error = %e,
                query = "UPDATE web_crawl_jobs SET status = crawling WHERE id AND status = idle",
                "try_acquire_crawl_lock.sql_error"
            );
            AppError::InternalError(format!("Failed to acquire crawl lock: {e}"))
        })?;

        let acquired = result.rows_affected() > 0;
        tracing::debug!(
            component = "web_crawl/repository",
            crawl_job_id = %id,
            acquired = acquired,
            "try_acquire_crawl_lock.exit"
        );
        Ok(acquired)
    }

    /// Cancel a job from any active status (idle, crawling) to cancelled.
    pub async fn cancel_job(&self, id: Uuid) -> Result<bool, AppError> {
        tracing::debug!(component = "web_crawl/repository", crawl_job_id = %id, "cancel_job.entry");

        let now = Utc::now();
        let result = sqlx::query(
            r#"
            UPDATE web_crawl_jobs
            SET status = 'cancelled',
                updated_at = $1
            WHERE id = $2 AND status IN ('idle', 'crawling')
            "#,
        )
        .bind(now)
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "web_crawl/repository",
                error = %e,
                query = "UPDATE web_crawl_jobs SET status = cancelled",
                "cancel_job.sql_error"
            );
            AppError::InternalError(format!("Failed to cancel crawl job: {e}"))
        })?;

        let cancelled = result.rows_affected() > 0;
        tracing::debug!(
            component = "web_crawl/repository",
            crawl_job_id = %id,
            cancelled = cancelled,
            "cancel_job.exit"
        );
        Ok(cancelled)
    }

    // ── Page CRUD ──

    /// Insert a new page for a crawl job.
    pub async fn create_page(&self, page: &CrawlPage) -> Result<Uuid, AppError> {
        tracing::debug!(
            component = "web_crawl/repository",
            page_id = %page.id,
            job_id = %page.job_id,
            "create_page.entry"
        );

        sqlx::query(
            r#"
            INSERT INTO web_crawl_pages
                (id, job_id, url, depth, status, http_status, title, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(page.id)
        .bind(page.job_id)
        .bind(&page.url)
        .bind(page.depth)
        .bind(&page.status)
        .bind(page.http_status)
        .bind(&page.title)
        .bind(page.created_at)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "web_crawl/repository",
                error = %e,
                query = "INSERT web_crawl_pages",
                "create_page.sql_error"
            );
            AppError::InternalError(format!("Failed to create crawl page: {e}"))
        })?;

        tracing::debug!(component = "web_crawl/repository", page_id = %page.id, "create_page.exit");
        Ok(page.id)
    }

    /// List pages for a given crawl job.
    pub async fn list_pages_by_job(&self, job_id: Uuid) -> Result<Vec<CrawlPage>, AppError> {
        tracing::debug!(component = "web_crawl/repository", job_id = %job_id, "list_pages.entry");

        let pages = sqlx::query_as::<_, CrawlPage>(
            r#"
            SELECT id, job_id, url, depth, status, http_status, title, created_at
            FROM web_crawl_pages
            WHERE job_id = $1
            ORDER BY depth ASC, created_at ASC
            "#,
        )
        .bind(job_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "web_crawl/repository",
                error = %e,
                query = "SELECT web_crawl_pages WHERE job_id",
                "list_pages.sql_error"
            );
            AppError::InternalError(format!("Failed to list crawl pages: {e}"))
        })?;

        tracing::debug!(
            component = "web_crawl/repository",
            job_id = %job_id,
            count = pages.len(),
            "list_pages.exit"
        );
        Ok(pages)
    }

    /// Batch-update page status and http_status for all pages of a job.
    /// Update the status of pages by job, optionally filtered by the current status.
    /// When `filter_status` is `Some`, only pages with that status are updated.
    /// This prevents resetting successfully indexed pages during a retry.
    pub async fn update_pages_status_by_job(
        &self,
        job_id: Uuid,
        status: &str,
        http_status: Option<i32>,
        filter_status: Option<&str>,
    ) -> Result<u64, AppError> {
        tracing::debug!(
            component = "web_crawl/repository",
            job_id = %job_id,
            status = %status,
            filter_status = ?filter_status,
            "update_pages_status.entry"
        );

        let result = if let Some(filter) = filter_status {
            sqlx::query(
                r#"
                UPDATE web_crawl_pages
                SET status = $1,
                    http_status = COALESCE($2, http_status)
                WHERE job_id = $3 AND status = $4
                ""#,
            )
            .bind(status)
            .bind(http_status)
            .bind(job_id)
            .bind(filter)
            .execute(&self.db)
            .await
        } else {
            sqlx::query(
                r#"
                UPDATE web_crawl_pages
                SET status = $1,
                    http_status = COALESCE($2, http_status)
                WHERE job_id = $3
                ""#,
            )
            .bind(status)
            .bind(http_status)
            .bind(job_id)
            .execute(&self.db)
            .await
        }
        .map_err(|e| {
            tracing::error!(
                component = "web_crawl/repository",
                error = %e,
                query = "UPDATE web_crawl_pages SET status WHERE job_id",
                "update_pages_status.sql_error"
            );
            AppError::InternalError(format!("Failed to update pages status: {e}"))
        })?;

        let updated = result.rows_affected();
        tracing::debug!(
            component = "web_crawl/repository",
            job_id = %job_id,
            updated = updated,
            "update_pages_status.exit"
        );
        Ok(updated)
    }

    /// Update the status of a single page by ID.
    pub async fn update_page_status(
        &self,
        page_id: Uuid,
        status: &str,
        http_status: Option<i32>,
    ) -> Result<u64, AppError> {
        tracing::debug!(
            component = "web_crawl/repository",
            page_id = %page_id,
            status = %status,
            "update_page_status.entry"
        );

        let result = sqlx::query(
            r#"
            UPDATE web_crawl_pages
            SET status = $1,
                http_status = COALESCE($2, http_status)
            WHERE id = $3
            ""#,
        )
        .bind(status)
        .bind(http_status)
        .bind(page_id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!(
                component = "web_crawl/repository",
                error = %e,
                query = "UPDATE web_crawl_pages SET status WHERE id",
                "update_page_status.sql_error"
            );
            AppError::InternalError(format!("Failed to update page status: {e}"))
        })?;

        let updated = result.rows_affected();
        tracing::debug!(
            component = "web_crawl/repository",
            page_id = %page_id,
            updated = updated,
            "update_page_status.exit"
        );
        Ok(updated)
    }
}
