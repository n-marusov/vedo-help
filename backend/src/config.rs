use std::env;

/// Application configuration loaded from environment variables.
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub chroma_url: String,
    pub llm_api_key: String,
    pub llm_base_url: String,
    pub llm_fallback_base_url: String,
    pub llm_model: String,
    /// RouterAI embedding API key (defaults to llm_api_key)
    pub embedding_api_key: String,
    /// RouterAI embedding API base URL (defaults to llm_base_url)
    pub embedding_base_url: String,
    /// Max entries in local embedding LRU cache
    pub embedding_cache_size: usize,
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
    /// Advanced RAG enabled
    pub advanced_rag_enabled: bool,
    /// Rerank Top K chunks to keep
    pub rerank_top_k: usize,
    /// Hybrid search initial Top K chunks
    pub hybrid_top_k: usize,
    /// Multi-query count (number of query variants to generate)
    pub multi_query_count: usize,
    /// LLM model for reranking
    pub llm_rerank_model: String,
    /// Embedding model for vector search
    pub embedding_model: String,
    /// BM25 k1 parameter (control term frequency saturation, default 1.2)
    pub bm25_k1: f64,
    /// BM25 b parameter (control document length normalization, default 0.75)
    pub bm25_b: f64,
    /// Hybrid search alpha weight (vector search weight, 0.0-1.0).
    /// 0.0 = pure BM25, 1.0 = pure vector search.
    pub hybrid_search_alpha: f64,
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

        let llm_api_key = env::var("LLM_API_KEY").unwrap_or_else(|_| String::new());
        let llm_base_url =
            env::var("LLM_BASE_URL").unwrap_or_else(|_| "https://routerai.ru/api/v1".to_string());
        let llm_fallback_base_url = env::var("LLM_FALLBACK_BASE_URL")
            .unwrap_or_else(|_| "https://opencode.ai/api/v1".to_string());

        Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://vedo:CHANGEME-db-password@localhost:5432/vedo".to_string()
            }),
            chroma_url: env::var("CHROMA_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_string()),
            llm_api_key: llm_api_key.clone(),
            llm_base_url: llm_base_url.clone(),
            llm_fallback_base_url: llm_fallback_base_url.clone(),
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
            advanced_rag_enabled: env::var("ADVANCED_RAG_ENABLED")
                .map(|v| v.to_lowercase() == "true" || v == "1")
                .unwrap_or(true),
            rerank_top_k: env::var("RERANK_TOP_K")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .expect("RERANK_TOP_K must be a valid number"),
            hybrid_top_k: env::var("HYBRID_TOP_K")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .expect("HYBRID_TOP_K must be a valid number"),
            multi_query_count: env::var("MULTI_QUERY_COUNT")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .expect("MULTI_QUERY_COUNT must be a valid number"),
            llm_rerank_model: env::var("LLM_RERANK_MODEL")
                .unwrap_or_else(|_| "anthropic/claude-sonnet-4.6".to_string()), // default to same as LLM_MODEL for now
            embedding_api_key: env::var("EMBEDDING_API_KEY")
                .unwrap_or_else(|_| llm_api_key.clone()),
            embedding_base_url: env::var("EMBEDDING_BASE_URL")
                .unwrap_or_else(|_| llm_base_url.clone()),
            embedding_cache_size: env::var("EMBEDDING_CACHE_SIZE")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .expect("EMBEDDING_CACHE_SIZE must be a valid number"),
            embedding_model: env::var("EMBEDDING_MODEL")
                .unwrap_or_else(|_| "sentence-transformers/all-minilm-l6-v2".to_string()),
            bm25_k1: env::var("BM25_K1")
                .unwrap_or_else(|_| "1.2".to_string())
                .parse()
                .expect("BM25_K1 must be a valid number"),
            bm25_b: env::var("BM25_B")
                .unwrap_or_else(|_| "0.75".to_string())
                .parse()
                .expect("BM25_B must be a valid number"),
            hybrid_search_alpha: env::var("HYBRID_SEARCH_ALPHA")
                .unwrap_or_else(|_| "0.5".to_string())
                .parse()
                .expect("HYBRID_SEARCH_ALPHA must be a valid number between 0.0 and 1.0"),
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
    fn test_embedding_config_defaults() {
        let config = AppConfig::from_env();
        // Embedding config should default to LLM values
        assert_eq!(config.embedding_api_key, config.llm_api_key);
        assert_eq!(config.embedding_base_url, config.llm_base_url);
        assert_eq!(config.embedding_cache_size, 1000);
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
