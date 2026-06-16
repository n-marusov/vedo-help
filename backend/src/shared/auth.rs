use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{
    decode, decode_header,
    jwk::{AlgorithmParameters, JwkSet},
    Algorithm, DecodingKey, Validation,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Mutex;

use crate::config::AppConfig;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// User information extracted from a validated KeyCloak JWT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub sub: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
    #[serde(rename = "provider")]
    pub provider: Option<String>,
}

/// The result of token validation — either the admin API key or a JWT user.
#[derive(Debug, Clone)]
pub enum AuthInfo {
    /// Legacy admin API key was used.
    ApiKey,
    /// A valid JWT was presented; carries the user's claims.
    User(AuthUser),
}

/// Simple auth token wrapper (legacy compatibility).
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub token: String,
}

// ---------------------------------------------------------------------------
// JWT Validator
// ---------------------------------------------------------------------------

/// Validates KeyCloak-issued JWTs by fetching and caching the JWKS endpoint.
pub struct JwtValidator {
    jwks_uri: String,
    issuer: String,
    client_id: String,
    /// Cached JWKS key set.
    jwks: Option<JwkSet>,
    /// When the JWKS was last fetched.
    last_fetch: Instant,
}

impl JwtValidator {
    /// Default TTL for cached JWKS (1 hour).
    const JWKS_TTL: Duration = Duration::from_secs(3600);

    /// Create a new validator from the app configuration.
    pub fn from_config(config: &AppConfig) -> Self {
        let jwks_uri = format!(
            "{}/realms/{}/protocol/openid-connect/certs",
            config.keycloak_url.trim_end_matches('/'),
            config.keycloak_realm,
        );
        let issuer = format!(
            "{}/realms/{}",
            config.keycloak_url.trim_end_matches('/'),
            config.keycloak_realm,
        );

        Self {
            jwks_uri,
            issuer,
            client_id: config.keycloak_client_id.clone(),
            jwks: None,
            last_fetch: Instant::now(),
        }
    }

    /// Wrap the validator in a shared, thread-safe container for use in middleware.
    pub fn shared(config: &AppConfig) -> SharedJwtValidator {
        Arc::new(Mutex::new(Self::from_config(config)))
    }

    /// Validate a JWT bearer token and, on success, return the extracted user claims.
    ///
    /// Returns `None` when the token cannot be validated (expired, bad signature,
    /// wrong issuer/audience, etc.). Logs the reason at WARN level.
    pub async fn validate(&mut self, token: &str) -> Option<AuthUser> {
        // Refresh JWKS cache if stale.
        if self.jwks.is_none() || self.last_fetch.elapsed() >= Self::JWKS_TTL {
            if let Err(e) = self.fetch_jwks().await {
                tracing::warn!("JWKS fetch failed (cached keys may be stale): {e}");
                // If we have no cached keys at all, bail early.
                self.jwks.as_ref()?;
            }
        }

        let jwks = match self.jwks.as_ref() {
            Some(j) => j,
            None => {
                tracing::error!("JWKS cache is empty after fetch attempt");
                return None;
            }
        };

        // Decode the JWT header to determine the key ID (kid).
        let header = match decode_header(token) {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("JWT header decode failed: {e}");
                return None;
            }
        };

        let kid = match header.kid {
            Some(ref k) => k.clone(),
            None => {
                tracing::warn!("JWT header missing 'kid' — cannot resolve signing key");
                return None;
            }
        };

        // Look up the JWK matching the header's kid.
        let jwk = match jwks.find(&kid) {
            Some(k) => k,
            None => {
                tracing::warn!("No JWK found for kid={kid}");
                return None;
            }
        };

        // Extract the public key from the JWK.
        let decoding_key = match &jwk.algorithm {
            AlgorithmParameters::RSA(rsa) => {
                match DecodingKey::from_rsa_components(&rsa.n, &rsa.e) {
                    Ok(k) => k,
                    Err(e) => {
                        tracing::warn!("Failed to construct RSA decoding key: {e}");
                        return None;
                    }
                }
            }
            AlgorithmParameters::EllipticCurve(ec) => {
                match DecodingKey::from_ec_components(&ec.x, &ec.y) {
                    Ok(k) => k,
                    Err(e) => {
                        tracing::warn!("Failed to construct EC decoding key: {e}");
                        return None;
                    }
                }
            }
            other => {
                tracing::warn!("Unsupported JWK algorithm: {other:?}");
                return None;
            }
        };

