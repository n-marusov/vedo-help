use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::conversations::models::{Message, Session};
use crate::shared::error::AppError;

/// Repository for session and message data access.
#[derive(Clone, Debug)]
pub struct ConversationRepository {
    db: PgPool,
}

impl ConversationRepository {
    /// Create a new ConversationRepository with the given database pool.
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Insert a new session into PostgreSQL.
    pub async fn create_session(&self, session: &Session) -> Result<Uuid, AppError> {
        tracing::debug!("Creating session: {}", session.title);

        sqlx::query(
            "INSERT INTO sessions (id, title, collection_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(session.id)
        .bind(&session.title)
        .bind(session.collection_id)
        .bind(session.created_at)
        .bind(session.updated_at)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to create session: {e}")))?;

        tracing::info!("Session created: {id}", id = session.id);
        Ok(session.id)
    }

    /// List all sessions ordered by most recently updated.
    pub async fn list_sessions(&self) -> Result<Vec<Session>, AppError> {
        tracing::debug!("Listing all sessions");

        let rows = sqlx::query_as::<_, (uuid::Uuid, String, Option<uuid::Uuid>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, title, collection_id, created_at, updated_at FROM sessions ORDER BY updated_at DESC",
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let mut sessions = Vec::with_capacity(rows.len());
        for row in rows {
            let count = self.get_message_count(row.0).await.unwrap_or(0);

            sessions.push(Session {
                id: row.0,
                title: row.1,
                collection_id: row.2,
                created_at: row.3,
                updated_at: row.4,
                message_count: count,
            });
        }

        tracing::debug!("Found {} sessions", sessions.len());
        Ok(sessions)
    }

    /// Retrieve a single session by ID.
    pub async fn get_session(&self, id: Uuid) -> Result<Session, AppError> {
        tracing::debug!("Fetching session: {id}");

        let row = sqlx::query_as::<
            _,
            (
                uuid::Uuid,
                String,
                Option<uuid::Uuid>,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            "SELECT id, title, collection_id, created_at, updated_at FROM sessions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        .ok_or_else(|| AppError::NotFound(format!("Session {id} not found")))?;

        let count = self.get_message_count(id).await.unwrap_or(0);

        Ok(Session {
            id: row.0,
            title: row.1,
            collection_id: row.2,
            created_at: row.3,
            updated_at: row.4,
            message_count: count,
        })
    }

    /// Delete a session and its associated messages.
    pub async fn delete_session(&self, id: Uuid) -> Result<(), AppError> {
        tracing::debug!("Deleting session: {id}");

        // Delete messages first (explicit cascade for clarity)
        sqlx::query("DELETE FROM messages WHERE session_id = $1")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete messages: {e}")))?;

        // Delete the session
        let affected = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete session: {e}")))?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Session {id} not found")));
        }

        tracing::info!("Session deleted: {id}");
        Ok(())
    }

    /// Insert a message into PostgreSQL.
    pub async fn add_message(&self, msg: &Message) -> Result<(), AppError> {
        tracing::debug!(
            "Adding message to session {}: role={}",
            msg.session_id,
            msg.role
        );

        let sources_json = msg
            .sources
            .as_ref()
            .map(|s| {
                serde_json::from_str::<serde_json::Value>(s).unwrap_or(serde_json::Value::Null)
            })
            .unwrap_or(serde_json::Value::Null);

        sqlx::query(
            "INSERT INTO messages (id, session_id, role, content, sources, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(msg.id)
        .bind(msg.session_id)
        .bind(&msg.role)
        .bind(&msg.content)
        .bind(&sources_json)
        .bind(msg.created_at)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to add message: {e}")))?;

        // Update session updated_at timestamp
        sqlx::query("UPDATE sessions SET updated_at = $1 WHERE id = $2")
            .bind(chrono::Utc::now())
            .bind(msg.session_id)
            .execute(&self.db)
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to update session timestamp: {e}"))
            })?;

        tracing::debug!("Message added to session {}", msg.session_id);
        Ok(())
    }

    /// Retrieve messages for a session, ordered by creation time.
    pub async fn get_messages(&self, session_id: Uuid) -> Result<Vec<Message>, AppError> {
        tracing::debug!("Fetching messages for session: {session_id}");

        let rows = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, String, String, Option<serde_json::Value>, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, session_id, role, content, sources, created_at FROM messages WHERE session_id = $1 ORDER BY created_at",
        )
        .bind(session_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let messages: Vec<Message> = rows
            .into_iter()
            .map(|row| Message {
                id: row.0,
                session_id: row.1,
                role: row.2,
                content: row.3,
                sources: row.4.map(|v| v.to_string()),
                created_at: row.5,
            })
            .collect();

        tracing::debug!("Found {} messages for session {session_id}", messages.len());
        Ok(messages)
    }

    /// Delete all sessions and their messages.
    /// Returns the number of sessions deleted.
    pub async fn delete_all_sessions(&self) -> Result<u64, AppError> {
        tracing::warn!("Deleting all sessions");

        // Delete all messages first
        sqlx::query("DELETE FROM messages")
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete messages: {e}")))?;

        // Delete all sessions
        let result = sqlx::query("DELETE FROM sessions")
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete sessions: {e}")))?;

        let count = result.rows_affected();
        tracing::info!("Deleted {count} sessions");
        Ok(count)
    }

    /// Count messages belonging to a session.
    async fn get_message_count(&self, session_id: Uuid) -> Result<i64, AppError> {
        let row =
            sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM messages WHERE session_id = $1")
                .bind(session_id)
                .fetch_one(&self.db)
                .await
                .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        Ok(row.0)
    }
}

#[cfg(test)]
mod tests {
    // Tests migrated to sqlx::test with PostgreSQL fixtures (Phase 3)
}
