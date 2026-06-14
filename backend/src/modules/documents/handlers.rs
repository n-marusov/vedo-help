use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::modules::documents::models::{DocumentSummary, UploadResponse};
use crate::modules::documents::service::DocumentService;
use crate::shared::error::AppError;

/// Query parameters for listing documents.
#[derive(Debug, Deserialize)]
pub struct ListDocumentsQuery {
    pub collection_id: Option<Uuid>,
}

/// Upload a document via multipart form data.
///
/// Endpoint: `POST /api/documents/upload`
pub async fn upload(
    State(svc): State<DocumentService>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    tracing::info!("Document upload request received");

    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart data: {e}")))?
        .ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;

    let filename = field.file_name().unwrap_or("unknown").to_string();
    let content_type = field
        .content_type()
        .map(|m| m.to_string())
        .unwrap_or_default();
    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to read file data: {e}")))?;

    tracing::debug!(
        "Uploaded file: {filename}, type={content_type}, size={}",
        data.len()
    );

    let response = svc.process_upload(&data, &filename, content_type).await?;

    tracing::info!(
        "Upload complete: doc_id={}, chunks={}",
        response.document_id,
        response.chunks_indexed
    );

    Ok(Json(response))
}

/// List documents, optionally filtered by collection.
///
/// Endpoint: `GET /api/documents`
pub async fn list(
    State(svc): State<DocumentService>,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Json<Vec<DocumentSummary>>, AppError> {
    let collection_id = query.collection_id.unwrap_or_default();
    let documents = svc.list_documents(collection_id).await?;
    Ok(Json(documents))
}

/// Delete a document by ID.
///
/// Endpoint: `DELETE /api/documents/:id`
pub async fn delete(
    State(svc): State<DocumentService>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    svc.delete_document(id).await?;
    Ok(Json(serde_json::json!({"status": "deleted", "id": id})))
}
