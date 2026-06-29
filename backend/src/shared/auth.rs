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
    /// Realm roles extracted from the JWT `realm_access.roles` claim.
    #[serde(default)]
    pub roles: Vec<String>,
}

/// The result of JWT token validation — carries the authenticated user's claims.
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub user: AuthUser,
}

// ---------------------------------------------------------------------------
// JWT Validator
// ---------------------------------------------------------------------------

/// Validates KeyCloak-issued JWTs by fetching and caching the JWKS endpoint.
///
/// Audience validation is intentionally omitted — the token is issued by the
/// `vedo-frontend` KeyCloak client (public client, PKCE flow) while the
/// backend uses `vedo-backend` as its client ID. Issuer + signature checks
/// provide sufficient security.
pub struct JwtValidator {
    jwks_uri: String,
    issuer: String,
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
            config.keycloak_jwks_url.trim_end_matches('/'),
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
                tracing::warn!(component = "auth", error = %e, "jwks.fetch_failed");
                // If we have no cached keys at all, bail early.
                self.jwks.as_ref()?;
            }
        }

        let jwks = match self.jwks.as_ref() {
            Some(j) => j,
            None => {
                tracing::error!(component = "auth", "jwks.cache_empty");
                return None;
            }
        };

        // Decode the JWT header to determine the key ID (kid).
        let header = match decode_header(token) {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!(component = "auth", error = %e, "jwt.header_decode_failed");
                return None;
            }
        };

        let kid = match header.kid {
            Some(ref k) => k.clone(),
            None => {
                tracing::warn!(component = "auth", "jwt.header_missing_kid");
                return None;
            }
        };

        // Look up the JWK matching the header's kid.
        let jwk = match jwks.find(&kid) {
            Some(k) => k,
            None => {
                tracing::warn!(component = "auth", kid = %kid, "jwk.not_found");
                return None;
            }
        };

        // Extract the public key from the JWK.
        let decoding_key = match &jwk.algorithm {
            AlgorithmParameters::RSA(rsa) => {
                match DecodingKey::from_rsa_components(&rsa.n, &rsa.e) {
                    Ok(k) => k,
                    Err(e) => {
                        tracing::warn!(component = "auth", error = %e, "jwk.rsa_key_construction_failed");
                        return None;
                    }
                }
            }
            AlgorithmParameters::EllipticCurve(ec) => {
                match DecodingKey::from_ec_components(&ec.x, &ec.y) {
                    Ok(k) => k,
                    Err(e) => {
                        tracing::warn!(component = "auth", error = %e, "jwk.ec_key_construction_failed");
                        return None;
                    }
                }
            }
            other => {
                tracing::warn!(component = "auth", algorithm = %format!("{:?}", other), "jwk.unsupported_algorithm");
                return None;
            }
        };

        // Configure validation parameters.
        // NOTE: Audience validation is intentionally omitted because the token
        // is issued by the `vedo-frontend` KeyCloak client (public, PKCE flow)
        // while the backend uses `vedo-backend` as its client ID. Validating the
        // audience against `vedo-backend` would reject all frontend-issued tokens.
        // The issuer + signature checks provide sufficient security.
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.validate_aud = false;
        validation.validate_exp = true;
        // Allow some leeway for clock skew (30 seconds).
        validation.leeway = 30;

        // Decode and validate the token.
        let token_data = match decode::<serde_json::Value>(token, &decoding_key, &validation) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(component = "auth", error = %e, "jwt.validation_failed");
                return None;
            }
        };

        let claims = &token_data.claims;

        let roles = claims
            .get("realm_access")
            .and_then(|ra| ra.get("roles"))
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        tracing::debug!(
            component = "auth",
            roles = %roles.join(","),
            "jwt.roles_extracted"
        );

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
            roles,
        };

        tracing::debug!(
            component = "auth",
            user_id = %auth_user.sub,
            provider = %auth_user.provider.as_deref().unwrap_or("none"),
            "jwt.validation_succeeded"
        );

        Some(auth_user)
    }

    /// Fetch the JWKS from the KeyCloak certs endpoint.
    async fn fetch_jwks(&mut self) -> Result<(), String> {
        tracing::debug!(component = "auth", jwks_uri = %self.jwks_uri, "jwks.fetching");

        let response = reqwest::get(&self.jwks_uri)
            .await
            .map_err(|e| format!("JWKS endpoint unreachable: {e}"))?;

        if !response.status().is_success() {
            tracing::error!(component = "auth", status = %response.status(), "jwks.endpoint_http_error");
            return Err(format!("JWKS endpoint returned HTTP {}", response.status()));
        }

        let jwks: JwkSet = response
            .json()
            .await
            .map_err(|e| format!("JWKS parse error: {e}"))?;

        tracing::debug!(
            component = "auth",
            key_count = jwks.keys.len(),
            "jwks.fetched"
        );
        self.jwks = Some(jwks);
        self.last_fetch = Instant::now();

        Ok(())
    }
}

