use std::env;

/// Application configuration loaded from environment variables.
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub admin_api_key: String,
    pub database_url: String,
    pub embedding_service_url: String,
    pub chroma_url: String,
    pub openrouter_api_key: String,
    pub openrouter_model: String,
    pub host: String,
    pub port: u16,
    pub rust_log: String,
    pub frontend_url: String,
}

impl AppConfig {
    /// Load configuration from environment variables with sensible defaults.
    pub fn from_env() -> Self {
        Self {
            admin_api_key: env::var("ADMIN_API_KEY").unwrap_or_else(|_| "change-me".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/vedo.db?mode=rwc".to_string()),
            embedding_service_url: env::var("EMBEDDING_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8001".to_string()),
            chroma_url: env::var("CHROMA_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_string()),
            openrouter_api_key: env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| String::new()),
            openrouter_model: env::var("OPENROUTER_MODEL")
                .unwrap_or_else(|_| "anthropic/claude-sonnet-20241022".to_string()),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a valid number"),
            rust_log: env::var("RUST_LOG")
                .unwrap_or_else(|_| "vedo_backend=debug,tower_http=debug".to_string()),
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
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
        assert_eq!(config.admin_api_key, "change-me");
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert!(!config.openrouter_model.is_empty());
    }
}
