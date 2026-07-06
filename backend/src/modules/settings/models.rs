use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single setting entry from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

/// All RAG-related settings with typed values and env-var defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSettings {
    pub advanced_rag_enabled: bool,
    pub multi_query_enabled: bool,
    pub hyde_enabled: bool,
    pub bm25_enabled: bool,
    pub reranking_enabled: bool,
    pub chunk_method: ChunkMethod,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub hybrid_top_k: usize,
    pub rerank_top_k: usize,
    pub multi_query_count: usize,
    /// Main inference LLM model for chat (e.g. anthropic/claude-sonnet-4.6)
    pub llm_model: String,
    /// LLM model for reranking
    pub llm_rerank_model: String,
    /// Embedding model for vector search (e.g. sentence-transformers/all-minilm-l6-v2)
    pub embedding_model: String,
    pub llm_max_history_messages: usize,
    pub llm_context_token_budget: usize,
}

impl Default for RagSettings {
    fn default() -> Self {
        Self {
            advanced_rag_enabled: true,
            multi_query_enabled: true,
            hyde_enabled: true,
            bm25_enabled: true,
            reranking_enabled: true,
            chunk_method: ChunkMethod::Paragraph,
            chunk_size: 1000,
            chunk_overlap: 200,
            hybrid_top_k: 20,
            rerank_top_k: 5,
            multi_query_count: 3,
            llm_model: "anthropic/claude-sonnet-4.6".to_string(),
            llm_rerank_model: "anthropic/claude-sonnet-4.6".to_string(),
            embedding_model: "sentence-transformers/all-minilm-l6-v2".to_string(),
            llm_max_history_messages: 20,
            llm_context_token_budget: 6000,
        }
    }
}

/// Supported chunking strategies.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkMethod {
    /// Paragraph-aware splitting on double newlines.
    #[default]
    Paragraph,
    /// Fixed-size character-based split with overlap.
    Fixed,
}

impl std::fmt::Display for ChunkMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paragraph => write!(f, "paragraph"),
            Self::Fixed => write!(f, "fixed"),
        }
    }
}

impl std::str::FromStr for ChunkMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "paragraph" => Ok(Self::Paragraph),
            "fixed" => Ok(Self::Fixed),
            other => Err(format!(
                "Unknown chunk method: {other} (expected 'paragraph' or 'fixed')"
            )),
        }
    }
}

/// Response type for GET /api/admin/settings.
pub type SettingsResponse = HashMap<String, Value>;

impl RagSettings {
    /// Merge with env-var-based overrides: env values take precedence.
    pub fn with_env_overrides(mut self, env: &crate::config::AppConfig) -> Self {
        self.advanced_rag_enabled = env.advanced_rag_enabled;
        self.hybrid_top_k = env.hybrid_top_k;
        self.rerank_top_k = env.rerank_top_k;
        self.multi_query_count = env.multi_query_count;
        self.llm_model = env.llm_model.clone();
        self.llm_rerank_model = env.llm_rerank_model.clone();
        self.embedding_model = env.embedding_model.clone();
        self.llm_max_history_messages = env.llm_max_history_messages;
        self.llm_context_token_budget = env.llm_context_token_budget;
        self
    }

