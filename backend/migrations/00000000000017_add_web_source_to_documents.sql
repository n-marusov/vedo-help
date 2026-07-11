-- Add 'web' as an allowed source for documents
DO $$ BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_documents_source' AND table_name = 'documents'
    ) THEN
        ALTER TABLE documents DROP CONSTRAINT chk_documents_source;
    END IF;
END $$;

ALTER TABLE documents ADD CONSTRAINT chk_documents_source
    CHECK (source IN ('upload', 'git', 'web'));