        // Configure validation parameters.
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.client_id]);
        validation.validate_exp = true;
        // Allow some leeway for clock skew (30 seconds).
        validation.leeway = 30;

        // Decode and validate the token.
        let token_data = match decode::<serde_json::Value>(token, &decoding_key, &validation) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("JWT validation failed: {e}");
                return None;
            }
        };

        let claims = &token_data.claims;

        let auth_user = AuthUser {
            sub: claims
                .get("sub")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            name: claims
                .get("name")
                .and_then(|v| v.as_str())
                .map(String::from),
            email: claims
                .get("email")
                .and_then(|v| v.as_str())
                .map(String::from),
            preferred_username: claims
                .get("preferred_username")
                .and_then(|v| v.as_str())
                .map(String::from),
            provider: claims
                .get("provider")
                .and_then(|v| v.as_str())
                .map(String::from),
        };

        tracing::debug!(
            "JWT validation succeeded — sub={}, provider={:?}",
            auth_user.sub,
            auth_user.provider,
        );

        Some(auth_user)
    }

    /// Fetch the JWKS from the KeyCloak certs endpoint.
    async fn fetch_jwks(&mut self) -> Result<(), String> {
        tracing::debug!("Fetching JWKS from {}", self.jwks_uri);

        let response = reqwest::get(&self.jwks_uri)
            .await
            .map_err(|e| format!("JWKS endpoint unreachable: {e}"))?;

        if !response.status().is_success() {
            tracing::error!("JWKS endpoint returned HTTP {}", response.status());
            return Err(format!("JWKS endpoint returned HTTP {}", response.status()));
        }

        let jwks: JwkSet = response
            .json()
            .await
            .map_err(|e| format!("JWKS parse error: {e}"))?;

        tracing::debug!("JWKS fetched successfully ({} keys)", jwks.keys.len());
        self.jwks = Some(jwks);
        self.last_fetch = Instant::now();

        Ok(())
    }
}

/// Thread-safe wrapper for shared JWT validator access.
pub type SharedJwtValidator = Arc<Mutex<JwtValidator>>;

// ---------------------------------------------------------------------------
// AuthToken — legacy helper
// ---------------------------------------------------------------------------

#[allow(clippy::result_large_err)]
impl AuthToken {
    /// Validate the Authorization header against the configured API key.
    ///
    /// This is synchronous and only checks the legacy API key. Returns `Self`
    /// on success or a 401 `Response` on failure.
    pub fn check(headers: &HeaderMap, config: &AppConfig) -> Result<Self, Response> {
        let remote_addr = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|token| {
                if token == config.admin_api_key {
                    Ok(AuthToken {
                        token: token.to_string(),
                    })
                } else {
                    tracing::warn!("Unauthorized request from {remote_addr}: invalid API key");
                    Err(auth_failure_response())
                }
            })
            .unwrap_or_else(|| {
                tracing::warn!(
                    "Unauthorized request from {remote_addr}: missing or malformed auth header"
                );
                Err(auth_failure_response())
            })
    }

    /// Synchronous check — admin API key only.
    /// Returns `true` if the request carries a valid `Bearer <admin_api_key>`.
    pub fn is_authenticated(headers: &HeaderMap, config: &AppConfig) -> bool {
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|token| token == config.admin_api_key)
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// Combined async authentication for middleware
// ---------------------------------------------------------------------------

/// Validate an HTTP request using both the legacy API key and JWT methods.
///
/// 1. Try legacy API key (synchronous fast path).
/// 2. Fall through to JWT validation via the shared `JwtValidator`.
///
/// Returns `AuthInfo` on success or a 401 response on failure.
pub async fn authenticate_request(
    headers: &HeaderMap,
    config: &AppConfig,
    jwt_validator: Option<&SharedJwtValidator>,
) -> Result<AuthInfo, Response> {
    let remote_addr = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let token = match headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        Some(t) => t,
        None => {
            tracing::warn!(
                "Unauthorized request from {remote_addr}: missing or malformed auth header"
            );
            return Err(auth_failure_response());
        }
    };

    // 1. Try legacy API key validation (fast path).
    if token == config.admin_api_key {
        tracing::info!("Auth: admin API key — {remote_addr}");
        return Ok(AuthInfo::ApiKey);
    }

    // 2. Try JWT validation when a validator is available.
    if let Some(shared) = jwt_validator {
        let mut validator = shared.lock().await;
        if let Some(user) = validator.validate(token).await {
            tracing::info!("Auth: JWT — sub={}, provider={:?}", user.sub, user.provider);
            return Ok(AuthInfo::User(user));
        }

        tracing::warn!("Unauthorized request from {remote_addr}: JWT validation failed");
        return Err(auth_failure_response());
    }

    // 3. No validator available — token did not match the API key.
    tracing::warn!(
        "Unauthorized request from {remote_addr}: invalid API key (JWT validator not configured)"
    );
    Err(auth_failure_response())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a standardized auth failure JSON response.
fn auth_failure_response() -> Response {
    let body = json!({
        "error": {
            "type": "unauthorized",
            "message": "Invalid or missing API key"
        }
    });

    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}