    /// Serialize to a flat JSON map for the API response.
    pub fn to_map(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert(
            "advanced_rag_enabled".to_string(),
            Value::Bool(self.advanced_rag_enabled),
        );
        map.insert(
            "multi_query_enabled".to_string(),
            Value::Bool(self.multi_query_enabled),
        );
        map.insert("hyde_enabled".to_string(), Value::Bool(self.hyde_enabled));
        map.insert("bm25_enabled".to_string(), Value::Bool(self.bm25_enabled));
        map.insert(
            "reranking_enabled".to_string(),
            Value::Bool(self.reranking_enabled),
        );
        map.insert(
            "chunk_method".to_string(),
            Value::String(self.chunk_method.to_string()),
        );
        map.insert(
            "chunk_size".to_string(),
            Value::Number(serde_json::Number::from(self.chunk_size as u64)),
        );
        map.insert(
            "chunk_overlap".to_string(),
            Value::Number(serde_json::Number::from(self.chunk_overlap as u64)),
        );
        map.insert(
            "hybrid_top_k".to_string(),
            Value::Number(serde_json::Number::from(self.hybrid_top_k as u64)),
        );
        map.insert(
            "rerank_top_k".to_string(),
            Value::Number(serde_json::Number::from(self.rerank_top_k as u64)),
        );
        map.insert(
            "multi_query_count".to_string(),
            Value::Number(serde_json::Number::from(self.multi_query_count as u64)),
        );
        map.insert(
            "llm_model".to_string(),
            Value::String(self.llm_model.clone()),
        );
        map.insert(
            "llm_rerank_model".to_string(),
            Value::String(self.llm_rerank_model.clone()),
        );
        map.insert(
            "embedding_model".to_string(),
            Value::String(self.embedding_model.clone()),
        );
        map.insert(
            "llm_max_history_messages".to_string(),
            Value::Number(serde_json::Number::from(
                self.llm_max_history_messages as u64,
            )),
        );
        map.insert(
            "llm_context_token_budget".to_string(),
            Value::Number(serde_json::Number::from(
                self.llm_context_token_budget as u64,
            )),
        );
        map
    }

    /// Build from a raw JSON map, validating types and falling back to current values for missing keys.
    pub fn from_map(map: &HashMap<String, Value>, current: &RagSettings) -> Result<Self, String> {
        Ok(Self {
            advanced_rag_enabled: map
                .get("advanced_rag_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(current.advanced_rag_enabled),
            multi_query_enabled: map
                .get("multi_query_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(current.multi_query_enabled),
            hyde_enabled: map
                .get("hyde_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(current.hyde_enabled),
            bm25_enabled: map
                .get("bm25_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(current.bm25_enabled),
            reranking_enabled: map
                .get("reranking_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(current.reranking_enabled),
            chunk_method: map
                .get("chunk_method")
                .and_then(|v| v.as_str())
                .map(|s| s.parse::<ChunkMethod>())
                .transpose()?
                .unwrap_or(current.chunk_method),
            chunk_size: map
                .get("chunk_size")
                .and_then(|v| v.as_u64().map(|n| n as usize))
                .unwrap_or(current.chunk_size),
            chunk_overlap: map
                .get("chunk_overlap")
                .and_then(|v| v.as_u64().map(|n| n as usize))
                .unwrap_or(current.chunk_overlap),
            hybrid_top_k: map
                .get("hybrid_top_k")
                .and_then(|v| v.as_u64().map(|n| n as usize))
                .unwrap_or(current.hybrid_top_k),
            rerank_top_k: map
                .get("rerank_top_k")
                .and_then(|v| v.as_u64().map(|n| n as usize))
                .unwrap_or(current.rerank_top_k),
            multi_query_count: map
                .get("multi_query_count")
                .and_then(|v| v.as_u64().map(|n| n as usize))
                .unwrap_or(current.multi_query_count),
            llm_model: map
                .get("llm_model")
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or(current.llm_model.clone()),
            llm_rerank_model: map
                .get("llm_rerank_model")
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or(current.llm_rerank_model.clone()),
            embedding_model: map
                .get("embedding_model")
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or(current.embedding_model.clone()),
            llm_max_history_messages: map
                .get("llm_max_history_messages")
                .and_then(|v| v.as_u64().map(|n| n as usize))
                .unwrap_or(current.llm_max_history_messages),
            llm_context_token_budget: map
                .get("llm_context_token_budget")
                .and_then(|v| v.as_u64().map(|n| n as usize))
                .unwrap_or(current.llm_context_token_budget),
        })
    }
}

/// Internal row DTO for sqlx query_as on the settings table.
#[derive(Debug, sqlx::FromRow)]
pub struct SettingRow {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl SettingRow {
    pub fn into_entry(self) -> SettingEntry {
        SettingEntry {
            key: self.key,
            value: self.value,
            updated_at: self.updated_at,
        }
    }
}
