use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde_json::json;
use tokio::sync::Mutex;
use tower_http::limit::RequestBodyLimitLayer;

use crate::shared::auth::AuthUser;

/// Key for rate limiting — identifies a request source.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum RateLimiterKey {
    /// Rate limit by authenticated user ID.
    User(String),
    /// Rate limit by IP address.
    Ip(String),
    /// Global rate limit (shared across all requests).
    Global,
}

impl std::fmt::Display for RateLimiterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimiterKey::User(id) => write!(f, "user:{id}"),
            RateLimiterKey::Ip(ip) => write!(f, "ip:{ip}"),
            RateLimiterKey::Global => write!(f, "global"),
        }
    }
}

/// Result of a rate limit check.
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed.
    pub allowed: bool,
    /// Remaining requests in the current window.
    pub remaining: u64,
    /// Seconds until the rate limit resets.
    pub retry_after_secs: u64,
}

/// Sliding-window rate limiter using per-key timestamp logs.
///
/// Maintains a `VecDeque<Instant>` per key. On each check, timestamps older
/// than the window duration are pruned. If the remaining count is below the
/// limit, the request is allowed and its timestamp is recorded. Otherwise,
/// the request is rejected with `retry_after_secs` computed from the oldest
/// active timestamp.
#[derive(Clone, Debug)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
    max_requests: u64,
    window_secs: u64,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// `max_requests` sets how many requests are allowed within `window_secs`.
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        tracing::info!(
            component = "rate_limit",
            max_requests = max_requests,
            window_secs = window_secs,
            "rate_limiter.configured"
        );
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_secs,
        }
    }

    /// Check whether a request identified by `key` should be allowed.
    ///
    /// Returns a `RateLimitResult` with `allowed`, `remaining`, and
    /// `retry_after_secs`.
    pub async fn check(&self, key: &RateLimiterKey) -> RateLimitResult {
        let key_str = key.to_string();
        let mut state = self.inner.lock().await;
        let now = Instant::now();
        let window_duration = std::time::Duration::from_secs(self.window_secs);

        // Get or create the deque for this key
        let timestamps = state.entry(key_str.clone()).or_insert_with(VecDeque::new);

        // Prune expired timestamps
        let cutoff = now - window_duration;
        while let Some(&oldest) = timestamps.front() {
            if oldest < cutoff {
                timestamps.pop_front();
            } else {
                break;
            }
        }

        let count = timestamps.len() as u64;

        if count >= self.max_requests {
            // Rate limited — compute retry-after based on oldest timestamp
            let oldest = timestamps.front().copied().unwrap_or(now);
            let elapsed = now - oldest;
            let retry_after = window_duration.saturating_sub(elapsed);
            let retry_after_secs = retry_after.as_secs().max(1);

            tracing::warn!(
                component = "rate_limit",
                key = %key_str,
                limit = self.max_requests,
                current = count,
                window_secs = self.window_secs,
                "rate_limit.exceeded"
            );

            RateLimitResult {
                allowed: false,
                remaining: 0,
                retry_after_secs,
            }
        } else {
            // Allow — record this request
            timestamps.push_back(now);
            let remaining = self.max_requests.saturating_sub(count + 1);

            tracing::debug!(
                component = "rate_limit",
                key = %key_str,
                remaining = remaining,
                "rate_limit.approved"
            );

            RateLimitResult {
                allowed: true,
                remaining,
                retry_after_secs: 0,
            }
        }
    }

    /// Extract a rate limit key from the request.
    ///
    /// Checks for an authenticated `AuthUser` in request extensions first,
    /// then falls back to the `X-Forwarded-For` header, then `unknown`.
    pub fn extract_key(req: &Request) -> RateLimiterKey {
        // Check for authenticated user in extensions
        if let Some(auth_user) = req.extensions().get::<AuthUser>() {
            return RateLimiterKey::User(auth_user.sub.clone());
        }

        // Fall back to IP address from headers
        let ip = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        RateLimiterKey::Ip(ip)
    }

    /// Create the 429 rate-limit response body and headers.
    fn build_429_response(retry_after_secs: u64) -> Response {
        let body = json!({
            "error": "rate_limit_exceeded",
            "retry_after_secs": retry_after_secs,
        });

        let mut response = (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response();
        response.headers_mut().insert(
            header::RETRY_AFTER,
            axum::http::HeaderValue::from_str(&retry_after_secs.to_string())
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("1")),
        );
        response
    }
}

/// Axum middleware that checks the rate limiter before passing to the next handler.
///
/// Usage:
/// ```rust,ignore
/// .route_layer(middleware::from_fn(rate_limit_check))
///     .layer(Extension(rate_limiter))
/// ```
///
/// The `RateLimiter` must be added as an `Extension` to the router.
pub async fn rate_limit_check(
    Extension(limiter): Extension<Arc<RateLimiter>>,
    req: Request,
    next: Next,
) -> Response {
    let key = RateLimiter::extract_key(&req);
    let result = limiter.check(&key).await;

    if !result.allowed {
        tracing::warn!(
            component = "rate_limit",
            key = %key,
            retry_after_secs = result.retry_after_secs,
            "rate_limit.rejected"
        );
        return RateLimiter::build_429_response(result.retry_after_secs);
    }

    next.run(req).await
}

