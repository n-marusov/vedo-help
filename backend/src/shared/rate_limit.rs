use tower_http::limit::RequestBodyLimitLayer;

/// Create a request body size limit layer (10 MB).
///
/// Placeholder for future rate-limiting middleware.
/// Extend with `tower` middleware once a concrete rate-limit crate is chosen.
pub fn body_limit_layer() -> RequestBodyLimitLayer {
    tracing::debug!(component = "rate_limit", "body_limit.configured");
    RequestBodyLimitLayer::new(10 * 1024 * 1024) // 10 MB
}
