use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::modules::conversations::models::{
    CreateSessionRequest, SessionSummary, UpdateMessageRequest, UpdateSessionRequest,
};
use crate::modules::conversations::service::ConversationService;
use crate::shared::error::AppError;

/// Query parameters for export endpoint.
#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
}

/// List all sessions, most recently updated first.
///
/// Endpoint: `GET /api/sessions`
pub async fn list_sessions(
    State(svc): State<ConversationService>,
) -> Result<Json<Vec<SessionSummary>>, AppError> {
    tracing::info!(component = "conversations/handlers", "session.list");
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
    tracing::info!(component = "conversations/handlers", "session.create");
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
    tracing::info!(component = "conversations/handlers", session_id = %id, "session.get");
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
    tracing::info!(component = "conversations/handlers", session_id = %id, "session.delete");
    svc.delete_session(id).await?;
    Ok(Json(serde_json::json!({"status": "deleted", "id": id})))
}

/// Export a session as JSON or Markdown.
///
/// Endpoint: `GET /api/sessions/:id/export?format={json|md|markdown}`
/// Default format is `json` when omitted.
/// Both "md" and "markdown" produce Markdown output.
pub async fn export_session(
    State(svc): State<ConversationService>,
    Path(id): Path<Uuid>,
    Query(query): Query<ExportQuery>,
) -> Result<Response, AppError> {
    let format = query.format.as_deref().unwrap_or("json");

    match format {
        "json" => {
            tracing::info!(component = "conversations/handlers", session_id = %id, format = "json", "session.export");
            let export = svc.export_session(id).await?;
            Ok(Json(export).into_response())
        }
        "md" | "markdown" => {
            tracing::info!(component = "conversations/handlers", session_id = %id, format = %format, "session.export");
            let md = svc.export_session_markdown(id).await?;
            Ok(([(axum::http::header::CONTENT_TYPE, "text/markdown")], md).into_response())
        }
        other => {
            tracing::warn!(component = "conversations/handlers", format = %other, "session.export.unknown_format");
            Err(AppError::UnprocessableEntity(format!(
                "Unknown export format: {other}. Supported: json, md, markdown"
            )))
        }
    }
}

/// Delete all sessions.
///
/// Endpoint: `DELETE /api/sessions`
pub async fn delete_all_sessions(
    State(svc): State<ConversationService>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::warn!(component = "conversations/handlers", "session.delete_all");
    let result = svc.delete_all_sessions().await?;
    Ok(Json(result))
}

/// Update a message's content.
///
/// Endpoint: `PATCH /api/sessions/:session_id/messages/:message_id`
/// Only user messages can be edited. Returns 422 for assistant messages.
pub async fn patch_message(
    State(svc): State<ConversationService>,
    Path((session_id, message_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateMessageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(component = "conversations/handlers", session_id = %session_id, message_id = %message_id, "message.update");
    let updated = svc.update_message(session_id, message_id, req).await?;
    Ok(Json(serde_json::json!(updated)))
}

/// Update a session's title or pinned status.
///
/// Endpoint: `PATCH /api/sessions/:id`
pub async fn patch_session(
    State(svc): State<ConversationService>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateSessionRequest>,
) -> Result<Json<SessionSummary>, AppError> {
    tracing::info!(component = "conversations/handlers", session_id = %id, "session.update");
    let summary = svc.update_session(id, req).await?;
    Ok(Json(summary))
}

/// Soft-delete a message.
///
/// Endpoint: `DELETE /api/sessions/:session_id/messages/:message_id`
/// Returns 204 on success.
pub async fn delete_message(
    State(svc): State<ConversationService>,
    Path((session_id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(component = "conversations/handlers", session_id = %session_id, message_id = %message_id, "message.delete");
    svc.delete_message(session_id, message_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
