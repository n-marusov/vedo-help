use std::collections::HashMap;

use serde_json::Value;

use crate::modules::settings::models::{RagSettings, SettingsResponse};
use crate::modules::settings::repository::SettingsRepository;
use crate::shared::error::AppError;

/// Service for managing application settings.
///
/// Orchestrates loading settings from the database, merging with
/// environment-based defaults, and validating updates.
#[derive(Clone, Debug)]
pub struct SettingsService {
    repo: SettingsRepository,
    env_config: crate::config::AppConfig,
}

impl SettingsService {
    /// Create a new SettingsService.
    pub fn new(repo: SettingsRepository, env_config: crate::config::AppConfig) -> Self {
        tracing::info!(
            component = "settings/service",
            has_env_fallback = true,
            "settings.service.initialized"
        );
        Self { repo, env_config }
    }

    /// Load all RAG settings by merging database overrides with env-var defaults.
    ///
    /// Database values take precedence when present. If a key is not in the
    /// database, the env-var default (from AppConfig) is used.
    pub async fn get_rag_settings(&self) -> Result<RagSettings, AppError> {
        tracing::debug!(
            component = "settings/service",
            "settings.get_rag_settings.started"
        );

        let db_overrides = self.repo.get_all().await?;

        // Start with defaults, then apply env overrides, then DB overrides
        let mut settings = RagSettings::default().with_env_overrides(&self.env_config);

        // Apply DB overrides on top of env defaults
        if let Some(v) = db_overrides.get("advanced_rag_enabled") {
            if let Some(b) = v.value.as_bool() {
                settings.advanced_rag_enabled = b;
            }
        }
        if let Some(v) = db_overrides.get("multi_query_enabled") {
            if let Some(b) = v.value.as_bool() {
                settings.multi_query_enabled = b;
            }
        }
        if let Some(v) = db_overrides.get("hyde_enabled") {
            if let Some(b) = v.value.as_bool() {
                settings.hyde_enabled = b;
            }
        }
        if let Some(v) = db_overrides.get("bm25_enabled") {
            if let Some(b) = v.value.as_bool() {
                settings.bm25_enabled = b;
            }
        }
        if let Some(v) = db_overrides.get("reranking_enabled") {
            if let Some(b) = v.value.as_bool() {
                settings.reranking_enabled = b;
            }
        }
        if let Some(v) = db_overrides.get("chunk_method") {
            if let Some(s) = v.value.as_str() {
                if let Ok(m) = s.parse() {
                    settings.chunk_method = m;
                }
            }
        }
        if let Some(v) = db_overrides.get("chunk_size") {
            if let Some(n) = v.value.as_u64() {
                settings.chunk_size = n as usize;
            }
        }
        if let Some(v) = db_overrides.get("chunk_overlap") {
            if let Some(n) = v.value.as_u64() {
                settings.chunk_overlap = n as usize;
            }
        }
        if let Some(v) = db_overrides.get("hybrid_top_k") {
            if let Some(n) = v.value.as_u64() {
                settings.hybrid_top_k = n as usize;
            }
        }
        if let Some(v) = db_overrides.get("rerank_top_k") {
            if let Some(n) = v.value.as_u64() {
                settings.rerank_top_k = n as usize;
            }
        }
        if let Some(v) = db_overrides.get("multi_query_count") {
            if let Some(n) = v.value.as_u64() {
                settings.multi_query_count = n as usize;
            }
        }
        if let Some(v) = db_overrides.get("llm_model") {
            if let Some(s) = v.value.as_str() {
                settings.llm_model = s.to_string();
            }
        }
        if let Some(v) = db_overrides.get("llm_rerank_model") {
            if let Some(s) = v.value.as_str() {
                settings.llm_rerank_model = s.to_string();
            }
        }
        if let Some(v) = db_overrides.get("embedding_model") {
            if let Some(s) = v.value.as_str() {
                settings.embedding_model = s.to_string();
            }
        }
        if let Some(v) = db_overrides.get("llm_max_history_messages") {
            if let Some(n) = v.value.as_u64() {
                settings.llm_max_history_messages = n as usize;
            }
        }
        if let Some(v) = db_overrides.get("llm_context_token_budget") {
            if let Some(n) = v.value.as_u64() {
                settings.llm_context_token_budget = n as usize;
            }
        }

        tracing::debug!(
            component = "settings/service",
            db_override_count = db_overrides.len(),
            effective_method = %settings.chunk_method,
            "settings.get_rag_settings.complete"
        );

        Ok(settings)
    }

    /// Update settings from a raw JSON map. Validates types and returns
    /// the full merged settings after the update.
    pub async fn update_settings(
        &self,
        updates: HashMap<String, Value>,
    ) -> Result<SettingsResponse, AppError> {
        tracing::info!(
            component = "settings/service",
            update_keys = updates.len(),
            "settings.update.started"
        );

        // Validate the incoming values before writing
        let mut db_updates = HashMap::new();
        for (key, value) in &updates {
            validate_setting_value(key, value)
                .map_err(|msg| AppError::BadRequest(format!("Invalid value for '{key}': {msg}")))?;
            db_updates.insert(key.clone(), value.clone());
        }

        // Persist to database
        self.repo.upsert_batch(&db_updates).await?;

        // Return the full merged state after update
        let updated = self.get_rag_settings().await?;

        tracing::info!(
            component = "settings/service",
            updated_keys = updates.len(),
            "settings.update.complete"
        );

        Ok(updated.to_map())
    }
}

/// Validate that a setting value has the correct JSON type for its key.
fn validate_setting_value(key: &str, value: &Value) -> Result<(), String> {
    match key {
        "advanced_rag_enabled" => {
            if !value.is_boolean() {
                return Err("expected boolean".to_string());
            }
        }
        "multi_query_enabled" | "hyde_enabled" | "bm25_enabled" | "reranking_enabled" => {
            if !value.is_boolean() {
                return Err("expected boolean".to_string());
            }
        }
        "chunk_method" => {
            let s = value
                .as_str()
                .ok_or_else(|| "expected string".to_string())?;
            s.parse::<crate::modules::settings::models::ChunkMethod>()?;
        }
        "chunk_size"
        | "chunk_overlap"
        | "hybrid_top_k"
        | "rerank_top_k"
        | "multi_query_count"
        | "llm_max_history_messages"
        | "llm_context_token_budget" => {
            if !value.is_number() {
                return Err("expected number".to_string());
            }
            if let Some(n) = value.as_u64() {
                if n > 1_000_000 {
                    return Err("value too large".to_string());
                }
            }
        }
        "llm_model" | "llm_rerank_model" | "embedding_model" => {
            if !value.is_string() {
                return Err("expected string".to_string());
            }
            if value.as_str().unwrap_or("").is_empty() {
                return Err("must not be empty".to_string());
            }
        }
        _ => {
            // Unknown keys are silently ignored (not persisted)
            return Err(format!("unknown setting key: {key}"));
        }
    }
    Ok(())
}
