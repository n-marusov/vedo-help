-- Create web_crawl_jobs and web_crawl_pages tables for the web crawler module.
-- Supports tracking crawl jobs with pages per job, cascading deletes,
-- and status state machine.

CREATE TABLE IF NOT EXISTS web_crawl_jobs (
    id          UUID PRIMARY KEY,
    entry_url   TEXT NOT NULL,
    config      JSONB NOT NULL DEFAULT '{}',
    status      VARCHAR(20) NOT NULL DEFAULT 'idle',
    pages_found INTEGER NOT NULL DEFAULT 0,
    pages_indexed INTEGER NOT NULL DEFAULT 0,
    collection_id UUID NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL,
    error_message TEXT,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL,
    CONSTRAINT chk_web_crawl_job_status CHECK (status IN ('idle', 'crawling', 'completed', 'cancelled', 'error'))
);

CREATE TABLE IF NOT EXISTS web_crawl_pages (
    id          UUID PRIMARY KEY,
    job_id      UUID NOT NULL REFERENCES web_crawl_jobs(id) ON DELETE CASCADE,
    url         TEXT NOT NULL,
    depth       INTEGER NOT NULL DEFAULT 0,
    status      VARCHAR(20) NOT NULL DEFAULT 'pending',
    http_status INTEGER,
    title       TEXT,
    created_at  TIMESTAMPTZ NOT NULL,
    CONSTRAINT chk_web_crawl_page_status CHECK (status IN ('pending', 'crawled', 'indexed', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS idx_web_crawl_pages_job_id ON web_crawl_pages(job_id);
