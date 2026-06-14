use uuid::Uuid;

use crate::modules::conversations::models::{
    CreateSessionRequest, Message, Session, SessionSummary,
};
use crate::modules::conversations::repository::ConversationRepository;
use crate::shared::error::AppError;

/// Service for conversation management operations.
#[derive(Clone, Debug)]
pub struct ConversationService {
    repo: ConversationRepository,
}

impl ConversationService {
    /// Create a new ConversationService.
    pub fn new(repo: ConversationRepository) -> Self {
        Self { repo }
    }

    /// Create a new session with an optional title and collection association.
    pub async fn create_session(
        &self,
        req: CreateSessionRequest,
    ) -> Result<SessionSummary, AppError> {
        let now = chrono::Utc::now();
        let title = req
            .title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "New Chat".to_string());

        let session = Session {
            id: Uuid::new_v4(),
            title,
            collection_id: req.collection_id,
            created_at: now,
            updated_at: now,
            message_count: 0,
        };

        let id = self.repo.create_session(&session).await?;

        tracing::info!("Session created: {id} ({title})", title = session.title);

        Ok(SessionSummary {
            id,
            title: session.title,
            message_count: 0,
            created_at: now,
            updated_at: now,
        })
    }

    /// List all sessions, most recently updated first.
    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>, AppError> {
        tracing::debug!("Listing all sessions");

        let sessions = self.repo.list_sessions().await?;
        let summaries = sessions
            .into_iter()
            .map(|s| SessionSummary {
                id: s.id,
                title: s.title,
                message_count: s.message_count,
                created_at: s.created_at,
                updated_at: s.updated_at,
            })
            .collect();

        Ok(summaries)
    }

    /// Get a session with its full message history.
    pub async fn get_session_history(&self, id: Uuid) -> Result<(Session, Vec<Message>), AppError> {
        tracing::debug!("Fetching session history: {id}");

        let session = self.repo.get_session(id).await?;
        let messages = self.repo.get_messages(id).await?;

        Ok((session, messages))
    }

    /// Delete a session and its messages.
    pub async fn delete_session(&self, id: Uuid) -> Result<(), AppError> {
        tracing::info!("Deleting session: {id}");
        self.repo.delete_session(id).await
    }

    /// Delete all sessions and their messages.
    /// Returns a JSON response with the count of deleted sessions.
    pub async fn delete_all_sessions(&self) -> Result<serde_json::Value, AppError> {
        tracing::warn!("Deleting all sessions");

        let count = self.repo.delete_all_sessions().await?;

        Ok(serde_json::json!({
            "status": "deleted",
            "count": count,
        }))
    }

    /// Export a session (with messages) as a JSON value.
    pub async fn export_session(&self, id: Uuid) -> Result<serde_json::Value, AppError> {
        tracing::debug!("Exporting session: {id}");

        let (session, messages) = self.get_session_history(id).await?;

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
            })).collect::<Vec<_>>(),
        }))
    }
}
