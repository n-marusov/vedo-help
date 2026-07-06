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

// ── Model lists (single source of truth) ──

/// A single model option returned by the /api/admin/models endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOption {
    pub value: String,
    pub label: String,
}

/// Response type for GET /api/admin/models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub llm_models: Vec<ModelOption>,
    pub embedding_models: Vec<ModelOption>,
    pub rerank_models: Vec<ModelOption>,
}

impl ModelsResponse {
    pub fn all() -> Self {
        Self {
            llm_models: llm_models(),
            embedding_models: embedding_models(),
            rerank_models: rerank_models(),
        }
    }
}

fn llm_models() -> Vec<ModelOption> {
    vec![
        // ── Anthropic Claude (Premium Frontier) ──
        ModelOption {
            value: "anthropic/claude-sonnet-5".into(),
            label: "Claude Sonnet 5 — Frontier".into(),
        },
        ModelOption {
            value: "anthropic/claude-sonnet-4.6".into(),
            label: "Claude Sonnet 4.6 — Frontier".into(),
        },
        ModelOption {
            value: "anthropic/claude-sonnet-4.5".into(),
            label: "Claude Sonnet 4.5 — Frontier".into(),
        },
        ModelOption {
            value: "anthropic/claude-sonnet-4".into(),
            label: "Claude Sonnet 4 — Frontier".into(),
        },
        ModelOption {
            value: "anthropic/claude-opus-4.8".into(),
            label: "Claude Opus 4.8 — Premium".into(),
        },
        ModelOption {
            value: "anthropic/claude-opus-4.6".into(),
            label: "Claude Opus 4.6 — Premium".into(),
        },
        ModelOption {
            value: "anthropic/claude-haiku-4.5".into(),
            label: "Claude Haiku 4.5 — Fast".into(),
        },
        ModelOption {
            value: "anthropic/claude-3-haiku".into(),
            label: "Claude 3 Haiku — Legacy".into(),
        },
        // ── OpenAI GPT (Premium Frontier) ──
        ModelOption {
            value: "openai/gpt-5.5".into(),
            label: "GPT 5.5 — Frontier".into(),
        },
        ModelOption {
            value: "openai/gpt-5.4".into(),
            label: "GPT 5.4 — Frontier".into(),
        },
        ModelOption {
            value: "openai/gpt-5.4-mini".into(),
            label: "GPT 5.4 Mini — Balanced".into(),
        },
        ModelOption {
            value: "openai/gpt-5.4-nano".into(),
            label: "GPT 5.4 Nano — Fast".into(),
        },
        ModelOption {
            value: "openai/gpt-5.3-codex".into(),
            label: "GPT 5.3 Codex — Coding".into(),
        },
        ModelOption {
            value: "openai/gpt-5.2-chat".into(),
            label: "GPT 5.2 Chat".into(),
        },
        ModelOption {
            value: "openai/gpt-5-nano".into(),
            label: "GPT 5 Nano — Ultra-cheap".into(),
        },
        ModelOption {
            value: "openai/o3-mini".into(),
            label: "O3 Mini — Reasoning".into(),
        },
        // ── Google Gemini (Premium Frontier) ──
        ModelOption {
            value: "google/gemini-2.5-pro".into(),
            label: "Gemini 2.5 Pro — Top".into(),
        },
        ModelOption {
            value: "google/gemini-2.5-flash".into(),
            label: "Gemini 2.5 Flash — Fast".into(),
        },
        ModelOption {
            value: "google/gemini-3-flash-preview".into(),
            label: "Gemini 3 Flash Preview".into(),
        },
        // ── DeepSeek (Premium Frontier) ──
        ModelOption {
            value: "deepseek/deepseek-v4-pro".into(),
            label: "DeepSeek V4 Pro".into(),
        },
        ModelOption {
            value: "deepseek/deepseek-v4-flash".into(),
            label: "DeepSeek V4 Flash".into(),
        },
        // ── Best Value (Balanced Price/Performance) ──
        ModelOption {
            value: "qwen/qwen3-coder-plus".into(),
            label: "Qwen 3 Coder Plus".into(),
        },
        ModelOption {
            value: "qwen/qwen3-plus".into(),
            label: "Qwen 3 Plus — Balanced".into(),
        },
        ModelOption {
            value: "qwen/qwen3.5-flash".into(),
            label: "Qwen 3.5 Flash — Budget".into(),
        },
        ModelOption {
            value: "mistralai/mistral-large-3-2512".into(),
            label: "Mistral Large 3 — Apache 2.0".into(),
        },
        ModelOption {
            value: "mistralai/mistral-small-4".into(),
            label: "Mistral Small 4".into(),
        },
        ModelOption {
            value: "meta-llama/llama-4-maverick".into(),
            label: "Llama 4 Maverick — 1M ctx".into(),
        },
        ModelOption {
            value: "meta-llama/llama-4-scout".into(),
            label: "Llama 4 Scout — 10M ctx".into(),
        },
        ModelOption {
            value: "nvidia/nemotron-3-super".into(),
            label: "Nemotron 3 Super — 1M ctx".into(),
        },
        ModelOption {
            value: "cohere/command-r-08-2024".into(),
            label: "Command R — RAG & Tools".into(),
        },
        // ── Budget / Open Models ──
        ModelOption {
            value: "qwen/qwen3-32b".into(),
            label: "Qwen 3 32B — Budget".into(),
        },
        ModelOption {
            value: "google/gemma-3-27b-it".into(),
            label: "Gemma 3 27B — Open".into(),
        },
        ModelOption {
            value: "qwen/qwen3-8b".into(),
            label: "Qwen 3 8B — Budget".into(),
        },
    ]
}

