use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Incoming request to ask a question against a collection.
#[derive(Debug, Clone, Deserialize)]
pub struct QueryRequest {
    /// The collection to search.
    pub collection_id: Uuid,
    /// The user's question.
    pub query: String,
    /// Optional session for conversation continuity.
    pub session_id: Option<Uuid>,
    /// Enable debug data collection (admin users).
    #[serde(default)]
    pub debug: bool,
}

/// A single source reference returned in the SSE "sources" event.
#[derive(Debug, Clone, Serialize)]
pub struct SourceRef {
    pub document_id: Uuid,
    pub document_name: String,
    pub chunk_index: usize,
    pub text: String,
    pub relevance: f64,
}

/// Internal event type yielded by the query stream before conversion to SSE.
#[derive(Debug, Clone, Serialize)]
pub struct StreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

/// Data included in the `done` SSE event payload.
#[derive(Debug, Clone, Serialize)]
pub struct DonePayload {
    #[serde(rename = "type")]
    pub event_type: String,
    pub user_message_id: Option<Uuid>,
    pub assistant_message_id: Option<Uuid>,
}
