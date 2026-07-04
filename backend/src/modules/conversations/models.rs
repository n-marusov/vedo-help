use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A chat session belonging to a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub title: String,
    pub pinned: bool,
    pub collection_id: Option<Uuid>,
    /// The KeyCloak user `sub` that owns this session.
    /// Never serialized in API responses to prevent info leakage.
    #[serde(skip_serializing)]
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: i64,
    /// Display name for the user (populated from JWT on session creation).
    pub user_name: Option<String>,
}

/// A single message within a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
    /// Sources stored as a JSON string (e.g. chunk citations).
    pub sources: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Timestamp of the last edit (NULL = never edited).
    pub edited_at: Option<DateTime<Utc>>,
    /// Original content before the first edit (audit trail).
    pub original_content: Option<String>,
    /// Soft-delete timestamp (NULL = live).
    pub deleted_at: Option<DateTime<Utc>>,
    /// Debug data JSON blob (admin panel). Populated when query.debug = true.
    pub debug_data: Option<String>,
}

/// Request payload for updating a message.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMessageRequest {
    /// Updated content (1..8000 chars).
    pub content: String,
}

/// Lightweight summary of a session for list views.
#[derive(Debug, Clone, Serialize)]
pub struct SessionSummary {
    pub id: Uuid,
    pub title: String,
    pub message_count: i64,
    pub pinned: bool,
    pub collection_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// User display name (admin sessions only).
    pub user_name: Option<String>,
}

/// Request payload for creating a new session.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
    pub collection_id: Option<Uuid>,
}

/// Request payload for updating a session (title, pinned).
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSessionRequest {
    pub title: Option<String>,
    pub pinned: Option<bool>,
}
