use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A document stored in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub name: String,
    pub file_type: String,
    pub file_size: i64,
    pub uploaded_at: DateTime<Utc>,
    pub collection_id: Uuid,
    pub is_active: bool,
}

/// Response returned after a successful document upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub document_id: Uuid,
    pub chunks_indexed: usize,
    pub document_name: String,
}

/// Summary view of a document (without full text).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSummary {
    pub id: Uuid,
    pub name: String,
    pub file_type: String,
    pub file_size: i64,
    pub uploaded_at: DateTime<Utc>,
    pub collection_id: Uuid,
    pub is_active: bool,
}

/// A chunk of text extracted from a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: Uuid,
    pub document_id: Uuid,
    pub index: usize,
    pub text: String,
    pub is_active: bool,
}

/// Result of processing a single file within a ZIP archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipUploadItem {
    pub filename: String,
    pub status: String,
    pub document_id: Option<Uuid>,
    pub error: Option<String>,
}

/// Response returned after processing a ZIP batch upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipUploadResponse {
    pub total_files: usize,
    pub processed: usize,
    pub failed: usize,
    pub items: Vec<ZipUploadItem>,
}
