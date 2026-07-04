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
        tracing::debug!(component = "conversations/repository", session_title = %session.title, "session.create.started");

        sqlx::query(
            "INSERT INTO sessions (id, title, collection_id, user_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(session.id)
        .bind(&session.title)
        .bind(session.collection_id)
        .bind(&session.user_id)
        .bind(session.created_at)
        .bind(session.updated_at)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to create session: {e}")))?;

        tracing::info!(component = "conversations/repository", session_id = %session.id, "session.created");
        Ok(session.id)
    }

    /// List all sessions ordered by most recently updated.
    /// Used by admin users who can see all sessions.
    pub async fn list_sessions(&self) -> Result<Vec<Session>, AppError> {
        self.list_sessions_by_user("", true).await
    }

    /// List sessions scoped by user.
    /// Non-admin users see only their own sessions; admin users see all.
    pub async fn list_sessions_by_user(
        &self,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<Session>, AppError> {
        tracing::debug!(
            component = "conversations/repository",
            "session.list.started"
        );

        let rows = if is_admin {
            sqlx::query_as::<_, (uuid::Uuid, String, bool, Option<uuid::Uuid>, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
                "SELECT id, title, pinned, collection_id, user_id, created_at, updated_at FROM sessions ORDER BY pinned DESC, updated_at DESC",
            )
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        } else {
            sqlx::query_as::<_, (uuid::Uuid, String, bool, Option<uuid::Uuid>, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
                "SELECT id, title, pinned, collection_id, user_id, created_at, updated_at FROM sessions WHERE user_id = $1 ORDER BY pinned DESC, updated_at DESC",
            )
            .bind(user_id)
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        };

        let mut sessions = Vec::with_capacity(rows.len());
        for row in rows {
            let count = self.get_message_count(row.0).await.unwrap_or(0);

            sessions.push(Session {
                id: row.0,
                title: row.1,
                pinned: row.2,
                collection_id: row.3,
                user_id: row.4,
                created_at: row.5,
                updated_at: row.6,
                message_count: count,
            });
        }

        tracing::debug!(
            component = "conversations/repository",
            count = sessions.len(),
            "session.list.found"
        );
        Ok(sessions)
    }

    /// Retrieve a single session by ID.
    /// Used by admin users who can access any session.
    pub async fn get_session(&self, id: Uuid) -> Result<Session, AppError> {
        self.get_session_for_user(id, "", true).await
    }

    /// Retrieve a single session by ID with ownership check.
    /// Non-admin users can only access their own sessions.
    pub async fn get_session_for_user(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Session, AppError> {
        tracing::debug!(component = "conversations/repository", session_id = %id, "session.get.started");

        let row = if is_admin {
            sqlx::query_as::<
                _,
                (
                    uuid::Uuid,
                    String,
                    bool,
                    Option<uuid::Uuid>,
                    String,
                    chrono::DateTime<chrono::Utc>,
                    chrono::DateTime<chrono::Utc>,
                ),
            >(
                "SELECT id, title, pinned, collection_id, user_id, created_at, updated_at FROM sessions WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::NotFound(format!("Session {id} not found")))?
        } else {
            sqlx::query_as::<
                _,
                (
                    uuid::Uuid,
                    String,
                    bool,
                    Option<uuid::Uuid>,
                    String,
                    chrono::DateTime<chrono::Utc>,
                    chrono::DateTime<chrono::Utc>,
                ),
            >(
                "SELECT id, title, pinned, collection_id, user_id, created_at, updated_at FROM sessions WHERE id = $1 AND user_id = $2",
            )
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::NotFound(format!("Session {id} not found")))?
        };

        let count = self.get_message_count(id).await.unwrap_or(0);

        Ok(Session {
            id: row.0,
            title: row.1,
            pinned: row.2,
            collection_id: row.3,
            user_id: row.4,
            created_at: row.5,
            updated_at: row.6,
            message_count: count,
        })
    }

    /// Delete a session and its associated messages.
    /// Used by admin users who can delete any session.
    pub async fn delete_session(&self, id: Uuid) -> Result<(), AppError> {
        self.delete_session_for_user(id, "", true).await
    }

    /// Delete a session and its associated messages with ownership check.
    /// Non-admin users can only delete their own sessions.
    pub async fn delete_session_for_user(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<(), AppError> {
        tracing::debug!(component = "conversations/repository", session_id = %id, "session.delete.started");

        // Delete messages first (explicit cascade for clarity)
        let msg_query = if is_admin {
            "DELETE FROM messages WHERE session_id IN (SELECT id FROM sessions WHERE id = $1)"
        } else {
            "DELETE FROM messages WHERE session_id IN (SELECT id FROM sessions WHERE id = $1 AND user_id = $2)"
        };
        sqlx::query(msg_query)
            .bind(id)
            .bind(user_id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete messages: {e}")))?;

        // Delete the session
        let query_str = if is_admin {
            "DELETE FROM sessions WHERE id = $1".to_string()
        } else {
            "DELETE FROM sessions WHERE id = $1 AND user_id = $2".to_string()
        };
        let mut q = sqlx::query(&query_str).bind(id);
        if !is_admin {
            q = q.bind(user_id);
        }
        let affected = q
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete session: {e}")))?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Session {id} not found")));
        }

        tracing::info!(component = "conversations/repository", session_id = %id, "session.deleted");
        Ok(())
    }

    /// Insert a message into PostgreSQL.
    pub async fn add_message(&self, msg: &Message) -> Result<(), AppError> {
        tracing::debug!(component = "conversations/repository", session_id = %msg.session_id, role = %msg.role, "message.add.started");

        let sources_json = msg
            .sources
            .as_ref()
            .map(|s| {
                serde_json::from_str::<serde_json::Value>(s).unwrap_or(serde_json::Value::Null)
            })
            .unwrap_or(serde_json::Value::Null);

        sqlx::query(
            "INSERT INTO messages (id, session_id, role, content, sources, created_at, edited_at, original_content, deleted_at, debug_data) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(msg.id)
        .bind(msg.session_id)
        .bind(&msg.role)
        .bind(&msg.content)
        .bind(&sources_json)
        .bind(msg.created_at)
        .bind(msg.edited_at)
        .bind(&msg.original_content)
        .bind(msg.deleted_at)
        .bind(&msg.debug_data)
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

        tracing::debug!(component = "conversations/repository", session_id = %msg.session_id, "message.added");
        Ok(())
    }

    /// Retrieve live (non-deleted) messages for a session, ordered by creation time.
    pub async fn get_messages(&self, session_id: Uuid) -> Result<Vec<Message>, AppError> {
        tracing::debug!(component = "conversations/repository", session_id = %session_id, "message.list.started");

        let rows = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, String, String, Option<serde_json::Value>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, Option<String>, Option<chrono::DateTime<chrono::Utc>>, Option<String>)>(
            "SELECT id, session_id, role, content, sources, created_at, edited_at, original_content, deleted_at, debug_data \
             FROM messages WHERE session_id = $1 AND deleted_at IS NULL ORDER BY created_at",
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
                edited_at: row.6,
                original_content: row.7,
                deleted_at: row.8,
                debug_data: row.9,
            })
            .collect();

        tracing::debug!(component = "conversations/repository", session_id = %session_id, count = messages.len(), "message.list.found");
        Ok(messages)
    }

    /// Retrieve a single message by ID.
    /// Returns NotFound if the message does not exist or is soft-deleted.
    pub async fn get_message(&self, id: Uuid) -> Result<Message, AppError> {
        tracing::debug!(component = "conversations/repository", message_id = %id, "message.get.started");

        let row = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, String, String, Option<serde_json::Value>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, Option<String>, Option<chrono::DateTime<chrono::Utc>>, Option<String>)>(
            "SELECT id, session_id, role, content, sources, created_at, edited_at, original_content, deleted_at, debug_data \
             FROM messages WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
        .ok_or_else(|| AppError::NotFound(format!("Message {id} not found")))?;

        // Treat soft-deleted messages as not found
        if row.8.is_some() {
            return Err(AppError::NotFound(format!("Message {id} not found")));
        }

        Ok(Message {
            id: row.0,
            session_id: row.1,
            role: row.2,
            content: row.3,
            sources: row.4.map(|v| v.to_string()),
            created_at: row.5,
            edited_at: row.6,
            original_content: row.7,
            deleted_at: row.8,
            debug_data: row.9,
        })
    }

    /// Update the content of a message. On the first edit, preserves the
    /// original content for audit trail. Sets `edited_at` on every edit.
    pub async fn update_message(&self, id: Uuid, new_content: String) -> Result<Message, AppError> {
        tracing::debug!(component = "conversations/repository", message_id = %id, "message.update.started");

        // Fetch current message to get original content if not yet set
        let current = self.get_message(id).await?;

        // If this is the first edit, preserve the original content
        let original = current
            .original_content
            .clone()
            .unwrap_or(current.content.clone());

        sqlx::query(
            "UPDATE messages SET content = $1, edited_at = $2, original_content = $3 WHERE id = $4 AND deleted_at IS NULL",
        )
        .bind(&new_content)
        .bind(chrono::Utc::now())
        .bind(&original)
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to update message: {e}")))?;

        // Fetch the updated message
        self.get_message(id).await
    }

    /// Soft-delete a message by setting `deleted_at` to now.
    pub async fn soft_delete_message(&self, id: Uuid) -> Result<(), AppError> {
        tracing::info!(component = "conversations/repository", message_id = %id, "message.delete.started");

        let affected =
            sqlx::query("UPDATE messages SET deleted_at = $1 WHERE id = $2 AND deleted_at IS NULL")
                .bind(chrono::Utc::now())
                .bind(id)
                .execute(&self.db)
                .await
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to soft-delete message: {e}"))
                })?;

        if affected.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Message {id} not found or already deleted"
            )));
        }

        tracing::info!(component = "conversations/repository", message_id = %id, "message.deleted");
        Ok(())
    }

    /// Delete all sessions and their messages.
    /// Used by admin users who can delete all sessions.
    /// Returns the number of sessions deleted.
    pub async fn delete_all_sessions(&self) -> Result<u64, AppError> {
        tracing::warn!(
            component = "conversations/repository",
            "session.delete_all.started"
        );

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
        tracing::info!(
            component = "conversations/repository",
            count = count,
            "session.delete_all.done"
        );
        Ok(count)
    }

    /// Delete all sessions belonging to a specific user.
    /// Returns the number of sessions deleted.
    pub async fn delete_all_sessions_for_user(&self, user_id: &str) -> Result<u64, AppError> {
        tracing::warn!(
            component = "conversations/repository",
            user_id = %user_id,
            "session.delete_all_for_user.started"
        );

        // Delete messages for this user's sessions first
        sqlx::query(
            "DELETE FROM messages WHERE session_id IN (SELECT id FROM sessions WHERE user_id = $1)",
        )
        .bind(user_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to delete messages: {e}")))?;

        // Delete this user's sessions
        let result = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete sessions: {e}")))?;

        let count = result.rows_affected();
        tracing::info!(
            component = "conversations/repository",
            user_id = %user_id,
            count = count,
            "session.delete_all_for_user.done"
        );
        Ok(count)
    }

    /// Update a session's title and/or pinned status.
    /// Only the provided fields are updated (Option::None = no change).
    pub async fn update_session(
        &self,
        id: Uuid,
        title: Option<String>,
        pinned: Option<bool>,
    ) -> Result<Session, AppError> {
        tracing::debug!(component = "conversations/repository", session_id = %id, "session.update.started");

        let current = self.get_session(id).await?;
        let new_title = title.unwrap_or(current.title);
        let new_pinned = pinned.unwrap_or(current.pinned);

        sqlx::query("UPDATE sessions SET title = $1, pinned = $2, updated_at = $3 WHERE id = $4")
            .bind(&new_title)
            .bind(new_pinned)
            .bind(chrono::Utc::now())
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to update session: {e}")))?;

        tracing::info!(component = "conversations/repository", session_id = %id, session_title = %new_title, pinned = new_pinned, "session.updated");

        // Re-fetch to get consistent state
        self.get_session(id).await
    }

    /// Count live (non-deleted) messages belonging to a session.
    pub async fn get_message_count(&self, session_id: Uuid) -> Result<i64, AppError> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM messages WHERE session_id = $1 AND deleted_at IS NULL",
        )
        .bind(session_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        Ok(row.0)
    }

    /// Search sessions by title substring and/or date range.
    /// All parameters are optional — omitted filters are not applied.
    pub async fn search_sessions(
        &self,
        search: Option<String>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        user_id: Option<String>,
    ) -> Result<Vec<Session>, AppError> {
        tracing::debug!(
            "Searching sessions: search={:?} from={:?} to={:?} user_id={:?}",
            search,
            from,
            to,
            user_id
        );

        let mut sql = String::from(
            "SELECT id, title, pinned, collection_id, created_at, updated_at, user_id FROM sessions WHERE 1=1"
        );

        if search.is_some() {
            sql.push_str(" AND (title ILIKE $1 OR EXISTS (SELECT 1 FROM messages m WHERE m.session_id = sessions.id AND m.content ILIKE $1))");
        }
        if from.is_some() {
            sql.push_str(" AND created_at >= $2");
        }
        if to.is_some() {
            sql.push_str(" AND created_at <= $3");
        }
        if user_id.is_some() {
            sql.push_str(" AND user_id = $4");
        }

        sql.push_str(" ORDER BY updated_at DESC");

        let mut query = sqlx::query_as::<
            _,
            (
                uuid::Uuid,
                String,
                bool,
                Option<uuid::Uuid>,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
                String,
            ),
        >(&sql);

        if search.is_some() {
            query = query.bind(format!("%{}%", search.unwrap_or_default()));
        }
        if let Some(d) = from {
            query = query.bind(d);
        }
        if let Some(d) = to {
            query = query.bind(d);
        }
        if let Some(uid) = user_id {
            query = query.bind(uid);
        }

        let rows = query
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

        let mut sessions = Vec::with_capacity(rows.len());
        for row in rows {
            let count = self.get_message_count(row.0).await.unwrap_or(0);
            sessions.push(Session {
                id: row.0,
                title: row.1,
                pinned: row.2,
                collection_id: row.3,
                created_at: row.4,
                updated_at: row.5,
                user_id: row.6,
                message_count: count,
            });
        }

        tracing::debug!("Found {} sessions matching criteria", sessions.len());
        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    // Tests migrated to sqlx::test with PostgreSQL fixtures (Phase 3)
}
