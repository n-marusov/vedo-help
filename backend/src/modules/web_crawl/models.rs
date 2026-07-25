use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A web crawl job tracking the crawling state for a single entry URL.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CrawlJob {
    pub id: Uuid,
    pub entry_url: String,
    pub config: serde_json::Value,
    /// One of `idle`, `crawling`, `completed`, `cancelled`, `error`.
    pub status: String,
    pub pages_found: i32,
    pub pages_indexed: i32,
    pub collection_id: Uuid,
    /// The KeyCloak user `sub` that owns this job.
    pub user_id: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public summary of a crawl job, safe for API responses.
///
/// Excludes sensitive/internal fields (`user_id`, `error_message`).
/// Includes the resolved `collection_name` via a JOIN query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlJobSummary {
    pub id: Uuid,
    pub entry_url: String,
    pub config: serde_json::Value,
    pub status: String,
    pub pages_found: i32,
    pub pages_indexed: i32,
    pub collection_id: Uuid,
    pub collection_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CrawlJob> for CrawlJobSummary {
    fn from(job: CrawlJob) -> Self {
        tracing::debug!(
            component = "web_crawl/models",
            crawl_job_id = %job.id,
            "CrawlJobSummary.strip_internal_fields"
        );

        Self {
            id: job.id,
            entry_url: job.entry_url,
            config: job.config,
            status: job.status,
            pages_found: job.pages_found,
            pages_indexed: job.pages_indexed,
            collection_id: job.collection_id,
            // collection_name is resolved via JOIN — unavailable from CrawlJob alone.
            // The caller must populate it separately.
            collection_name: String::new(),
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}

/// A page discovered during a crawl job.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CrawlPage {
    pub id: Uuid,
    pub job_id: Uuid,
    pub url: String,
    pub depth: i32,
    /// One of `pending`, `crawled`, `indexed`, `cancelled`.
    pub status: String,
    pub http_status: Option<i32>,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request payload for creating a new crawl job.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCrawlJobRequest {
    pub entry_url: String,
    pub collection_id: Uuid,
    pub config: Option<CrawlConfig>,
}

/// Configuration for a crawl job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    #[serde(default = "default_max_pages")]
    pub max_pages: u32,
    #[serde(default = "default_delay_ms")]
    pub delay_ms: u64,
    #[serde(default)]
    pub path_prefix: String,
}

fn default_max_depth() -> u32 {
    2
}

fn default_max_pages() -> u32 {
    50
}

fn default_delay_ms() -> u64 {
    1000
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
            max_pages: default_max_pages(),
            delay_ms: default_delay_ms(),
            path_prefix: String::new(),
        }
    }
}

/// Response for a job detail request, including the pages list.
#[derive(Debug, Clone, Serialize)]
pub struct CrawlJobDetailResponse {
    pub id: Uuid,
    pub entry_url: String,
    pub config: serde_json::Value,
    pub status: String,
    pub pages_found: i32,
    pub pages_indexed: i32,
    pub collection_id: Uuid,
    pub collection_name: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pages: Vec<CrawlPage>,
}

/// In-memory progress for an active crawl job.
#[derive(Debug, Clone, Serialize)]
pub struct CrawlProgress {
    pub pages_found: i32,
    pub pages_indexed: i32,
    pub current_url: String,
    pub phase: String,
}

/// A page crawled by the BFS engine, before indexing.
#[derive(Debug, Clone)]
pub struct CrawledPage {
    pub url: String,
    pub title: Option<String>,
    pub text: String,
    pub depth: u32,
    pub http_status: Option<i32>,
}

/// Response for a crawl status endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct CrawlStatusResponse {
    pub job_id: Uuid,
    pub status: String,
    pub pages_found: i32,
    pub pages_indexed: i32,
    pub error: Option<String>,
    pub progress: Option<CrawlProgress>,
}
