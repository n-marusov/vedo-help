-- Create git_repositories table (PostgreSQL migration)
CREATE TABLE IF NOT EXISTS git_repositories (
    id UUID PRIMARY KEY,
    url TEXT NOT NULL,
    branch VARCHAR(255) NOT NULL DEFAULT 'main',
    access_token TEXT,
    local_path TEXT NOT NULL,
    last_commit_hash TEXT,
    last_synced_at TIMESTAMPTZ,
    collection_id UUID NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'idle' CHECK(status IN ('idle', 'syncing', 'error')),
    webhook_secret TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_git_repos_collection ON git_repositories(collection_id);
