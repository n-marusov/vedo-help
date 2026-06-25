use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::modules::collections::models::{Collection, CollectionSummary, CreateCollectionRequest};
use crate::modules::collections::service::CollectionService;
use crate::shared::error::AppError;

/// Create a new collection.
///
/// Endpoint: `POST /api/collections`
pub async fn create(
    State(svc): State<CollectionService>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<CollectionSummary>, AppError> {
    tracing::info!(component = "collections/handlers", collection_name = %req.name, "collection.create");
    let summary = svc.create(req).await?;
    Ok(Json(summary))
}

/// List all collections.
///
/// Endpoint: `GET /api/collections`
pub async fn list(
    State(svc): State<CollectionService>,
) -> Result<Json<Vec<CollectionSummary>>, AppError> {
    tracing::info!(component = "collections/handlers", "collection.list");
    let collections = svc.list().await?;
    Ok(Json(collections))
}

/// Get a single collection by ID.
///
/// Endpoint: `GET /api/collections/:id`
pub async fn get(
    State(svc): State<CollectionService>,
    Path(id): Path<Uuid>,
) -> Result<Json<Collection>, AppError> {
    tracing::info!(component = "collections/handlers", collection_id = %id, "collection.get");
    let collection = svc.get(id).await?;
    Ok(Json(collection))
}

/// Delete a collection by ID.
///
/// Endpoint: `DELETE /api/collections/:id`
pub async fn delete(
    State(svc): State<CollectionService>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(component = "collections/handlers", collection_id = %id, "collection.delete");
    svc.delete(id).await?;
    Ok(Json(serde_json::json!({"status": "deleted", "id": id})))
}
