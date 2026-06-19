use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::modules::documents::models::{
    BatchDeleteResponse, DocumentSummary, UploadResponse, ZipUploadResponse,
};
use crate::modules::documents::service::DocumentService;
use crate::shared::error::AppError;

/// Query parameters for listing documents.
#[derive(Debug, Deserialize)]
pub struct ListDocumentsQuery {
    pub collection_id: Option<Uuid>,
}

/// Request body for bulk document deletion.
#[derive(Debug, Deserialize)]
pub struct BatchDeleteRequest {
    pub ids: Vec<Uuid>,
}

/// Upload a document via multipart form data.
///
/// Endpoint: `POST /api/documents/upload`
pub async fn upload(
    State(svc): State<DocumentService>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    tracing::info!("Document upload request received");

    let mut collection_id: Option<Uuid> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart data: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                filename = Some(field.file_name().unwrap_or("unknown").to_string());
                content_type = Some(
                    field
                        .content_type()
                        .map(|m| m.to_string())
                        .unwrap_or_default(),
                );
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Failed to read file data: {e}")))?;
                tracing::debug!(
                    "Uploaded file: {name}, type={ct}, size={}",
                    data.len(),
                    name = filename.as_deref().unwrap_or("unknown"),
                    ct = content_type.as_deref().unwrap_or(""),
                );
                file_data = Some(data.to_vec());
            }
            "collection_id" => {
                let val = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Invalid collection_id: {e}")))?;
                collection_id = Some(Uuid::parse_str(&val).map_err(|_| {
                    AppError::BadRequest(format!("Invalid collection_id format: {val}"))
                })?);
            }
            _ => {
                tracing::warn!("Unknown field in multipart: {name}");
            }
        }
    }

    let collection_id = collection_id
        .ok_or_else(|| AppError::BadRequest("collection_id is required".to_string()))?;
    let filename = filename.unwrap_or_else(|| "unknown".to_string());
    let content_type = content_type.unwrap_or_default();
    let data = file_data.ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;

    let response = svc
        .process_upload(&data, &filename, collection_id, content_type)
        .await?;

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

/// Delete documents by IDs.
///
/// Endpoint: `DELETE /api/documents/batch`
pub async fn delete_batch(
    State(svc): State<DocumentService>,
    Json(req): Json<BatchDeleteRequest>,
) -> Result<Json<BatchDeleteResponse>, AppError> {
    tracing::info!(
        "[documents.delete_batch] bulk delete request received: count={count}",
        count = req.ids.len()
    );
    tracing::debug!(
        "[documents.delete_batch] requested document ids: {:?}",
        req.ids
    );

    if req.ids.is_empty() {
        tracing::warn!("[documents.delete_batch] empty document id list rejected");
        return Err(AppError::BadRequest("No document IDs provided".to_string()));
    }

    let response = svc.delete_documents_batch(req.ids).await?;
    tracing::info!(
        "[documents.delete_batch] bulk delete complete: deleted_count={deleted_count}",
        deleted_count = response.deleted_count
    );

    Ok(Json(response))
}

/// Reload/re-index a document via multipart file upload.
///
/// Endpoint: `POST /api/documents/reload/{id}`
pub async fn reload(
    State(svc): State<DocumentService>,
    Path(id): Path<Uuid>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    tracing::info!("Document reload request received for document: {id}");

    let mut filename: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart data: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            filename = Some(field.file_name().unwrap_or("unknown").to_string());
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read file data: {e}")))?;
            tracing::debug!(
                "Reload file: {}, size={}",
                filename.as_deref().unwrap_or("unknown"),
                data.len()
            );
            file_data = Some(data.to_vec());
        } else {
            tracing::warn!("Unknown field in multipart: {name}");
        }
    }

    let filename = filename.unwrap_or_else(|| "unknown".to_string());
    let data = file_data.ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;

    let response = svc.reload_document(&data, &filename, id).await?;

    tracing::info!(
        "Reload complete: doc_id={}, chunks={}",
        response.document_id,
        response.chunks_indexed
    );

    Ok(Json(response))
}

/// Upload a ZIP archive for batch processing.
///
/// Endpoint: `POST /api/documents/upload-zip`
pub async fn upload_zip(
    State(svc): State<DocumentService>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<ZipUploadResponse>, AppError> {
    tracing::info!("ZIP upload request received");

    let mut collection_id: Option<Uuid> = None;
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart data: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Failed to read file data: {e}")))?;
                tracing::debug!("ZIP file received: {} bytes", data.len());
                file_data = Some(data.to_vec());
            }
            "collection_id" => {
                let val = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Invalid collection_id: {e}")))?;
                collection_id = Some(Uuid::parse_str(&val).map_err(|_| {
                    AppError::BadRequest(format!("Invalid collection_id format: {val}"))
                })?);
            }
            _ => {
                tracing::warn!("Unknown field in multipart: {name}");
            }
        }
    }

    let collection_id = collection_id
        .ok_or_else(|| AppError::BadRequest("collection_id is required".to_string()))?;
    let data = file_data.ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;

    tracing::info!(
        "Processing ZIP upload for collection {collection_id}: {} bytes",
        data.len()
    );

    let response = svc.process_zip_upload(&data, collection_id).await?;

    tracing::info!(
        "ZIP upload complete: {processed}/{total} files",
        processed = response.processed,
        total = response.total_files
    );

    Ok(Json(response))
}
