-- Add debug_data column to messages for RAG pipeline debug info (admin panel).
ALTER TABLE messages ADD COLUMN IF NOT EXISTS debug_data TEXT NULL;
