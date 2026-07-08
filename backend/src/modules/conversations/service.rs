use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::modules::conversations::models::{
    CreateSessionRequest, Message, Session, SessionSummary, UpdateMessageRequest,
    UpdateSessionRequest,
};
use crate::modules::conversations::repository::ConversationRepository;
use crate::shared::error::AppError;
use crate::shared::llm::LlmClient;

/// Service for conversation management operations.
#[derive(Clone, Debug)]
pub struct ConversationService {
    repo: ConversationRepository,
    llm: LlmClient,
}

impl ConversationService {
    /// Create a new ConversationService.
    pub fn new(repo: ConversationRepository, llm: LlmClient) -> Self {
        Self { repo, llm }
    }

    /// Create a new session with an optional title and collection association.
    pub async fn create_session(
        &self,
        req: CreateSessionRequest,
        user_id: &str,
        user_name: Option<String>,
    ) -> Result<SessionSummary, AppError> {
        let now = chrono::Utc::now();
        let title = req
            .title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "New Chat".to_string());

        let session = Session {
            id: Uuid::new_v4(),
            title,
            pinned: false,
            collection_id: req.collection_id,
            user_id: user_id.to_string(),
            user_name,
            created_at: now,
            updated_at: now,
            message_count: 0,
        };

        let id = self.repo.create_session(&session).await?;

        tracing::info!(component = "conversations/service", session_id = %id, session_title = %session.title, "session.created");

