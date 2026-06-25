use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A collection that groups related documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub document_count: i64,
    /// Owner user ID (KeyCloak sub claim).
    #[serde(skip_serializing)]
    pub user_id: String,
}

/// Request payload for creating a new collection.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub description: Option<String>,
}

/// Lightweight summary of a collection for list views.
#[derive(Debug, Clone, Serialize)]
pub struct CollectionSummary {
    pub id: Uuid,
    pub name: String,
    pub document_count: i64,
    pub created_at: DateTime<Utc>,
}
