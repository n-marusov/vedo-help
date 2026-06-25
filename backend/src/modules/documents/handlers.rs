use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::modules::auth::models::UserContext;
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
    user_ctx: UserContext,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    let is_admin = user_ctx.roles.contains(&"admin".to_string());
    tracing::info!(component = "documents/handlers", user_id = %user_ctx.user_id, "document.upload.request");

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
                    component = "documents/handlers",
                    file_name = %filename.as_deref().unwrap_or("unknown"),
                    content_type = %content_type.as_deref().unwrap_or(""),
                    file_size = data.len(),
                    "document.upload.file.received"
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
        .process_upload(
            &data,
            &filename,
            collection_id,
            content_type,
            &user_ctx.user_id,
            is_admin,
        )
        .await?;

    tracing::info!(
        component = "documents/handlers",
        document_id = %response.document_id,
        chunk_count = response.chunks_indexed,
        "document.upload.complete"
    );

    Ok(Json(response))
}

/// List documents, optionally filtered by collection.
///
/// Endpoint: `GET /api/documents`
pub async fn list(
    State(svc): State<DocumentService>,
    user_ctx: UserContext,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Json<Vec<DocumentSummary>>, AppError> {
    let is_admin = user_ctx.roles.contains(&"admin".to_string());
    let collection_id = query.collection_id.unwrap_or_default();
    tracing::info!(component = "documents/handlers", collection_id = %collection_id, user_id = %user_ctx.user_id, "document.list");
    let documents = svc
        .list_documents(collection_id, &user_ctx.user_id, is_admin)
        .await?;
    Ok(Json(documents))
}

/// Delete a document by ID.
///
/// Endpoint: `DELETE /api/documents/:id`
pub async fn delete(
    State(svc): State<DocumentService>,
    user_ctx: UserContext,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let is_admin = user_ctx.roles.contains(&"admin".to_string());
    tracing::info!(component = "documents/handlers", document_id = %id, user_id = %user_ctx.user_id, "document.delete");
    svc.delete_document(id, &user_ctx.user_id, is_admin).await?;
    Ok(Json(serde_json::json!({"status": "deleted", "id": id})))
}

/// Delete documents by IDs.
///
/// Endpoint: `DELETE /api/documents/batch`
pub async fn delete_batch(
    State(svc): State<DocumentService>,
    user_ctx: UserContext,
    Json(req): Json<BatchDeleteRequest>,
) -> Result<Json<BatchDeleteResponse>, AppError> {
    let is_admin = user_ctx.roles.contains(&"admin".to_string());
    tracing::info!(
        component = "documents/handlers",
        request_count = req.ids.len(),
        user_id = %user_ctx.user_id,
        "document.batch_delete.request"
    );
    tracing::debug!(
        component = "documents/handlers",
        document_ids = ?req.ids,
        "document.batch_delete.ids"
    );

    if req.ids.is_empty() {
        tracing::warn!(
            component = "documents/handlers",
            "document.batch_delete.empty_ids"
        );
        return Err(AppError::BadRequest("No document IDs provided".to_string()));
    }

    let response = svc
        .delete_documents_batch(req.ids, &user_ctx.user_id, is_admin)
        .await?;
    tracing::info!(
        component = "documents/handlers",
        deleted_count = response.deleted_count,
        "document.batch_delete.complete"
    );

    Ok(Json(response))
}

/// Reload/re-index a document via multipart file upload.
///
/// Endpoint: `POST /api/documents/reload/{id}`
pub async fn reload(
    State(svc): State<DocumentService>,
    user_ctx: UserContext,
    Path(id): Path<Uuid>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    let is_admin = user_ctx.roles.contains(&"admin".to_string());
    tracing::info!(component = "documents/handlers", document_id = %id, user_id = %user_ctx.user_id, "document.reload.request");

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
                component = "documents/handlers",
                file_name = %filename.as_deref().unwrap_or("unknown"),
                file_size = data.len(),
                "document.reload.file.received"
            );
            file_data = Some(data.to_vec());
        } else {
            tracing::warn!("Unknown field in multipart: {name}");
        }
    }

    let filename = filename.unwrap_or_else(|| "unknown".to_string());
    let data = file_data.ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;

    let response = svc
        .reload_document(&data, &filename, id, &user_ctx.user_id, is_admin)
        .await?;

    tracing::info!(
        component = "documents/handlers",
        document_id = %response.document_id,
        chunk_count = response.chunks_indexed,
        "document.reload.complete"
    );

    Ok(Json(response))
}

/// Upload a ZIP archive for batch processing.
///
/// Endpoint: `POST /api/documents/upload-zip`
pub async fn upload_zip(
    State(svc): State<DocumentService>,
    user_ctx: UserContext,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<ZipUploadResponse>, AppError> {
    let is_admin = user_ctx.roles.contains(&"admin".to_string());
    tracing::info!(component = "documents/handlers", user_id = %user_ctx.user_id, "zip.upload.request");

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
                tracing::debug!(
                    component = "documents/handlers",
                    file_size = data.len(),
                    "zip.upload.file.received"
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
    let data = file_data.ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;

    tracing::info!(
        component = "documents/handlers",
        collection_id = %collection_id,
        file_size = data.len(),
        "zip.upload.processing"
    );

    let response = svc
        .process_zip_upload(&data, collection_id, &user_ctx.user_id, is_admin)
        .await?;

    tracing::info!(
        component = "documents/handlers",
        processed_count = response.processed,
        total_count = response.total_files,
        "zip.upload.complete"
    );

    Ok(Json(response))
}
