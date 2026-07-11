use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::time::Duration;
use uuid::Uuid;

use crate::modules::auth::models::UserContext;
use crate::modules::web_crawl::models::{
    CrawlJobDetailResponse, CrawlJobSummary, CreateCrawlJobRequest,
};
use crate::modules::web_crawl::service::WebCrawlService;
use crate::shared::error::AppError;

/// Create a new web crawl job.
///
/// Endpoint: `POST /api/web-crawl`
///
/// Validates that the URL starts with `http://` or `https://` and that
/// `max_depth` does not exceed 10.
pub async fn create_job(
    user_ctx: UserContext,
    State(svc): State<WebCrawlService>,
    Json(req): Json<CreateCrawlJobRequest>,
) -> Result<Json<CrawlJobSummary>, AppError> {
    // Validate URL — only HTTP/HTTPS URLs are supported
    if !req.entry_url.starts_with("http://") && !req.entry_url.starts_with("https://") {
        return Err(AppError::BadRequest(
            "URL must start with http:// or https://".to_string(),
        ));
    }

    // Validate max_depth ≤ 10
    if let Some(ref config) = req.config {
        if config.max_depth > 10 {
            return Err(AppError::BadRequest("max_depth must be ≤ 10".to_string()));
        }
    }

    tracing::info!(
        component = "web_crawl/handlers",
        entry_url = %req.entry_url,
        collection_id = %req.collection_id,
        "create_job.start"
    );

    let summary = svc.create_job(req, &user_ctx.user_id).await.map_err(|e| {
        tracing::error!(component = "web_crawl/handlers", error = %e, "create_job.failed");
        e
    })?;

    // Automatically start crawling after creation
    if let Err(e) = svc
        .start_crawl(summary.id, &user_ctx.user_id, user_ctx.is_admin())
        .await
    {
        tracing::error!(
            component = "web_crawl/handlers",
            crawl_job_id = %summary.id,
            error = %e,
            "create_job.start_crawl_failed"
        );
    }

    tracing::info!(
        component = "web_crawl/handlers",
        crawl_job_id = %summary.id,
        status = %summary.status,
        "create_job.completed"
    );

    Ok(Json(summary))
}

/// List all web crawl jobs for the current user (or all jobs if admin).
///
/// Endpoint: `GET /api/web-crawl`
pub async fn list_jobs(
    user_ctx: UserContext,
    State(svc): State<WebCrawlService>,
) -> Result<Json<Vec<CrawlJobSummary>>, AppError> {
    tracing::info!(
        component = "web_crawl/handlers",
        user_id = %user_ctx.user_id,
        "list_jobs"
    );

    let jobs = svc
        .list_jobs(&user_ctx.user_id, user_ctx.is_admin())
        .await
        .map_err(|e| {
            tracing::error!(component = "web_crawl/handlers", error = %e, "list_jobs.failed");
            e
        })?;

    tracing::debug!(
        component = "web_crawl/handlers",
        count = jobs.len(),
        "list_jobs.result"
    );

    Ok(Json(jobs))
}

/// Get a single crawl job with its pages.
///
/// Endpoint: `GET /api/web-crawl/{id}`
pub async fn get_job(
    user_ctx: UserContext,
    State(svc): State<WebCrawlService>,
    Path(id): Path<Uuid>,
) -> Result<Json<CrawlJobDetailResponse>, AppError> {
    tracing::info!(
        component = "web_crawl/handlers",
        crawl_job_id = %id,
        user_id = %user_ctx.user_id,
        "get_job"
    );

    let detail = svc
        .get_job_with_pages(id, &user_ctx.user_id, user_ctx.is_admin())
        .await
        .map_err(|e| {
            tracing::error!(component = "web_crawl/handlers", crawl_job_id = %id, error = %e, "get_job.failed");
            e
        })?;

    Ok(Json(detail))
}

