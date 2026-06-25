use axum::{http::Request, middleware::Next, response::Response};

use crate::modules::audit::models::CreateAuditEvent;
use crate::modules::audit::service::AuditService;
use crate::shared::auth::AuthInfo;

/// Axum middleware that records audit events for all API requests.
///
/// This middleware is non-blocking — it never fails the request. Audit events
/// are inserted fire-and-forget via `tokio::spawn` so the response is not
/// delayed by the database write.
///
/// Place this middleware **after** the auth middleware so that `AuthInfo` is
/// available in the request extensions.
pub async fn audit_middleware(req: Request<axum::body::Body>, next: Next) -> Response {
    // Capture request metadata before the handler consumes the request.
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let ip_address = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Extract user info from auth extensions (may be absent for public routes).
    let user_id = req
        .extensions()
        .get::<AuthInfo>()
        .map(|a| a.user.sub.clone())
        .unwrap_or_default();

    // Extract audit service from extensions (may be absent if not registered).
    let audit_svc: Option<AuditService> = req.extensions().get::<AuditService>().cloned();

    // Extract resource type from the path prefix.
    let resource_type = path
        .trim_start_matches('/')
        .split('/')
        .nth(1)
        .unwrap_or("unknown")
        .to_string();

    // Run the inner service (handler).
    let response = next.run(req).await;
    let status = response.status().as_u16();

    // Fire-and-forget audit event recording (non-blocking).
    if let Some(svc) = audit_svc {
        tokio::spawn(async move {
            let action = format!("{} {}", method, path);

            let event = CreateAuditEvent {
                user_id,
                action,
                resource_type,
                resource_id: String::new(),
                details: serde_json::json!({
                    "method": method,
                    "path": path,
                    "status": status,
                }),
                ip_address,
            };

            if let Err(e) = svc.record_event(event).await {
                tracing::warn!(
                    component = "audit/middleware",
                    error = %e,
                    "audit.record_failed"
                );
            }
        });
    }

    response
}