        Ok(SessionSummary {
            id,
            title: session.title,
            message_count: 0,
            pinned: false,
            collection_id: session.collection_id,
            created_at: now,
            updated_at: now,
            user_name: session.user_name,
        })
    }

    /// List all sessions, most recently updated first.
    pub async fn list_sessions(
        &self,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<SessionSummary>, AppError> {
        tracing::debug!(component = "conversations/service", user_id = %user_id, is_admin = is_admin, "session.list");

        let sessions = self.repo.list_sessions_by_user(user_id, is_admin).await?;
        let summaries = sessions
            .into_iter()
            .map(|s| SessionSummary {
                id: s.id,
                title: s.title,
                message_count: s.message_count,
                pinned: s.pinned,
                collection_id: s.collection_id,
                created_at: s.created_at,
                updated_at: s.updated_at,
                user_name: s.user_name,
            })
            .collect();

        Ok(summaries)
    }

    /// Get a session with its full message history.
    pub async fn get_session_history(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<(Session, Vec<Message>), AppError> {
        tracing::debug!(component = "conversations/service", session_id = %id, "session.get");

        let session = self
            .repo
            .get_session_for_user(id, user_id, is_admin)
            .await?;
        let messages = self.repo.get_messages(id).await?;

        Ok((session, messages))
    }

    /// Delete a session and its messages.
    pub async fn delete_session(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<(), AppError> {
        tracing::info!(component = "conversations/service", session_id = %id, user_id = %user_id, is_admin = is_admin, "session.delete");
        self.repo
            .delete_session_for_user(id, user_id, is_admin)
            .await
    }

    /// Delete all sessions and their messages.
    /// Returns a JSON response with the count of deleted sessions.
    pub async fn delete_all_sessions(
        &self,
        user_id: &str,
        is_admin: bool,
    ) -> Result<serde_json::Value, AppError> {
        tracing::warn!(component = "conversations/service", user_id = %user_id, is_admin = is_admin, "session.delete_all");

        let count = if is_admin {
            self.repo.delete_all_sessions().await?
        } else {
            self.repo.delete_all_sessions_for_user(user_id).await?
        };

        Ok(serde_json::json!({
            "status": "deleted",
            "count": count,
        }))
    }

    /// Update a message's content.
    ///
    /// Only user messages can be edited. Assistant messages return
    /// `UnprocessableEntity`. Validates content length (1..8000 chars).
    pub async fn update_message(
        &self,
        session_id: Uuid,
        msg_id: Uuid,
        req: UpdateMessageRequest,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Message, AppError> {
        tracing::info!(component = "conversations/service", session_id = %session_id, message_id = %msg_id, new_len = req.content.len(), "message.update");

        // Validate content length
        if req.content.is_empty() || req.content.len() > 8000 {
            return Err(AppError::BadRequest(
                "Content must be between 1 and 8000 characters".to_string(),
            ));
        }

        // Verify session ownership first
        self.repo
            .get_session_for_user(session_id, user_id, is_admin)
            .await?;

        // Fetch the current message to check role
        let msg = self.repo.get_message(msg_id).await?;

        // Only user messages can be edited
        if msg.role != "user" {
            tracing::warn!(component = "conversations/service", session_id = %session_id, message_id = %msg_id, role = %msg.role, "message.update.rejected_role");
            return Err(AppError::UnprocessableEntity(
                "Assistant messages cannot be edited".to_string(),
            ));
        }

        let old_len = msg.content.len();
        let updated = self.repo.update_message(msg_id, req.content).await?;

        tracing::info!(component = "conversations/service", session_id = %session_id, message_id = %msg_id, old_len = old_len, new_len = updated.content.len(), "message.updated");

        Ok(updated)
    }

    /// Update a session's title or pinned status.
    pub async fn update_session(
        &self,
        id: Uuid,
        req: UpdateSessionRequest,
        user_id: &str,
        is_admin: bool,
    ) -> Result<SessionSummary, AppError> {
        tracing::info!(component = "conversations/service", session_id = %id, session_title = ?req.title, pinned = ?req.pinned, "session.update");

        // Verify ownership first
        self.repo
            .get_session_for_user(id, user_id, is_admin)
            .await?;

        let updated = self.repo.update_session(id, req.title, req.pinned).await?;

        Ok(SessionSummary {
            id: updated.id,
            title: updated.title,
            message_count: updated.message_count,
            pinned: updated.pinned,
            collection_id: updated.collection_id,
            created_at: updated.created_at,
            updated_at: updated.updated_at,
            user_name: updated.user_name,
        })
    }

    /// Generate a concise title for a session using the LLM.
    ///
    /// Reads the first user message in the session, sends it to the LLM
    /// with a summarization prompt, and updates the session title with
    /// the generated short phrase.
    pub async fn generate_title(
        &self,
        session_id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<String, AppError> {
        tracing::info!(
            component = "conversations/service",
            session_id = %session_id,
            "session.generate_title"
        );

        // Get the session and its messages
        let (_session, messages) = self
            .get_session_history(session_id, user_id, is_admin)
            .await?;

        // Find the first user message and the first assistant response
        let mut first_query: Option<&str> = None;
        let mut first_response: Option<&str> = None;

        for msg in &messages {
            if msg.role == "user" && first_query.is_none() {
                first_query = Some(msg.content.as_str());
            } else if msg.role == "assistant" && first_response.is_none() {
                first_response = Some(msg.content.as_str());
                break;
            }
        }

        let first_query = first_query.unwrap_or("");

        if first_query.is_empty() {
            return Err(AppError::BadRequest(
                "No user messages in session".to_string(),
            ));
        }

        // Build prompt — include the assistant response for better context
        let system_prompt = "You are a summarization assistant. Summarize the user's question as a short phrase (up to 5 words). Reply with only the phrase, no quotes, no punctuation, no extra text.";
        let user_prompt = match first_response {
            Some(response) => {
                let truncated = if response.len() > 300 {
                    format!("{}...", &response[..300])
                } else {
                    response.to_string()
                };
                format!(
                    "User query: {}\n\nAssistant response: {}",
                    first_query, truncated
                )
            }
            None => format!("User query: {}", first_query),
        };

        let title = self.llm.query_single(system_prompt, &user_prompt).await?;
        let title = title.trim().to_string();

        // Update the session title
        self.repo
            .update_session(session_id, Some(title.clone()), None)
            .await?;

        tracing::info!(
            component = "conversations/service",
            session_id = %session_id,
            new_title = %title,
            "session.title_generated"
        );

        Ok(title)
    }

    /// Soft-delete a message.
    pub async fn delete_message(
        &self,
        session_id: Uuid,
        msg_id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<(), AppError> {
        tracing::info!(component = "conversations/service", session_id = %session_id, message_id = %msg_id, "message.delete");

        // Verify session ownership first
        self.repo
            .get_session_for_user(session_id, user_id, is_admin)
            .await?;

        self.repo.soft_delete_message(msg_id).await
    }

    /// Export a session (with messages) as a JSON value.
    /// Soft-deleted messages are excluded.
    pub async fn export_session(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<serde_json::Value, AppError> {
        tracing::debug!(component = "conversations/service", session_id = %id, "session.export");

        let (session, messages) = self.get_session_history(id, user_id, is_admin).await?;

        Ok(serde_json::json!({
            "session": {
                "id": session.id,
                "title": session.title,
                "collection_id": session.collection_id,
                "created_at": session.created_at,
                "updated_at": session.updated_at,
            },
            "messages": messages.into_iter().map(|m| serde_json::json!({
                "id": m.id,
                "role": m.role,
                "content": m.content,
                "sources": m.sources,
                "created_at": m.created_at,
                "edited_at": m.edited_at,
                "original_content": m.original_content,
            })).collect::<Vec<_>>(),
        }))
    }

    /// Build a markdown export of a session.
    ///
    /// Format:
    /// ```markdown
    /// # Session Title
    ///
    /// ## user · 2026-06-21T12:00:00Z
    /// (edited)
    /// Message content
    ///
    /// ---
    ///
    /// ## assistant · 2026-06-21T12:00:05Z
    ///
    /// Response content
    ///
    /// ---
    /// ```
    pub async fn export_session_markdown(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<String, AppError> {
        tracing::info!(component = "conversations/service", session_id = %id, format = "markdown", "session.export");

        let (session, messages) = self.get_session_history(id, user_id, is_admin).await?;

        let mut lines = Vec::new();
        lines.push(format!("# {}", session.title));
        lines.push(String::new());

        for msg in &messages {
            let header = format!("## {} · {}", msg.role, msg.created_at.to_rfc3339());
            lines.push(header);
            lines.push(String::new());
            if msg.edited_at.is_some() {
                lines.push("*(edited)*".to_string());
            }
            lines.push(msg.content.clone());
            lines.push(String::new());
            lines.push("---".to_string());
            lines.push(String::new());
        }

        let result = lines.join("\n");
        tracing::info!(component = "conversations/service", session_id = %id, format = "markdown", bytes = result.len(), "session.exported");

        Ok(result)
    }

    /// Search sessions with optional title filter and date range.
    pub async fn search_sessions(
        &self,
        search: Option<String>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        user_name: Option<String>,
    ) -> Result<Vec<Session>, AppError> {
        tracing::info!(
            "[conv.search_sessions] search={:?} from={:?} to={:?} user_name={:?}",
            search,
            from,
            to,
            user_name
        );
        self.repo.search_sessions(search, from, to, user_name).await
    }

    /// Get distinct user names from all sessions.
    pub async fn list_session_users(&self) -> Result<Vec<String>, AppError> {
        tracing::debug!(component = "conversations/service", "session.list_users");
        self.repo.get_distinct_user_names().await
    }
}
