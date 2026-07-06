-- Create settings table (v0.5 — configurable RAG pipeline settings)
-- Stores application settings as key-value pairs with JSONB values
-- to support multiple types (boolean, integer, string).
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