fn embedding_models() -> Vec<ModelOption> {
    vec![
        ModelOption {
            value: "sentence-transformers/all-minilm-l6-v2".into(),
            label: "all-MiniLM-L6-v2 (384d, default)".into(),
        },
        ModelOption {
            value: "sentence-transformers/all-mpnet-base-v2".into(),
            label: "all-mpnet-base-v2 (768d)".into(),
        },
        ModelOption {
            value: "openai/text-embedding-3-small".into(),
            label: "Text Embedding 3 Small (512-1536d)".into(),
        },
        ModelOption {
            value: "openai/text-embedding-3-large".into(),
            label: "Text Embedding 3 Large (256-3072d)".into(),
        },
        ModelOption {
            value: "google/gemini-embedding-001".into(),
            label: "Gemini Embedding 001 (768d)".into(),
        },
        ModelOption {
            value: "google/gemini-embedding-2".into(),
            label: "Gemini Embedding 2 (128-3072d, multimodal)".into(),
        },
        ModelOption {
            value: "qwen/qwen3-embedding-8b".into(),
            label: "Qwen3 Embedding 8B (32K ctx)".into(),
        },
        ModelOption {
            value: "qwen/qwen3-embedding-4b".into(),
            label: "Qwen3 Embedding 4B (33K ctx)".into(),
        },
        ModelOption {
            value: "baai/bge-m3".into(),
            label: "BGE M3 (1024d, multilingual)".into(),
        },
        ModelOption {
            value: "baai/bge-large-en-v1.5".into(),
            label: "BGE Large EN v1.5 (1024d)".into(),
        },
        ModelOption {
            value: "baai/bge-base-en-v1.5".into(),
            label: "BGE Base EN v1.5 (768d)".into(),
        },
        ModelOption {
            value: "intfloat/e5-large-v2".into(),
            label: "E5 Large V2 (1024d)".into(),
        },
        ModelOption {
            value: "intfloat/e5-base-v2".into(),
            label: "E5 Base V2 (768d)".into(),
        },
        ModelOption {
            value: "intfloat/multilingual-e5-large".into(),
            label: "Multilingual E5 Large (1024d, 90+ langs)".into(),
        },
        ModelOption {
            value: "mistralai/mistral-embed-2312".into(),
            label: "Mistral Embed (1024d)".into(),
        },
        ModelOption {
            value: "mistralai/codestral-embed-2505".into(),
            label: "Codestral Embed (1024d, code)".into(),
        },
        ModelOption {
            value: "thenlper/gte-base".into(),
            label: "GTE Base (768d, efficient)".into(),
        },
        ModelOption {
            value: "thenlper/gte-large".into(),
            label: "GTE Large (1024d, high quality)".into(),
        },
        ModelOption {
            value: "perplexity/pplx-embed-v1-4b".into(),
            label: "Perplexity Embed v1 4B (variable dims, 32K ctx)".into(),
        },
        ModelOption {
            value: "perplexity/pplx-embed-v1-0.6b".into(),
            label: "Perplexity Embed v1 0.6B (ultra-cheap)".into(),
        },
    ]
}

