use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use crate::modules::auth::models::UserContext;
use crate::modules::collections::models::{
    ChunkSearchQuery, Collection, CollectionStats, CollectionSummary, CreateCollectionRequest,
};
use crate::modules::collections::service::CollectionService;
use crate::shared::error::AppError;

/// Create a new collection.
///
/// Endpoint: `POST /api/collections`
pub async fn create(
    State(svc): State<CollectionService>,
    user_ctx: UserContext,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<CollectionSummary>, AppError> {
    tracing::info!(component = "collections/handlers", collection_name = %req.name, user_id = %user_ctx.user_id, "collection.create");
    let summary = svc.create(req, &user_ctx.user_id).await?;
    Ok(Json(summary))
}

/// List all collections visible to the current user.
///
/// Endpoint: `GET /api/collections`
pub async fn list(
    State(svc): State<CollectionService>,
    user_ctx: UserContext,
) -> Result<Json<Vec<CollectionSummary>>, AppError> {
    let is_admin = user_ctx.roles.contains(&"admin".to_string());
    tracing::info!(component = "collections/handlers", user_id = %user_ctx.user_id, is_admin = %is_admin, "collection.list");
    let collections = svc.list(&user_ctx.user_id, is_admin).await?;
    Ok(Json(collections))
}

/// Get a single collection by ID.
///
/// Endpoint: `GET /api/collections/:id`
pub async fn get(
    State(svc): State<CollectionService>,
    user_ctx: UserContext,
    Path(id): Path<Uuid>,
) -> Result<Json<Collection>, AppError> {
    let is_admin = user_ctx.roles.contains(&"admin".to_string());
    tracing::info!(component = "collections/handlers", collection_id = %id, user_id = %user_ctx.user_id, "collection.get");
    let collection = svc.get(id, &user_ctx.user_id, is_admin).await?;
    Ok(Json(collection))
}

/// Admin-only: list all collections (no user_id scoping).
///
/// Endpoint: `GET /api/admin/collections`
pub async fn admin_list(
    State(svc): State<CollectionService>,
    user_ctx: UserContext,
) -> Result<Json<Vec<CollectionSummary>>, AppError> {
    tracing::info!(component = "collections/handlers", user_id = %user_ctx.user_id, "admin.collection.list");
    let collections = svc.list(&user_ctx.user_id, true).await?;
    Ok(Json(collections))
}

/// Admin-only: delete any collection by ID (bypasses ownership).
///
/// Endpoint: `DELETE /api/admin/collections/:id`
pub async fn admin_delete(
    State(svc): State<CollectionService>,
    user_ctx: UserContext,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(component = "collections/handlers", collection_id = %id, user_id = %user_ctx.user_id, "admin.collection.delete");
    svc.delete(id, &user_ctx.user_id, true).await?;
    Ok(Json(serde_json::json!({"status": "deleted", "id": id})))
}

/// Delete a collection by ID.
///
/// Endpoint: `DELETE /api/collections/:id`
pub async fn delete(
    State(svc): State<CollectionService>,
    user_ctx: UserContext,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let is_admin = user_ctx.roles.contains(&"admin".to_string());
    tracing::info!(component = "collections/handlers", collection_id = %id, user_id = %user_ctx.user_id, "collection.delete");
    svc.delete(id, &user_ctx.user_id, is_admin).await?;
    Ok(Json(serde_json::json!({"status": "deleted", "id": id})))
}

/// Get collection statistics.
///
/// Endpoint: `GET /api/collections/:id/stats`
pub async fn get_collection_stats(
    State(svc): State<CollectionService>,
    user_ctx: UserContext,
    Path(id): Path<Uuid>,
) -> Result<Json<CollectionStats>, AppError> {
    let is_admin = user_ctx.roles.contains(&"admin".to_string());
    tracing::info!(component = "collections/handlers", collection_id = %id, user_id = %user_ctx.user_id, "collection.stats");
    let stats = svc.get_stats(id, &user_ctx.user_id, is_admin).await?;
    Ok(Json(stats))
}

/// Search chunks within a collection.
///
/// Endpoint: `GET /api/collections/:id/chunks`
pub async fn search_collection_chunks(
    State(svc): State<CollectionService>,
    user_ctx: UserContext,
    Path(id): Path<Uuid>,
    Query(params): Query<ChunkSearchQuery>,
) -> Result<Json<Vec<crate::shared::chunk_search::ChunkSearchResult>>, AppError> {
    let is_admin = user_ctx.roles.contains(&"admin".to_string());
    tracing::info!(
        component = "collections/handlers",
        collection_id = %id,
        search_type = ?params.search_type,
        "collection.chunks_search"
    );
    let results = svc
        .search_chunks(id, &user_ctx.user_id, is_admin, &params)
        .await?;
    Ok(Json(results))
}
