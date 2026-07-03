-- Add source column to documents table (v0.3 admin stats: upload vs git)
ALTER TABLE documents
    ADD COLUMN source VARCHAR(20) NOT NULL DEFAULT 'upload';

ALTER TABLE documents
    ADD CONSTRAINT chk_documents_source CHECK (source IN ('upload', 'git'));

CREATE INDEX IF NOT EXISTS idx_documents_collection_id_source
    ON documents (collection_id, source);
