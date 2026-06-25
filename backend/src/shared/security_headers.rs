use axum::{
    extract::State,
    http::{
        header::{HeaderName, HeaderValue},
        Request,
    },
    middleware::Next,
    response::Response,
};

/// Axum middleware that adds security headers to all responses.
///
/// Use with `middleware::from_fn_with_state(environment.to_string(), security_headers::middleware)`.
/// HSTS (`Strict-Transport-Security`) is only added when the state is `"production"`.
pub async fn middleware(
    State(environment): State<String>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;

    // X-Content-Type-Options: nosniff — prevents MIME type sniffing
    response.headers_mut().insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );

    // X-Frame-Options: DENY — prevents clickjacking
    response.headers_mut().insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );

    // Referrer-Policy: strict-origin-when-cross-origin
    response.headers_mut().insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Permissions-Policy: restrict sensitive APIs
    response.headers_mut().insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );

    // Content-Security-Policy: restrict script/style sources
    response.headers_mut().insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'",
        ),
    );

    // Strict-Transport-Security: production only
    if environment == "production" {
        response.headers_mut().insert(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    response
}
