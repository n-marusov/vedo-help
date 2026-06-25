use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::audit::models::{AuditEvent, AuditLogPage, AuditLogQuery, CreateAuditEvent};
use crate::shared::error::AppError;

/// Repository for audit log data access.
#[derive(Clone, Debug)]
pub struct AuditRepository {
    db: PgPool,
}

impl AuditRepository {
    /// Create a new AuditRepository with the given database pool.
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Insert a new audit event into the log.
    pub async fn insert_event(&self, event: &CreateAuditEvent) -> Result<(), AppError> {
        let id = Uuid::new_v4();

        tracing::debug!(
            component = "audit/repository",
            user_id = %event.user_id,
            action = %event.action,
            "audit.insert.started"
        );

        sqlx::query(
            "INSERT INTO audit_log (id, user_id, action, resource_type, resource_id, details, ip_address, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
        )
        .bind(id)
        .bind(&event.user_id)
        .bind(&event.action)
        .bind(&event.resource_type)
        .bind(&event.resource_id)
        .bind(&event.details)
        .bind(&event.ip_address)
        .execute(&self.db)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to insert audit event: {e}"))
        })?;

        tracing::info!(
            component = "audit/repository",
            event_id = %id,
            user_id = %event.user_id,
            action = %event.action,
            "audit.inserted"
        );

        Ok(())
    }

    /// Query audit log entries with pagination and optional filters.
    pub async fn query_events(&self, query: &AuditLogQuery) -> Result<AuditLogPage, AppError> {
        tracing::debug!(
            component = "audit/repository",
            page = query.page,
            per_page = query.per_page,
            "audit.query.started"
        );

        // Build WHERE conditions dynamically
        let mut conditions: Vec<String> = Vec::new();
        let mut param_idx = 1;

        if query.user_id.is_some() {
            conditions.push(format!("user_id = ${}", param_idx));
            param_idx += 1;
        }
        if query.action.is_some() {
            conditions.push(format!("action = ${}", param_idx));
            param_idx += 1;
        }
        if query.from.is_some() {
            conditions.push(format!("created_at >= ${}", param_idx));
            param_idx += 1;
        }
        if query.to.is_some() {
            conditions.push(format!("created_at <= ${}", param_idx));
            param_idx += 1;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count total matching rows
        let count_sql = format!("SELECT COUNT(*) FROM audit_log {}", where_clause);
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);

        if let Some(ref uid) = query.user_id {
            count_query = count_query.bind(uid);
        }
        if let Some(ref act) = query.action {
            count_query = count_query.bind(act);
        }
        if let Some(ref from) = query.from {
            count_query = count_query.bind(from);
        }
        if let Some(ref to) = query.to {
            count_query = count_query.bind(to);
        }

        let total: i64 = count_query
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to count audit events: {e}")))?;

        // Fetch page of results
        let offset = (query.page - 1) * query.per_page;
        let data_sql = format!(
            "SELECT id, user_id, action, resource_type, resource_id, details, ip_address, created_at \
             FROM audit_log {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            where_clause,
            param_idx,
            param_idx + 1,
        );

        let mut data_query = sqlx::query_as::<_, AuditEventRow>(&data_sql);

        if let Some(ref uid) = query.user_id {
            data_query = data_query.bind(uid);
        }
        if let Some(ref act) = query.action {
            data_query = data_query.bind(act);
        }
        if let Some(ref from) = query.from {
            data_query = data_query.bind(from);
        }
        if let Some(ref to) = query.to {
            data_query = data_query.bind(to);
        }

        data_query = data_query.bind(query.per_page).bind(offset);

        let rows: Vec<AuditEventRow> = data_query
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to query audit events: {e}")))?;

        let events: Vec<AuditEvent> = rows.into_iter().map(|r| r.into_event()).collect();
        let total_pages = (total as f64 / query.per_page as f64).ceil() as i64;

        tracing::debug!(
            component = "audit/repository",
            count = events.len(),
            total = total,
            "audit.query.complete"
        );

        Ok(AuditLogPage {
            events,
            total,
            page: query.page,
            per_page: query.per_page,
            total_pages,
        })
    }
}

/// Intermediate row struct for sqlx query_as.
#[derive(Debug, sqlx::FromRow)]
struct AuditEventRow {
    id: uuid::Uuid,
    user_id: String,
    action: String,
    resource_type: String,
    resource_id: String,
    details: serde_json::Value,
    ip_address: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl AuditEventRow {
    fn into_event(self) -> AuditEvent {
        AuditEvent {
            id: self.id,
            user_id: self.user_id,
            action: self.action,
            resource_type: self.resource_type,
            resource_id: self.resource_id,
            details: self.details,
            ip_address: self.ip_address,
            created_at: self.created_at,
        }
    }
}
