use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::modules::audit::models::AuditLogPage;
use crate::modules::audit::service::AuditService;
use crate::modules::auth::models::UserContext;
use crate::shared::error::AppError;

/// Query parameters for the audit log endpoint.
#[derive(Debug, Deserialize, Default)]
pub struct AuditLogParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// GET /api/admin/audit-log — returns paginated audit events.
///
/// Requires admin role (enforced by RBAC middleware on the admin sub-router).
pub async fn list_audit_log(
    State(svc): State<AuditService>,
    user_ctx: UserContext,
    Query(params): Query<AuditLogParams>,
) -> Result<Json<AuditLogPage>, AppError> {
    tracing::info!(
        component = "admin/audit",
        user_id = %user_ctx.user_id,
        page = ?params.page,
        per_page = ?params.per_page,
        "admin.audit_log.list"
    );

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);

    // Parse optional date filters
    let from = match params.from.as_deref() {
        Some(s) if !s.is_empty() => Some(
            DateTime::parse_from_rfc3339(s)
                .map_err(|e| AppError::BadRequest(format!("Invalid 'from' date format: {e}")))?
                .with_timezone(&Utc),
        ),
        _ => None,
    };

    let to = match params.to.as_deref() {
        Some(s) if !s.is_empty() => Some(
            DateTime::parse_from_rfc3339(s)
                .map_err(|e| AppError::BadRequest(format!("Invalid 'to' date format: {e}")))?
                .with_timezone(&Utc),
        ),
        _ => None,
    };

    let query = crate::modules::audit::models::AuditLogQuery {
        user_id: params.user_id,
        action: params.action,
        from,
        to,
        page,
        per_page,
    };

    let result = svc.query(&query).await?;

    tracing::debug!(
        component = "admin/audit",
        total = result.total,
        page = result.page,
        "admin.audit_log.list.return"
    );

    Ok(Json(result))
}
