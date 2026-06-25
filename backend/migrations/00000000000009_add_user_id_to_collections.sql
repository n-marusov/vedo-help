-- Add user_id column to collections table (v0.6 multi-tenancy)
ALTER TABLE collections
    ADD COLUMN IF NOT EXISTS user_id VARCHAR(255) NOT NULL DEFAULT '';

-- Index for efficient per-user queries
CREATE INDEX IF NOT EXISTS idx_collections_user_id
    ON collections (user_id, id);
