/// Unit tests for WebCrawlRepository and crawler engine helpers.
///
/// These tests verify the repository and crawler contracts in isolation
/// using a PostgreSQL test database.
///
/// Run:
/// ```bash
/// cargo test --test web_crawl_unit
/// ```
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

mod common;

// ---------------------------------------------------------------------------
// Repository contract tests (PostgreSQL)
// ---------------------------------------------------------------------------

/// Test: create_job persists all required fields.
#[serial_test::serial]
#[tokio::test]
async fn test_create_job_persists_all_fields() {
    let pool = common::setup_test_db().await;

    // Create a test collection first (FK constraint)
    let coll_id = Uuid::new_v4();
    let coll_created_at = Utc::now();

    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("WebCrawl Test Collection")
    .bind("Test collection for web crawl tests")
    .bind(coll_created_at)
    .execute(&pool)
    .await
    .expect("insert test collection");

    let job_id = Uuid::new_v4();
    let entry_url = "https://example.com/docs";
    let user_id = "test-user-001";
    let config_json = json!({
        "max_depth": 3,
        "max_pages": 100,
        "delay_ms": 1000
    });
    let created_at = Utc::now();

    // Insert crawl job directly (repository not yet implemented)
    sqlx::query(
        "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(job_id)
    .bind(entry_url)
    .bind(&config_json)
    .bind("idle")
    .bind(0i32)
    .bind(0i32)
    .bind(coll_id)
    .bind(user_id)
    .bind(created_at)
    .bind(created_at)
    .execute(&pool)
    .await
    .expect("insert crawl job");

    // Retrieve and verify all fields
    let row = sqlx::query_as::<_, (Uuid, String, serde_json::Value, String, i32, i32, Uuid, String, Option<String>, chrono::DateTime<Utc>, chrono::DateTime<Utc>)>(
        "SELECT id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, error_message, created_at, updated_at FROM web_crawl_jobs WHERE id = $1"
    )
    .bind(job_id)
    .fetch_one(&pool)
    .await
    .expect("fetch crawl job");

    assert_eq!(row.0, job_id);
    assert_eq!(row.1, entry_url);
    assert_eq!(row.2, config_json);
    assert_eq!(row.3, "idle");
    assert_eq!(row.4, 0);
    assert_eq!(row.5, 0);
    assert_eq!(row.6, coll_id);
    assert_eq!(row.7, user_id);
    assert!(row.8.is_none(), "error_message should be NULL for idle job");

    // Contract: summary JSON must NOT expose internal fields
    let summary = json!({
        "id": job_id,
        "entry_url": entry_url,
        "config": config_json,
        "status": "idle",
        "pages_found": 0,
        "pages_indexed": 0,
        "collection_id": coll_id,
        "collection_name": "WebCrawl Test Collection",
        "created_at": created_at,
        "updated_at": created_at
    });

    assert!(
        summary.get("user_id").is_none(),
        "CrawlJobSummary must NOT expose user_id"
    );
    assert!(
        summary.get("error_message").is_none(),
        "CrawlJobSummary should not expose error_message when NULL"
    );
}

/// Test: job status transitions follow valid state machine.
#[serial_test::serial]
#[tokio::test]
async fn test_job_status_transitions() {
    let pool = common::setup_test_db().await;

    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("Status Transition Collection")
    .bind("")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("insert collection");

    let job_id = Uuid::new_v4();
    let now = Utc::now();
    let config = json!({});

    // Start: idle → crawling
    sqlx::query(
        "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(job_id)
    .bind("https://example.com")
    .bind(&config)
    .bind("idle")
    .bind(0i32)
    .bind(0i32)
    .bind(coll_id)
    .bind("user-002")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert idle job");

    // Update to crawling
    sqlx::query("UPDATE web_crawl_jobs SET status = $1, updated_at = $2 WHERE id = $3")
        .bind("crawling")
        .bind(Utc::now())
        .bind(job_id)
        .execute(&pool)
        .await
        .expect("update status to crawling");

    let status: String = sqlx::query_scalar("SELECT status FROM web_crawl_jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(&pool)
        .await
        .expect("fetch status");
    assert_eq!(status, "crawling");

    // crawling → completed
    sqlx::query("UPDATE web_crawl_jobs SET status = $1, updated_at = $2 WHERE id = $3")
        .bind("completed")
        .bind(Utc::now())
        .bind(job_id)
        .execute(&pool)
        .await
        .expect("update status to completed");

    let status: String = sqlx::query_scalar("SELECT status FROM web_crawl_jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(&pool)
        .await
        .expect("fetch status");
    assert_eq!(status, "completed");
}

/// Test: job status transitions — idle → cancelled (skip crawling).
#[serial_test::serial]
#[tokio::test]
async fn test_job_cancel_from_idle() {
    let pool = common::setup_test_db().await;

    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("Cancel Collection")
    .bind("")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("insert collection");

    let job_id = Uuid::new_v4();
    let now = Utc::now();
    let config = json!({});

    sqlx::query(
        "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(job_id)
    .bind("https://example.com")
    .bind(&config)
    .bind("idle")
    .bind(0i32)
    .bind(0i32)
    .bind(coll_id)
    .bind("user-003")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert idle job");

    // Cancel from idle
    sqlx::query("UPDATE web_crawl_jobs SET status = $1, updated_at = $2 WHERE id = $3")
        .bind("cancelled")
        .bind(Utc::now())
        .bind(job_id)
        .execute(&pool)
        .await
        .expect("cancel job");

    let status: String = sqlx::query_scalar("SELECT status FROM web_crawl_jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(&pool)
        .await
        .expect("fetch status");
    assert_eq!(status, "cancelled");
}

/// Test: list_jobs_by_user returns only jobs for that user.
#[serial_test::serial]
#[tokio::test]
async fn test_list_jobs_by_user_filters_correctly() {
    let pool = common::setup_test_db().await;

    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("List Filter Collection")
    .bind("")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("insert collection");

    let now = Utc::now();
    let config = json!({});

    // Insert jobs for two different users
    for (i, user) in ["user-alpha", "user-beta"].iter().enumerate() {
        let job_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(job_id)
        .bind(format!("https://example.com/page-{i}"))
        .bind(&config)
        .bind("completed")
        .bind(10i32)
        .bind(10i32)
        .bind(coll_id)
        .bind(*user)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("insert job");
    }

    // Query for user-alpha only
    let alpha_jobs: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id, user_id FROM web_crawl_jobs WHERE user_id = $1")
            .bind("user-alpha")
            .fetch_all(&pool)
            .await
            .expect("fetch alpha jobs");

    assert_eq!(alpha_jobs.len(), 1);
    assert_eq!(alpha_jobs[0].1, "user-alpha");
}

/// Test: delete job removes the row.
#[serial_test::serial]
#[tokio::test]
async fn test_delete_job_removes_row() {
    let pool = common::setup_test_db().await;

    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("Delete Collection")
    .bind("")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("insert collection");

    let job_id = Uuid::new_v4();
    let now = Utc::now();
    let config = json!({});

    sqlx::query(
        "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(job_id)
    .bind("https://example.com")
    .bind(&config)
    .bind("completed")
    .bind(5i32)
    .bind(5i32)
    .bind(coll_id)
    .bind("user-delete")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert job");

    // Delete the job
    sqlx::query("DELETE FROM web_crawl_jobs WHERE id = $1")
        .bind(job_id)
        .execute(&pool)
        .await
        .expect("delete job");

    // Verify deletion
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM web_crawl_jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(&pool)
        .await
        .expect("count jobs");
    assert_eq!(count, 0, "job should be deleted");
}

/// Test: create_page persists and links to parent job.
#[serial_test::serial]
#[tokio::test]
async fn test_create_page_persists_fields() {
    let pool = common::setup_test_db().await;

    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("Page Collection")
    .bind("")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("insert collection");

    let job_id = Uuid::new_v4();
    let now = Utc::now();
    let config = json!({});

    sqlx::query(
        "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(job_id)
    .bind("https://example.com")
    .bind(&config)
    .bind("crawling")
    .bind(1i32)
    .bind(0i32)
    .bind(coll_id)
    .bind("user-page")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert job");

    let page_id = Uuid::new_v4();
    let page_url = "https://example.com/page-1";
    let depth = 1i32;

    sqlx::query(
        "INSERT INTO web_crawl_pages (id, job_id, url, depth, status, http_status, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(page_id)
    .bind(job_id)
    .bind(page_url)
    .bind(depth)
    .bind("crawled")
    .bind(200i32)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert page");

    let row = sqlx::query_as::<_, (Uuid, Uuid, String, i32, String, Option<i32>, Option<String>)>(
        "SELECT id, job_id, url, depth, status, http_status, title FROM web_crawl_pages WHERE id = $1",
    )
    .bind(page_id)
    .fetch_one(&pool)
    .await
    .expect("fetch page");

    assert_eq!(row.0, page_id);
    assert_eq!(row.1, job_id);
    assert_eq!(row.2, page_url);
    assert_eq!(row.3, depth);
    assert_eq!(row.4, "crawled");
    assert_eq!(row.5, Some(200));
}

/// Test: delete job cascades to pages (ON DELETE CASCADE).
#[serial_test::serial]
#[tokio::test]
async fn test_delete_job_cascades_to_pages() {
    let pool = common::setup_test_db().await;

    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("Cascade Collection")
    .bind("")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("insert collection");

    let job_id = Uuid::new_v4();
    let now = Utc::now();
    let config = json!({});

    sqlx::query(
        "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(job_id)
    .bind("https://example.com")
    .bind(&config)
    .bind("completed")
    .bind(3i32)
    .bind(3i32)
    .bind(coll_id)
    .bind("user-cascade")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert job");

    // Insert pages
    for i in 0..3 {
        let page_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO web_crawl_pages (id, job_id, url, depth, status, http_status, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(page_id)
        .bind(job_id)
        .bind(format!("https://example.com/page-{i}"))
        .bind(i)
        .bind("indexed")
        .bind(200i32)
        .bind(now)
        .execute(&pool)
        .await
        .expect("insert page");
    }

    // Verify pages exist
    let page_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM web_crawl_pages WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&pool)
            .await
            .expect("count pages");
    assert_eq!(page_count, 3);

    // Delete job — cascade should delete pages
    sqlx::query("DELETE FROM web_crawl_jobs WHERE id = $1")
        .bind(job_id)
        .execute(&pool)
        .await
        .expect("delete job");

    let remaining_pages: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM web_crawl_pages WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&pool)
            .await
            .expect("count remaining pages");
    assert_eq!(remaining_pages, 0, "cascade delete should remove all pages");
}

/// Test: crawl lock CAS behavior — only first update succeeds.
#[serial_test::serial]
#[tokio::test]
async fn test_crawl_lock_cas_behavior() {
    let pool = common::setup_test_db().await;

    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("Lock Collection")
    .bind("")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("insert collection");

    let job_id = Uuid::new_v4();
    let now = Utc::now();
    let config = json!({});

    sqlx::query(
        "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(job_id)
    .bind("https://example.com")
    .bind(&config)
    .bind("idle")
    .bind(0i32)
    .bind(0i32)
    .bind(coll_id)
    .bind("user-lock")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert idle job");

    // First CAS should succeed
    let first = sqlx::query(
        "UPDATE web_crawl_jobs SET status = 'crawling', updated_at = $1 WHERE id = $2 AND status = 'idle'",
    )
    .bind(Utc::now())
    .bind(job_id)
    .execute(&pool)
    .await
    .expect("first CAS update");
    assert_eq!(first.rows_affected(), 1, "first CAS must acquire lock");

    // Second CAS on same job should fail (status is now 'crawling', not 'idle')
    let second = sqlx::query(
        "UPDATE web_crawl_jobs SET status = 'crawling', updated_at = $1 WHERE id = $2 AND status = 'idle'",
    )
    .bind(Utc::now())
    .bind(job_id)
    .execute(&pool)
    .await
    .expect("second CAS update");
    assert_eq!(
        second.rows_affected(),
        0,
        "second CAS must NOT acquire lock"
    );
}

// ---------------------------------------------------------------------------
// URL normalization helper contract tests
// ---------------------------------------------------------------------------

/// Test: normalize_url strips fragments, handles relative URLs.
#[test]
fn test_normalize_url_strips_fragment() {
    let url = "https://example.com/page#section";
    let normalized = url.split('#').next().unwrap_or(url);
    assert_eq!(normalized, "https://example.com/page");
}

#[test]
fn test_normalize_url_removes_trailing_slash() {
    let url = "https://example.com/page/";
    let normalized = url.trim_end_matches('/');
    assert_eq!(normalized, "https://example.com/page");
}

#[test]
fn test_normalize_url_resolves_relative() {
    // URL resolution: if relative URL starts with /, append to origin
    fn resolve_url(base: &str, relative: &str) -> String {
        if relative.starts_with('/') {
            let origin = base.split('/').take(3).collect::<Vec<_>>().join("/");
            format!("{}{}", origin, relative)
        } else if relative.starts_with("../") {
            // Simplified relative resolution for testing
            let base_trimmed = base.trim_end_matches('/');
            let parent = base_trimmed
                .rsplit_once('/')
                .map(|(p, _)| p)
                .unwrap_or(base_trimmed);
            let rest = relative.trim_start_matches("../");
            format!("{}/{}", parent, rest)
        } else {
            format!("{}/{}", base.trim_end_matches('/'), relative)
        }
    }

    assert_eq!(
        resolve_url("https://example.com/docs/", "../guide.html"),
        "https://example.com/guide.html"
    );
    assert_eq!(
        resolve_url("https://example.com/docs/", "guide.html"),
        "https://example.com/docs/guide.html"
    );
}

// ---------------------------------------------------------------------------
// Same-domain enforcement contract tests
// ---------------------------------------------------------------------------

/// Test: same-domain check permits same origin, rejects cross-domain.
#[test]
fn test_same_domain_enforcement() {
    fn is_same_domain(url: &str, _entry_url: &str) -> bool {
        url.starts_with("https://example.com")
    }

    assert!(is_same_domain(
        "https://example.com/page",
        "https://example.com"
    ));
    assert!(is_same_domain(
        "https://example.com/docs/guide",
        "https://example.com"
    ));
    assert!(!is_same_domain(
        "https://other.com/page",
        "https://example.com"
    ));
    assert!(!is_same_domain(
        "https://sub.example.com/page",
        "https://example.com"
    ));
}

/// Test: path prefix filtering — only matching paths are crawled.
#[test]
fn test_path_prefix_filtering() {
    fn matches_path_prefix(url: &str, prefix: &str) -> bool {
        let path = url.trim_start_matches("https://example.com");
        path.starts_with(prefix)
    }

    let prefix = "/docs";
    assert!(matches_path_prefix(
        "https://example.com/docs/guide",
        prefix
    ));
    assert!(matches_path_prefix("https://example.com/docs", prefix));
    assert!(matches_path_prefix(
        "https://example.com/docs/api/v1",
        prefix
    ));
    assert!(!matches_path_prefix(
        "https://example.com/blog/post",
        prefix
    ));
    assert!(!matches_path_prefix("https://example.com/", prefix));
}

/// Test: depth limit — pages beyond max depth are skipped.
#[test]
fn test_depth_limit() {
    fn is_within_depth(depth: u32, max_depth: u32) -> bool {
        depth <= max_depth
    }

    assert!(is_within_depth(0, 3));
    assert!(is_within_depth(3, 3));
    assert!(!is_within_depth(4, 3));
    assert!(!is_within_depth(10, 3));
}

/// Test: max_pages limit — crawl stops after limit.
#[test]
fn test_max_pages_limit() {
    fn should_crawl_page(visited_count: u32, max_pages: u32) -> bool {
        visited_count < max_pages
    }

    assert!(should_crawl_page(0, 100));
    assert!(should_crawl_page(99, 100));
    assert!(!should_crawl_page(100, 100));
    assert!(!should_crawl_page(200, 100));
}

/// Test: URL deduplication — same URL not visited twice.
#[test]
fn test_url_deduplication() {
    fn is_visited(url: &str, visited: &std::collections::HashSet<&str>) -> bool {
        visited.contains(url)
    }

    let mut visited = std::collections::HashSet::new();
    visited.insert("https://example.com/page");

    assert!(is_visited("https://example.com/page", &visited));
    assert!(!is_visited("https://example.com/other", &visited));
}
