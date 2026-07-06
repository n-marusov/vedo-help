use std::collections::HashMap;

use axum::{extract::State, Json};
use serde_json::Value;

use crate::modules::settings::models::{ModelsResponse, SettingsResponse};
use crate::modules::settings::service::SettingsService;
use crate::shared::error::AppError;

/// GET /api/admin/models — returns the available LLM, embedding, and rerank model lists.
///
/// The backend is the single source of truth for model definitions.
/// Requires admin role (enforced by RBAC middleware on the admin sub-router).
pub async fn get_models() -> Json<ModelsResponse> {
    tracing::info!(component = "admin/settings", "admin.models.get");
    Json(ModelsResponse::all())
}

/// GET /api/admin/settings — returns all RAG settings as a flat JSON map.
///
/// Requires admin role (enforced by RBAC middleware on the admin sub-router).
pub async fn get_settings(
    State(svc): State<SettingsService>,
) -> Result<Json<SettingsResponse>, AppError> {
    tracing::info!(component = "admin/settings", "admin.settings.get");

    let settings = svc.get_rag_settings().await?;

    tracing::debug!(
        component = "admin/settings",
        setting_count = 10,
        "admin.settings.get.complete"
    );

    Ok(Json(settings.to_map()))
}

/// PUT /api/admin/settings — updates one or more RAG settings.
///
/// Accepts a JSON object with setting keys and their new values.
/// Only recognized keys are persisted; unknown keys are rejected.
/// Returns the full settings state after the update.
///
/// Requires admin role (enforced by RBAC middleware on the admin sub-router).
pub async fn update_settings(
    State(svc): State<SettingsService>,
    Json(updates): Json<HashMap<String, Value>>,
) -> Result<Json<SettingsResponse>, AppError> {
    tracing::info!(
        component = "admin/settings",
        update_keys = updates.len(),
        "admin.settings.update"
    );

    if updates.is_empty() {
        return Err(AppError::BadRequest("No settings provided".to_string()));
    }

    let result = svc.update_settings(updates).await?;

    tracing::info!(
        component = "admin/settings",
        updated_keys = result.len(),
        "admin.settings.update.complete"
    );

    Ok(Json(result))
}
