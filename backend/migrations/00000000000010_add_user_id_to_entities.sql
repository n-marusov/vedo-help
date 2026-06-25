-- Add user_id column to documents table (v0.6 multi-tenancy)
ALTER TABLE documents
    ADD COLUMN IF NOT EXISTS user_id VARCHAR(255) NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_documents_user_id
    ON documents (user_id, collection_id);

-- Add user_id column to sessions table (v0.6 multi-tenancy)
ALTER TABLE sessions
    ADD COLUMN IF NOT EXISTS user_id VARCHAR(255) NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_sessions_user_id
    ON sessions (user_id, id);

-- Add user_id column to git_repositories table (v0.6 multi-tenancy)
ALTER TABLE git_repositories
    ADD COLUMN IF NOT EXISTS user_id VARCHAR(255) NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_git_repositories_user_id
    ON git_repositories (user_id, collection_id);