/// Delete a crawl job.
///
/// Endpoint: `DELETE /api/web-crawl/{id}`
pub async fn delete_job(
    user_ctx: UserContext,
    State(svc): State<WebCrawlService>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(
        component = "web_crawl/handlers",
        crawl_job_id = %id,
        user_id = %user_ctx.user_id,
        "delete_job"
    );

    svc.delete_job(id, &user_ctx.user_id, user_ctx.is_admin())
        .await
        .map_err(|e| {
            tracing::error!(component = "web_crawl/handlers", crawl_job_id = %id, error = %e, "delete_job.failed");
            e
        })?;

    tracing::info!(
        component = "web_crawl/handlers",
        crawl_job_id = %id,
        "delete_job.completed"
    );

    Ok(Json(serde_json::json!({"status": "deleted", "id": id})))
}

/// Cancel a crawl job.
///
/// Endpoint: `POST /api/web-crawl/{id}/cancel`
///
/// Uses CAS to transition from idle/crawling to cancelled.
pub async fn cancel_job(
    user_ctx: UserContext,
    State(svc): State<WebCrawlService>,
    Path(id): Path<Uuid>,
) -> Result<Json<CrawlJobSummary>, AppError> {
    tracing::info!(
        component = "web_crawl/handlers",
        crawl_job_id = %id,
        user_id = %user_ctx.user_id,
        "cancel_job"
    );

    let summary = svc
        .cancel_job(id, &user_ctx.user_id, user_ctx.is_admin())
        .await
        .map_err(|e| {
            tracing::error!(component = "web_crawl/handlers", crawl_job_id = %id, error = %e, "cancel_job.failed");
            e
        })?;

    tracing::info!(
        component = "web_crawl/handlers",
        crawl_job_id = %id,
        status = %summary.status,
        "cancel_job.completed"
    );

    Ok(Json(summary))
}

/// Retry failed pages in a crawl job.
///
/// Endpoint: `POST /api/web-crawl/{id}/retry`
pub async fn retry_failed(
    user_ctx: UserContext,
    State(svc): State<WebCrawlService>,
    Path(id): Path<Uuid>,
) -> Result<Json<CrawlJobSummary>, AppError> {
    tracing::info!(
        component = "web_crawl/handlers",
        crawl_job_id = %id,
        user_id = %user_ctx.user_id,
        "retry_failed"
    );

    let summary = svc
        .retry_failed_pages(id, &user_ctx.user_id, user_ctx.is_admin())
        .await
        .map_err(|e| {
            tracing::error!(component = "web_crawl/handlers", crawl_job_id = %id, error = %e, "retry_failed.failed");
            e
        })?;

    tracing::info!(
        component = "web_crawl/handlers",
        crawl_job_id = %id,
        status = %summary.status,
        "retry_failed.completed"
    );

    Ok(Json(summary))
}

/// SSE subscription for real-time crawl progress.
///
/// Endpoint: `GET /api/web-crawl/{id}/subscribe`
pub async fn subscribe(
    user_ctx: UserContext,
    State(svc): State<WebCrawlService>,
    Path(id): Path<Uuid>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::info!(
        component = "web_crawl/handlers",
        crawl_job_id = %id,
        user_id = %user_ctx.user_id,
        "subscribe"
    );

    // Create a stream that polls the crawl status every 2 seconds
    let svc = svc.clone();
    let user_id = user_ctx.user_id.clone();
    let is_admin = user_ctx.is_admin();

    let stream = stream::unfold(
        (svc, id, user_id, is_admin, true),
        move |(svc, job_id, uid, admin, mut running)| async move {
            if !running {
                return None;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
            match svc.get_crawl_status(job_id, &uid, admin).await {
                Ok(status) => {
                    let is_terminal =
                        matches!(status.status.as_str(), "completed" | "cancelled" | "error");
                    if is_terminal {
                        running = false;
                    }
                    let event = match serde_json::to_string(&status) {
                        Ok(json) => Ok(Event::default().data(json)),
                        Err(_) => Ok(Event::default().data("{}")),
                    };
                    Some((event, (svc, job_id, uid, admin, running)))
                }
                Err(_) => {
                    running = false;
                    Some((
                        Ok(Event::default().data("{}")),
                        (svc, job_id, uid, admin, running),
                    ))
                }
            }
        },
    );

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}
