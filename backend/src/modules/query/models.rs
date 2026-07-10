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
    /// When resending an edited message, the ID of the existing user message
    /// that was already persisted. The backend will skip creating a new user
    /// message and use this ID for the pipeline's done payload.
    #[serde(default)]
    pub existing_user_message_id: Option<Uuid>,
}

/// A single source reference returned in the SSE "sources" event.
#[derive(Debug, Clone, Serialize)]
pub struct SourceRef {
    pub document_id: Uuid,
    pub document_name: String,
    pub chunk_index: usize,
    pub text: String,
    pub relevance: f64,
    /// RAG pipeline stage that produced this source ("embedding" | "keyword" | "reranked")
    pub stage: Option<String>,
    /// LLM reranking score (0.0-1.0) for reranked sources
    pub rerank_score: Option<f64>,
    /// LLM reranking verdict ("брать" | "пропустить")
    pub rerank_verdict: Option<String>,
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
