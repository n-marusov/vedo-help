use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::modules::conversations::models::{CreateSessionRequest, SessionSummary};
use crate::modules::conversations::service::ConversationService;
use crate::shared::error::AppError;

/// List all sessions, most recently updated first.
///
/// Endpoint: `GET /api/sessions`
pub async fn list_sessions(
    State(svc): State<ConversationService>,
) -> Result<Json<Vec<SessionSummary>>, AppError> {
    tracing::info!("GET /api/sessions");
    let sessions = svc.list_sessions().await?;
    Ok(Json(sessions))
}

/// Create a new session.
///
/// Endpoint: `POST /api/sessions`
pub async fn create_session(
    State(svc): State<ConversationService>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionSummary>, AppError> {
    tracing::info!("POST /api/sessions");
    let summary = svc.create_session(req).await?;
    Ok(Json(summary))
}

/// Get a session with its message history.
///
/// Endpoint: `GET /api/sessions/:id`
pub async fn get_session(
    State(svc): State<ConversationService>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("GET /api/sessions/{id}");
    let (session, messages) = svc.get_session_history(id).await?;
    Ok(Json(serde_json::json!({
        "session": session,
        "messages": messages,
    })))
}

/// Delete a session by ID.
///
/// Endpoint: `DELETE /api/sessions/:id`
pub async fn delete_session(
    State(svc): State<ConversationService>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("DELETE /api/sessions/{id}");
    svc.delete_session(id).await?;
    Ok(Json(serde_json::json!({"status": "deleted", "id": id})))
}

/// Export a session as JSON.
///
/// Endpoint: `GET /api/sessions/:id/export`
pub async fn export_session(
    State(svc): State<ConversationService>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("GET /api/sessions/{id}/export");
    let export = svc.export_session(id).await?;
    Ok(Json(export))
}

/// Delete all sessions.
///
/// Endpoint: `DELETE /api/sessions`
pub async fn delete_all_sessions(
    State(svc): State<ConversationService>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::warn!("DELETE /api/sessions (bulk)");
    let result = svc.delete_all_sessions().await?;
    Ok(Json(result))
}