fn rerank_models() -> Vec<ModelOption> {
    vec![
        // ── Dedicated Rerankers (best quality) ──
        ModelOption {
            value: "cohere/rerank-4-pro".into(),
            label: "Cohere Rerank 4 Pro — 32K ctx, 100+ languages".into(),
        },
        ModelOption {
            value: "cohere/rerank-4-fast".into(),
            label: "Cohere Rerank 4 Fast — 32K ctx, low latency".into(),
        },
        ModelOption {
            value: "cohere/rerank-v3.5".into(),
            label: "Cohere Rerank v3.5 — 4K ctx, legacy".into(),
        },
        // ── LLMs for Reranking (prompt-based) ──
        ModelOption {
            value: "anthropic/claude-sonnet-4.6".into(),
            label: "Claude Sonnet 4.6 — Frontier (prompt-based)".into(),
        },
        ModelOption {
            value: "anthropic/claude-sonnet-4.5".into(),
            label: "Claude Sonnet 4.5 — Frontier (prompt-based)".into(),
        },
        ModelOption {
            value: "openai/gpt-5.4-mini".into(),
            label: "GPT 5.4 Mini — Balanced (prompt-based)".into(),
        },
        ModelOption {
            value: "openai/gpt-5.4-nano".into(),
            label: "GPT 5.4 Nano — Fast (prompt-based)".into(),
        },
        ModelOption {
            value: "google/gemini-2.5-flash".into(),
            label: "Gemini 2.5 Flash — Fast (prompt-based)".into(),
        },
        ModelOption {
            value: "qwen/qwen3-plus".into(),
            label: "Qwen 3 Plus — Balanced (prompt-based)".into(),
        },
        ModelOption {
            value: "qwen/qwen3.5-flash".into(),
            label: "Qwen 3.5 Flash — Budget (prompt-based)".into(),
        },
        ModelOption {
            value: "qwen/qwen3-32b".into(),
            label: "Qwen 3 32B — Budget (prompt-based)".into(),
        },
        ModelOption {
            value: "meta-llama/llama-4-scout".into(),
            label: "Llama 4 Scout — 10M ctx (prompt-based)".into(),
        },
    ]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let s = RagSettings::default();
        assert!(s.advanced_rag_enabled);
        assert_eq!(s.chunk_method, ChunkMethod::Paragraph);
        assert_eq!(s.chunk_size, 1000);
        assert_eq!(s.chunk_overlap, 200);
        assert_eq!(s.hybrid_top_k, 20);
        assert_eq!(s.rerank_top_k, 5);
        assert_eq!(s.multi_query_count, 3);
        assert_eq!(s.llm_model, "anthropic/claude-sonnet-4.6");
        assert_eq!(s.llm_rerank_model, "anthropic/claude-sonnet-4.6");
        assert_eq!(s.embedding_model, "sentence-transformers/all-minilm-l6-v2");
        assert_eq!(s.llm_max_history_messages, 20);
        assert_eq!(s.llm_context_token_budget, 6000);
    }

    #[test]
    fn test_to_map_contains_all_keys() {
        let s = RagSettings::default();
        let map = s.to_map();
        assert_eq!(map.len(), 16);
        assert!(map.contains_key("advanced_rag_enabled"));
        assert!(map.contains_key("llm_model"));
        assert!(map.contains_key("embedding_model"));
        assert!(map.contains_key("llm_rerank_model"));
        assert!(map.contains_key("chunk_size"));
        assert!(map.contains_key("chunk_overlap"));
        assert!(map.contains_key("chunk_method"));
        assert_eq!(map.get("llm_model").unwrap(), "anthropic/claude-sonnet-4.6");
        assert_eq!(
            map.get("embedding_model").unwrap(),
            "sentence-transformers/all-minilm-l6-v2"
        );
    }

    #[test]
    fn test_from_map_roundtrip() {
        let original = RagSettings::default();
        let map = original.to_map();
        let restored = RagSettings::from_map(&map, &original).unwrap();
        assert_eq!(restored.advanced_rag_enabled, original.advanced_rag_enabled);
        assert_eq!(restored.chunk_method, original.chunk_method);
        assert_eq!(restored.chunk_size, original.chunk_size);
        assert_eq!(restored.llm_model, original.llm_model);
        assert_eq!(restored.embedding_model, original.embedding_model);
        assert_eq!(restored.llm_rerank_model, original.llm_rerank_model);
        assert_eq!(
            restored.llm_max_history_messages,
            original.llm_max_history_messages
        );
    }

    #[test]
    fn test_from_map_falls_back_for_missing_keys() {
        let current = RagSettings::default();
        let empty = HashMap::new();
        let restored = RagSettings::from_map(&empty, &current).unwrap();
        // All values should fall back to current
        assert_eq!(restored.llm_model, current.llm_model);
        assert_eq!(restored.embedding_model, current.embedding_model);
        assert_eq!(restored.chunk_size, current.chunk_size);
        assert!(restored.advanced_rag_enabled);
    }

    #[test]
    fn test_from_map_overrides_specific_keys() {
        let current = RagSettings::default();
        let mut map = HashMap::new();
        map.insert(
            "llm_model".to_string(),
            Value::String("custom-model".to_string()),
        );
        map.insert(
            "chunk_size".to_string(),
            Value::Number(serde_json::Number::from(500u64)),
        );
        let restored = RagSettings::from_map(&map, &current).unwrap();
        assert_eq!(restored.llm_model, "custom-model");
        assert_eq!(restored.chunk_size, 500);
        // Unchanged values stay at current
        assert_eq!(restored.embedding_model, current.embedding_model);
        assert_eq!(restored.chunk_method, current.chunk_method);
    }

    #[test]
    fn test_from_map_rejects_invalid_chunk_method() {
        let current = RagSettings::default();
        let mut map = HashMap::new();
        map.insert(
            "chunk_method".to_string(),
            Value::String("invalid".to_string()),
        );
        let result = RagSettings::from_map(&map, &current);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown chunk method"));
    }

    #[test]
    fn test_with_env_overrides() {
        let defaults = RagSettings::default();
        let mut env = crate::config::AppConfig::from_env();
        env.llm_model = "env-llm".to_string();
        env.embedding_model = "env-embed".to_string();

        let overridden = defaults.with_env_overrides(&env);
        assert_eq!(overridden.llm_model, "env-llm");
        assert_eq!(overridden.embedding_model, "env-embed");
        // Non-overridden defaults preserved
        assert_eq!(overridden.chunk_method, ChunkMethod::Paragraph);
        assert_eq!(overridden.chunk_size, 1000);
    }

    #[test]
    fn test_chunk_method_display() {
        assert_eq!(ChunkMethod::Paragraph.to_string(), "paragraph");
        assert_eq!(ChunkMethod::Fixed.to_string(), "fixed");
    }

    #[test]
    fn test_chunk_method_from_str() {
        assert_eq!(
            "paragraph".parse::<ChunkMethod>().unwrap(),
            ChunkMethod::Paragraph
        );
        assert_eq!("fixed".parse::<ChunkMethod>().unwrap(), ChunkMethod::Fixed);
        assert_eq!(
            "PARAGRAPH".parse::<ChunkMethod>().unwrap(),
            ChunkMethod::Paragraph
        );
        assert!("invalid".parse::<ChunkMethod>().is_err());
    }

    #[test]
    fn test_setting_row_into_entry() {
        let row = SettingRow {
            key: "test_key".to_string(),
            value: Value::String("test_val".to_string()),
            updated_at: chrono::Utc::now(),
        };
        let entry: SettingEntry = row.into_entry();
        assert_eq!(entry.key, "test_key");
        assert_eq!(entry.value, "test_val");
    }
}
