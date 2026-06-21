-- Create sessions table (PostgreSQL migration)
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY,
    title VARCHAR(255) NOT NULL DEFAULT 'New Chat',
    collection_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE SET NULL
);