/// Thread-safe wrapper for shared JWT validator access.
pub type SharedJwtValidator = Arc<Mutex<JwtValidator>>;

// ---------------------------------------------------------------------------
// JWT-only authentication for middleware
// ---------------------------------------------------------------------------

/// Validate an HTTP request using JWT validation.
///
/// Extracts the Bearer token from the Authorization header and validates it
/// against the KeyCloak JWKS endpoint. Returns the authenticated user's info
/// or a 401 response on failure.
pub async fn authenticate_request(
    headers: &HeaderMap,
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
                component = "auth",
                remote_addr = %remote_addr,
                "auth.missing_or_malformed_header"
            );
            return Err(auth_failure_response());
        }
    };

    // Validate via JWT when a validator is available.
    if let Some(shared) = jwt_validator {
        let mut validator = shared.lock().await;
        if let Some(user) = validator.validate(token).await {
            tracing::info!(
                component = "auth",
                user_id = %user.sub,
                provider = %user.provider.as_deref().unwrap_or("none"),
                roles = %user.roles.join(","),
                "auth.jwt_authenticated"
            );
            return Ok(AuthInfo { user });
        }

        tracing::warn!(component = "auth", remote_addr = %remote_addr, "auth.jwt_validation_failed");
        return Err(auth_failure_response());
    }

    // No validator available — cannot validate the token.
    tracing::warn!(component = "auth", remote_addr = %remote_addr, "auth.no_jwt_validator");
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
            "message": "Invalid or missing token"
        }
    });

    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn jwt_validator_uses_separate_issuer_and_jwks_urls() {
        let config = AppConfig {
            database_url: ":memory:".to_string(),
            embedding_service_url: "http://localhost:18001".to_string(),
            chroma_url: "http://localhost:18000".to_string(),
            llm_api_key: "test".to_string(),
            llm_base_url: "http://llm-mock:18002".to_string(),
            llm_model: "test-model".to_string(),
            host: "127.0.0.1".to_string(),
            port: 0,
            rust_log: "off".to_string(),
            frontend_url: "http://localhost:5173".to_string(),
            keycloak_url: "http://localhost:8080".to_string(),
            keycloak_jwks_url: "http://keycloak:8080".to_string(),
            keycloak_realm: "vedo-hub".to_string(),
            keycloak_client_id: "vedo-backend".to_string(),
            git_clone_root: "/tmp/test-git-repos".to_string(),
            git_sync_interval_secs: 0,
            llm_max_history_messages: 20,
            llm_context_token_budget: 6000,
            otel_endpoint: "http://otel-collector:4317".to_string(),
            service_name: "vedo-backend-test".to_string(),
            environment: "test".to_string(),
            advanced_rag_enabled: true,
            rerank_top_k: 5,
            hybrid_top_k: 3,
            multi_query_count: 3,
            llm_rerank_model: "test-model".to_string(),
        };

        let validator = JwtValidator::from_config(&config);

        assert_eq!(validator.issuer, "http://localhost:8080/realms/vedo-hub");
        assert_eq!(
            validator.jwks_uri,
            "http://keycloak:8080/realms/vedo-hub/protocol/openid-connect/certs"
        );
    }
}
