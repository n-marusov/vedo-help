-- Add source column to documents table (v0.3 admin stats: upload vs git)
-- Idempotent via PL/pgSQL DO block: the test helper drops _sqlx_migrations
-- and re-runs migrations, so a plain ALTER TABLE ADD COLUMN would fail with
-- "column already exists" on the second run.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'documents' AND column_name = 'source'
    ) THEN
        ALTER TABLE documents
            ADD COLUMN source VARCHAR(20) NOT NULL DEFAULT 'upload';
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_documents_source' AND table_name = 'documents'
    ) THEN
        ALTER TABLE documents
            ADD CONSTRAINT chk_documents_source CHECK (source IN ('upload', 'git'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_documents_collection_id_source
    ON documents (collection_id, source);