/// Per-role rate limiter for the query endpoint.
///
/// Maintains three separate `RateLimiter` instances for guest, user, and admin
/// roles. The middleware extracts the user's role from `AuthUser` in request
/// extensions and checks the appropriate limiter.
#[derive(Clone)]
pub struct QueryRateLimiter {
    /// 5 requests per 60 seconds.
    pub guest: Arc<RateLimiter>,
    /// 20 requests per 60 seconds.
    pub user: Arc<RateLimiter>,
    /// 60 requests per 60 seconds.
    pub admin: Arc<RateLimiter>,
}

/// Default rate limit tiers for the query endpoint.
pub const QUERY_RATE_LIMIT_GUEST: (u64, u64) = (5, 60);
pub const QUERY_RATE_LIMIT_USER: (u64, u64) = (20, 60);
pub const QUERY_RATE_LIMIT_ADMIN: (u64, u64) = (60, 60);

/// Extract the user role from a request's `AuthUser` extension.
pub fn extract_user_role(req: &Request) -> &'static str {
    if let Some(auth_user) = req.extensions().get::<AuthUser>() {
        if auth_user.roles.iter().any(|r| r == "admin") {
            "admin"
        } else {
            "user"
        }
    } else {
        "guest"
    }
}

/// Axum middleware that rate-limits the query route by user role.
///
/// The `QueryRateLimiter` must be added as an `Extension` to the router.
pub async fn rate_limit_query(
    Extension(rl): Extension<Arc<QueryRateLimiter>>,
    req: Request,
    next: Next,
) -> Response {
    let role = extract_user_role(&req);
    let limiter = match role {
        "admin" => &rl.admin,
        "user" => &rl.user,
        _ => &rl.guest,
    };

    let key = RateLimiter::extract_key(&req);
    let result = limiter.check(&key).await;

    if !result.allowed {
        tracing::warn!(
            component = "rate_limit",
            user_role = role,
            key = %key,
            route = "/api/query",
            "rate_limit.query_exceeded"
        );
        return RateLimiter::build_429_response(result.retry_after_secs);
    }

    tracing::debug!(
        component = "rate_limit",
        user_role = role,
        key = %key,
        remaining = result.remaining,
        "rate_limit.query_allowed"
    );

    next.run(req).await
}

/// Create a request body size limit layer (10 MB).
pub fn body_limit_layer() -> RequestBodyLimitLayer {
    tracing::debug!(component = "rate_limit", "body_limit.configured");
    RequestBodyLimitLayer::new(10 * 1024 * 1024) // 10 MB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_accepts_within_limit() {
        let limiter = RateLimiter::new(5, 60);
        let key = RateLimiterKey::User("user-1".to_string());

        for i in 0..5 {
            let result = limiter.check(&key).await;
            assert!(result.allowed, "Request {} should be allowed", i + 1);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_rejects_excess() {
        let limiter = RateLimiter::new(3, 60);
        let key = RateLimiterKey::User("user-2".to_string());

        for i in 0..3 {
            let result = limiter.check(&key).await;
            assert!(result.allowed, "Request {} should be allowed", i + 1);
        }

        let result = limiter.check(&key).await;
        assert!(!result.allowed, "4th request should be rejected");
        assert_eq!(result.remaining, 0);
        assert!(result.retry_after_secs > 0);
    }

    #[tokio::test]
    async fn test_rate_limiter_window_resets() {
        let limiter = RateLimiter::new(1, 1); // 1 request per 1 second
        let key = RateLimiterKey::User("user-3".to_string());

        let r1 = limiter.check(&key).await;
        assert!(r1.allowed, "First request should be allowed");

        let r2 = limiter.check(&key).await;
        assert!(!r2.allowed, "Second request should be rate limited");

        // Wait for window to expire
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let r3 = limiter.check(&key).await;
        assert!(r3.allowed, "After window reset, request should be allowed");
    }

    #[tokio::test]
    async fn test_rate_limiter_per_user_isolation() {
        let limiter = RateLimiter::new(2, 60);
        let user_a = RateLimiterKey::User("user-a".to_string());
        let user_b = RateLimiterKey::User("user-b".to_string());

        // Exhaust user A's limit
        assert!(limiter.check(&user_a).await.allowed);
        assert!(limiter.check(&user_a).await.allowed);
        assert!(
            !limiter.check(&user_a).await.allowed,
            "User A should be blocked"
        );

        // User B should still have available requests
        assert!(limiter.check(&user_b).await.allowed);
        assert!(limiter.check(&user_b).await.allowed);
        assert!(
            !limiter.check(&user_b).await.allowed,
            "User B should also be blocked after 2"
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_ip_isolation() {
        let limiter = RateLimiter::new(1, 60);
        let ip_a = RateLimiterKey::Ip("10.0.0.1".to_string());
        let ip_b = RateLimiterKey::Ip("10.0.0.2".to_string());

        assert!(limiter.check(&ip_a).await.allowed);
        assert!(
            !limiter.check(&ip_a).await.allowed,
            "IP A should be blocked"
        );
        assert!(limiter.check(&ip_b).await.allowed, "IP B should be allowed");
    }

    #[tokio::test]
    async fn test_rate_limiter_key_format() {
        let user_key = RateLimiterKey::User("abc".to_string());
        let ip_key = RateLimiterKey::Ip("1.2.3.4".to_string());
        let global_key = RateLimiterKey::Global;

        assert_eq!(user_key.to_string(), "user:abc");
        assert_eq!(ip_key.to_string(), "ip:1.2.3.4");
        assert_eq!(global_key.to_string(), "global");
    }
}
