use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// Statistics about a collection's documents and chunks.
/// Includes breakdown by source (upload vs git).
#[derive(Debug, Clone, Serialize)]
pub struct CollectionStats {
    pub total_documents: i64,
    pub total_chunks: i64,
    pub total_git_repos: i64,
    pub upload_documents: i64,
    pub git_documents: i64,
    pub upload_chunks: i64,
    pub git_chunks: i64,
    pub total_file_size_bytes: i64,
    pub document_types: HashMap<String, i64>,
}

/// Query parameters for chunk search.
#[derive(Debug, Clone, Deserialize)]
pub struct ChunkSearchQuery {
    pub q: Option<String>,
    pub search_type: Option<String>,
    pub source: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub top_k: Option<usize>,
}

/// Lightweight summary of a collection for list views.
#[derive(Debug, Clone, Serialize)]
pub struct CollectionSummary {
    pub id: Uuid,
    pub name: String,
    pub document_count: i64,
    pub created_at: DateTime<Utc>,
}
