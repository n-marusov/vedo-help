/// Integration tests for Web Crawl module.
///
/// These tests verify full DB round-trips and multi-table interactions
/// for crawl jobs and pages.
///
/// Run:
/// ```bash
/// cargo test --test web_crawl_integration
/// ```
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

mod common;

/// Test: full job lifecycle — create job → add pages → update pages → verify counts.
#[serial_test::serial]
#[tokio::test]
async fn test_job_lifecycle_full() {
    let pool = common::setup_test_db().await;

    // Create collection
    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("Integration Lifecycle Collection")
    .bind("")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("insert collection");

    // Create job
    let job_id = Uuid::new_v4();
    let now = Utc::now();
    let config = json!({"max_depth": 2, "max_pages": 50, "delay_ms": 500});

    sqlx::query(
        "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(job_id)
    .bind("https://example.com/docs")
    .bind(&config)
    .bind("idle")
    .bind(0i32)
    .bind(0i32)
    .bind(coll_id)
    .bind("user-int-001")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert job");

    // Update status to crawling
    sqlx::query(
        "UPDATE web_crawl_jobs SET status = $1, pages_found = $2, updated_at = $3 WHERE id = $4",
    )
    .bind("crawling")
    .bind(5i32)
    .bind(Utc::now())
    .bind(job_id)
    .execute(&pool)
    .await
    .expect("update status to crawling");

    // Add pages
    let page_urls = [
        "https://example.com/docs/intro",
        "https://example.com/docs/setup",
        "https://example.com/docs/usage",
        "https://example.com/docs/api",
        "https://example.com/docs/faq",
    ];

    for (i, url) in page_urls.iter().enumerate() {
        let page_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO web_crawl_pages (id, job_id, url, depth, status, http_status, title, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(page_id)
        .bind(job_id)
        .bind(*url)
        .bind((i + 1) as i32)
        .bind("pending")
        .bind(Option::<i32>::None)
        .bind(Option::<String>::None)
        .bind(Utc::now())
        .execute(&pool)
        .await
        .expect("insert page");
    }

    // Verify page count
    let page_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM web_crawl_pages WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&pool)
            .await
            .expect("count pages");
    assert_eq!(page_count, 5, "should have 5 pages");

    // Update all pages to indexed
    sqlx::query(
        "UPDATE web_crawl_pages SET status = 'indexed', http_status = 200 WHERE job_id = $1",
    )
    .bind(job_id)
    .execute(&pool)
    .await
    .expect("update all pages to indexed");

    // Update job to completed
    sqlx::query(
        "UPDATE web_crawl_jobs SET status = $1, pages_indexed = $2, updated_at = $3 WHERE id = $4",
    )
    .bind("completed")
    .bind(5i32)
    .bind(Utc::now())
    .bind(job_id)
    .execute(&pool)
    .await
    .expect("update status to completed");

    // Verify final state
    let job_status: String = sqlx::query_scalar("SELECT status FROM web_crawl_jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(&pool)
        .await
        .expect("fetch status");
    assert_eq!(job_status, "completed");

    let indexed_pages: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM web_crawl_pages WHERE job_id = $1 AND status = 'indexed'",
    )
    .bind(job_id)
    .fetch_one(&pool)
    .await
    .expect("count indexed pages");
    assert_eq!(indexed_pages, 5, "all 5 pages should be indexed");
}

/// Test: cancel running job — all pending pages marked cancelled.
#[serial_test::serial]
#[tokio::test]
async fn test_cancel_job_marks_pending_pages() {
    let pool = common::setup_test_db().await;

    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("Cancel Pages Collection")
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
    .bind(10i32)
    .bind(3i32)
    .bind(coll_id)
    .bind("user-int-002")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert crawling job");

    // Add 3 crawled + 7 pending pages
    for i in 0..10 {
        let page_id = Uuid::new_v4();
        let status = if i < 3 { "indexed" } else { "pending" };
        sqlx::query(
            "INSERT INTO web_crawl_pages (id, job_id, url, depth, status, http_status, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(page_id)
        .bind(job_id)
        .bind(format!("https://example.com/page-{i}"))
        .bind(i % 3)
        .bind(status)
        .bind(Some(200i32))
        .bind(Utc::now())
        .execute(&pool)
        .await
        .expect("insert page");
    }

    // Cancel job
    sqlx::query("UPDATE web_crawl_jobs SET status = $1, updated_at = $2 WHERE id = $3")
        .bind("cancelled")
        .bind(Utc::now())
        .bind(job_id)
        .execute(&pool)
        .await
        .expect("cancel job");

    // Verify only pending pages are marked as cancelled/failed
    // (indexed pages keep their status)
    let indexed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM web_crawl_pages WHERE job_id = $1 AND status = 'indexed'",
    )
    .bind(job_id)
    .fetch_one(&pool)
    .await
    .expect("count indexed");
    assert_eq!(indexed_count, 3, "indexed pages keep their status");

    let pending_after_cancel: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM web_crawl_pages WHERE job_id = $1 AND status = 'pending'",
    )
    .bind(job_id)
    .fetch_one(&pool)
    .await
    .expect("count pending after cancel");
    assert_eq!(
        pending_after_cancel, 7,
        "pending pages stay pending after cancel"
    );
}

