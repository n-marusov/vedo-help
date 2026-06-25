use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An audit log event recording an administrative or security-relevant action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub user_id: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub details: serde_json::Value,
    pub ip_address: String,
    pub created_at: DateTime<Utc>,
}

/// Input for creating a new audit event.
#[derive(Debug, Clone)]
pub struct CreateAuditEvent {
    pub user_id: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub details: serde_json::Value,
    pub ip_address: String,
}

/// Paginated audit log response.
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogPage {
    pub events: Vec<AuditEvent>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

/// Filters for querying audit log entries.
#[derive(Debug, Clone, Default)]
pub struct AuditLogQuery {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub page: i64,
    pub per_page: i64,
}
