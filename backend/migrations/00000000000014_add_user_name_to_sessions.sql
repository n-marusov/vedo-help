-- Add user_name column to sessions table (v0.8 admin session debug)
ALTER TABLE sessions
    ADD COLUMN IF NOT EXISTS user_name TEXT;

-- Index for admin user name search
CREATE INDEX IF NOT EXISTS idx_sessions_user_name
    ON sessions (user_name);
