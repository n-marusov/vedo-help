-- Add 'failed' as an allowed status for web_crawl_pages.
-- Required by retry_failed_pages to distinguish pages that were crawled but
-- could not be indexed (embedding error, Chroma error, non-2xx HTTP status).

DO $$ BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_web_crawl_page_status' AND table_name = 'web_crawl_pages'
    ) THEN
        ALTER TABLE web_crawl_pages DROP CONSTRAINT chk_web_crawl_page_status;
    END IF;
END $$;

ALTER TABLE web_crawl_pages ADD CONSTRAINT chk_web_crawl_page_status
    CHECK (status IN ('pending', 'crawled', 'indexed', 'cancelled', 'failed'));