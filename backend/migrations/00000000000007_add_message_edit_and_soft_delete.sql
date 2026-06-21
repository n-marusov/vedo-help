-- Add edit and soft-delete support for messages (v0.3.1).
--
-- Rationale for soft-delete: messages are part of a conversation history
-- that must be preserved for audit and potential undo. A soft-delete with
-- a timestamp lets the system exclude deleted messages from queries while
-- keeping the data recoverable.
--
-- `edited_at` + `original_content` support message editing with audit trail.
-- Only the FIRST edit preserves `original_content`; subsequent edits update
-- `edited_at` but keep the original.
ALTER TABLE messages
    ADD COLUMN edited_at TIMESTAMPTZ NULL,
    ADD COLUMN original_content TEXT NULL,
    ADD COLUMN deleted_at TIMESTAMPTZ NULL;

-- Partial index to keep live-message scans fast by indexing only non-deleted
-- messages. Also benefits the session-history and message-count queries that
-- filter `deleted_at IS NULL`.
CREATE INDEX IF NOT EXISTS idx_messages_deleted_at
    ON messages (deleted_at)
    WHERE deleted_at IS NULL;
