use crate::modules::audit::models::{AuditLogPage, AuditLogQuery, CreateAuditEvent};
use crate::modules::audit::repository::AuditRepository;
use crate::shared::error::AppError;

/// Service for audit log operations.
#[derive(Clone, Debug)]
pub struct AuditService {
    repo: AuditRepository,
}

impl AuditService {
    /// Create a new AuditService.
    pub fn new(repo: AuditRepository) -> Self {
        Self { repo }
    }

    /// Record a new audit event.
    pub async fn record_event(&self, event: CreateAuditEvent) -> Result<(), AppError> {
        tracing::debug!(
            component = "audit/service",
            user_id = %event.user_id,
            action = %event.action,
            "audit.record"
        );

        self.repo.insert_event(&event).await?;

        tracing::info!(
            component = "audit/service",
            user_id = %event.user_id,
            action = %event.action,
            "audit.recorded"
        );

        Ok(())
    }

    /// Query audit log entries with pagination and filtering.
    pub async fn query(&self, query: &AuditLogQuery) -> Result<AuditLogPage, AppError> {
        tracing::debug!(
            component = "audit/service",
            page = query.page,
            per_page = query.per_page,
            "audit.query"
        );

        self.repo.query_events(query).await
    }
}