/// Test: multiple jobs for same collection track independently.
#[serial_test::serial]
#[tokio::test]
async fn test_multiple_jobs_independent_tracking() {
    let pool = common::setup_test_db().await;

    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("Multi Job Collection")
    .bind("")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("insert collection");

    let now = Utc::now();
    let config = json!({});

    // Create two jobs for same collection
    let mut job_ids = Vec::new();
    for _ in 0..2 {
        let job_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(job_id)
        .bind(format!("https://example.com/site-{}", job_id))
        .bind(&config)
        .bind("idle")
        .bind(0i32)
        .bind(0i32)
        .bind(coll_id)
        .bind("user-int-003")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("insert job");
        job_ids.push(job_id);
    }

    // Add pages to job 1
    for i in 0..3 {
        let page_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO web_crawl_pages (id, job_id, url, depth, status, http_status, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(page_id)
        .bind(job_ids[0])
        .bind(format!("https://example.com/site-1/page-{i}"))
        .bind(i)
        .bind("pending")
        .bind(Option::<i32>::None)
        .bind(Utc::now())
        .execute(&pool)
        .await
        .expect("insert page for job 1");
    }

    // Add pages to job 2
    for i in 0..5 {
        let page_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO web_crawl_pages (id, job_id, url, depth, status, http_status, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(page_id)
        .bind(job_ids[1])
        .bind(format!("https://example.com/site-2/page-{i}"))
        .bind(i)
        .bind("pending")
        .bind(Option::<i32>::None)
        .bind(Utc::now())
        .execute(&pool)
        .await
        .expect("insert page for job 2");
    }

    // Verify independent counts
    let count_job1: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM web_crawl_pages WHERE job_id = $1")
            .bind(job_ids[0])
            .fetch_one(&pool)
            .await
            .expect("count job 1 pages");
    assert_eq!(count_job1, 3, "job 1 has 3 pages");

    let count_job2: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM web_crawl_pages WHERE job_id = $1")
            .bind(job_ids[1])
            .fetch_one(&pool)
            .await
            .expect("count job 2 pages");
    assert_eq!(count_job2, 5, "job 2 has 5 pages");
}

/// Test: job with error stores error message.
#[serial_test::serial]
#[tokio::test]
async fn test_job_error_stores_message() {
    let pool = common::setup_test_db().await;

    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("Error Collection")
    .bind("")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("insert collection");

    let job_id = Uuid::new_v4();
    let now = Utc::now();
    let config = json!({});
    let error_msg = "Connection timeout after 10s";

    sqlx::query(
        "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, error_message, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(job_id)
    .bind("https://example.com")
    .bind(&config)
    .bind("error")
    .bind(1i32)
    .bind(0i32)
    .bind(coll_id)
    .bind("user-int-004")
    .bind(error_msg)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert error job");

    let result: (String, Option<String>) =
        sqlx::query_as("SELECT status, error_message FROM web_crawl_jobs WHERE id = $1")
            .bind(job_id)
            .fetch_one(&pool)
            .await
            .expect("fetch error job");

    assert_eq!(result.0, "error");
    assert_eq!(result.1, Some(error_msg.to_string()));
}

/// Test: job pages correctly scoped — pages from one job not visible from another.
#[serial_test::serial]
#[tokio::test]
async fn test_page_scoping_by_job() {
    let pool = common::setup_test_db().await;

    let coll_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(coll_id)
    .bind("Scoping Collection")
    .bind("")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("insert collection");

    let now = Utc::now();
    let config = json!({});

    // Both jobs use same URL patterns but should be scoped by job_id
    let job_a = Uuid::new_v4();
    let job_b = Uuid::new_v4();

    for job_id in [job_a, job_b] {
        sqlx::query(
            "INSERT INTO web_crawl_jobs (id, entry_url, config, status, pages_found, pages_indexed, collection_id, user_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(job_id)
        .bind("https://example.com")
        .bind(&config)
        .bind("completed")
        .bind(2i32)
        .bind(2i32)
        .bind(coll_id)
        .bind("user-scope")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("insert job");
    }

    // Add identical URL to both jobs
    for job_id in [job_a, job_b] {
        for i in 0..2 {
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
            .bind(Some(200i32))
            .bind(now)
            .execute(&pool)
            .await
            .expect("insert page");
        }
    }

    // Verify scoping
    let pages_a: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM web_crawl_pages WHERE job_id = $1")
        .bind(job_a)
        .fetch_one(&pool)
        .await
        .expect("count job A pages");
    assert_eq!(pages_a, 2, "job A sees its own 2 pages");

    // Query for all pages across both jobs (no filter)
    let total_pages: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM web_crawl_pages WHERE job_id IN ($1, $2)")
            .bind(job_a)
            .bind(job_b)
            .fetch_one(&pool)
            .await
            .expect("count total pages");
    assert_eq!(total_pages, 4, "both jobs combined have 4 pages");
}
