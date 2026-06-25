use std::env;

/// Application configuration loaded from environment variables.
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub embedding_service_url: String,
    pub chroma_url: String,
    pub llm_api_key: String,
    pub llm_base_url: String,
    pub llm_model: String,
    pub host: String,
    pub port: u16,
    pub rust_log: String,
    pub frontend_url: String,
    /// Public KeyCloak issuer base URL used in JWT `iss` validation.
    /// This must match the URL seen by the browser when tokens are issued.
    pub keycloak_url: String,
    /// Internal KeyCloak base URL used by the backend to fetch JWKS.
    /// In Docker this is usually http://keycloak:8080 while the issuer remains localhost.
    pub keycloak_jwks_url: String,
    /// KeyCloak realm name
    pub keycloak_realm: String,
    /// Backend client ID retained for KeyCloak client configuration.
    pub keycloak_client_id: String,
    /// Root directory for cloned git repositories
    pub git_clone_root: String,
    /// Git sync polling interval in seconds (0 = disabled)
    pub git_sync_interval_secs: u64,
    /// OpenTelemetry OTLP endpoint (gRPC)
    pub otel_endpoint: String,
    /// Service name for OTel resource attributes
    pub service_name: String,
    /// Deployment environment (development, staging, production)
    pub environment: String,
    /// Max history messages to include in LLM context
    pub llm_max_history_messages: usize,
    /// Token budget for LLM context window
    pub llm_context_token_budget: usize,
}

impl AppConfig {
    /// Load configuration from environment variables with sensible defaults.
    pub fn from_env() -> Self {
        let keycloak_url = env::var("KEYCLOAK_PUBLIC_URL")
            .or_else(|_| env::var("KEYCLOAK_URL"))
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        let keycloak_jwks_url = env::var("KEYCLOAK_JWKS_URL")
            .or_else(|_| env::var("KEYCLOAK_INTERNAL_URL"))
            .unwrap_or_else(|_| keycloak_url.clone());

        Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://vedo:CHANGEME-db-password@localhost:5432/vedo".to_string()
            }),
            embedding_service_url: env::var("EMBEDDING_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8001".to_string()),
            chroma_url: env::var("CHROMA_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_string()),
            llm_api_key: env::var("LLM_API_KEY").unwrap_or_else(|_| String::new()),
            llm_base_url: env::var("LLM_BASE_URL")
                .unwrap_or_else(|_| "https://routerai.ru/api/v1".to_string()),
            llm_model: env::var("LLM_MODEL")
                .unwrap_or_else(|_| "anthropic/claude-sonnet-4.6".to_string()),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a valid number"),
            rust_log: env::var("RUST_LOG")
                .unwrap_or_else(|_| "vedo_backend=debug,tower_http=debug".to_string()),
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
            keycloak_url,
            keycloak_jwks_url,
            keycloak_realm: env::var("KEYCLOAK_REALM").unwrap_or_else(|_| "vedo-hub".to_string()),
            keycloak_client_id: env::var("KEYCLOAK_CLIENT_ID")
                .unwrap_or_else(|_| "vedo-backend".to_string()),
            git_clone_root: env::var("GIT_CLONE_ROOT")
                .unwrap_or_else(|_| "data/git-repos".to_string()),
            git_sync_interval_secs: env::var("GIT_SYNC_INTERVAL_SECS")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .expect("GIT_SYNC_INTERVAL_SECS must be a valid number"),
            llm_max_history_messages: env::var("LLM_MAX_HISTORY_MESSAGES")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .expect("LLM_MAX_HISTORY_MESSAGES must be a valid number"),
            llm_context_token_budget: env::var("LLM_CONTEXT_TOKEN_BUDGET")
                .unwrap_or_else(|_| "6000".to_string())
                .parse()
                .expect("LLM_CONTEXT_TOKEN_BUDGET must be a valid number"),
            otel_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://otel-collector:4317".to_string()),
            service_name: env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| String::new()),
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = AppConfig::from_env();
        // Default values should be set when env vars are absent
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert!(!config.llm_model.is_empty());
    }

    #[test]
    fn test_otel_config_defaults() {
        let config = AppConfig::from_env();
        assert!(
            config.otel_endpoint.contains("otel-collector"),
            "OTEL endpoint should default to collector service"
        );
        assert_eq!(config.service_name, "");
        assert_eq!(config.environment, "development");
    }
}
