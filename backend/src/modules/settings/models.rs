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
    /// Auto-detected embedding vector dimension.
    /// `None` = not yet detected (backward-compat with settings that predate this feature).
    pub embedding_dimension: Option<usize>,
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
            embedding_dimension: None,
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
        if let Some(dim) = self.embedding_dimension {
            map.insert(
                "embedding_dimension".to_string(),
                Value::Number(serde_json::Number::from(dim as u64)),
            );
        }
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
            embedding_dimension: map
                .get("embedding_dimension")
                .and_then(|v| v.as_u64().map(|n| n as usize))
                .or(current.embedding_dimension),
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
    /// Human-readable cost string, e.g. "301 ₽/1M input, 1,506 ₽/1M output"
    /// or "0.25 ₽/search unit" for dedicated rerankers.
    /// Populated dynamically from the PricingCache; may be None if cache is not yet loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<String>,
}

impl ModelOption {
    /// Create a new ModelOption without pricing (pricing will be enriched by PricingCache at runtime).
    pub fn pair(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            pricing: None,
        }
    }
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
        ModelOption::pair("anthropic/claude-sonnet-5", "Claude Sonnet 5 — Frontier"),
        ModelOption::pair(
            "anthropic/claude-sonnet-4.6",
            "Claude Sonnet 4.6 — Frontier",
        ),
        ModelOption::pair(
            "anthropic/claude-sonnet-4.5",
            "Claude Sonnet 4.5 — Frontier",
        ),
        ModelOption::pair("anthropic/claude-sonnet-4", "Claude Sonnet 4 — Frontier"),
        ModelOption::pair("anthropic/claude-opus-4.8", "Claude Opus 4.8 — Premium"),
        ModelOption::pair("anthropic/claude-opus-4.7", "Claude Opus 4.7 — Premium"),
        ModelOption::pair("anthropic/claude-opus-4.6", "Claude Opus 4.6 — Premium"),
        ModelOption::pair("anthropic/claude-opus-4.5", "Claude Opus 4.5 — Premium"),
        ModelOption::pair("anthropic/claude-fable-5", "Claude Fable 5 — Mythos-tier"),
        ModelOption::pair("anthropic/claude-haiku-4.5", "Claude Haiku 4.5 — Fast"),
        ModelOption::pair("anthropic/claude-3-haiku", "Claude 3 Haiku — Legacy"),
        // ── OpenAI GPT (Premium Frontier) ──
        ModelOption::pair("openai/gpt-5.5", "GPT 5.5 — Frontier"),
        ModelOption::pair("openai/gpt-5.5-pro", "GPT 5.5 Pro — Premium"),
        ModelOption::pair("openai/gpt-5.4", "GPT 5.4 — Frontier"),
        ModelOption::pair("openai/gpt-5.4-pro", "GPT 5.4 Pro — Premium"),
        ModelOption::pair("openai/gpt-5.4-mini", "GPT 5.4 Mini — Balanced"),
        ModelOption::pair("openai/gpt-5.4-nano", "GPT 5.4 Nano — Fast"),
        ModelOption::pair("openai/gpt-5.3-codex", "GPT 5.3 Codex — Coding"),
        ModelOption::pair("openai/gpt-5.2", "GPT 5.2 — Frontier"),
        ModelOption::pair("openai/gpt-5.2-chat", "GPT 5.2 Chat — Fast"),
        ModelOption::pair("openai/gpt-5.2-pro", "GPT 5.2 Pro — Premium"),
        ModelOption::pair("openai/gpt-5.1", "GPT 5.1 — Frontier"),
        ModelOption::pair("openai/gpt-5.1-codex", "GPT 5.1 Codex — Coding"),
        ModelOption::pair("openai/gpt-5", "GPT 5 — Frontier"),
        ModelOption::pair("openai/gpt-5-mini", "GPT 5 Mini — Balanced"),
        ModelOption::pair("openai/gpt-5-nano", "GPT 5 Nano — Ultra-cheap"),
        ModelOption::pair("openai/gpt-5-codex", "GPT 5 Codex — Coding"),
        ModelOption::pair("openai/gpt-4o", "GPT 4o — Legacy multimodal"),
        ModelOption::pair("openai/gpt-4o-mini", "GPT 4o Mini — Legacy, cheap"),
        ModelOption::pair("openai/gpt-4.1", "GPT 4.1 — 1M ctx, legacy"),
        ModelOption::pair("openai/gpt-4.1-mini", "GPT 4.1 Mini — 1M ctx, legacy"),
        ModelOption::pair("openai/gpt-4.1-nano", "GPT 4.1 Nano — 1M ctx, legacy"),
        ModelOption::pair("openai/o4-mini", "O4 Mini — Compact reasoning"),
        ModelOption::pair("openai/o4-mini-high", "O4 Mini High — Deep reasoning"),
        ModelOption::pair("openai/o3-mini", "O3 Mini — Reasoning"),
        ModelOption::pair("openai/o3", "O3 — Strong reasoning"),
        ModelOption::pair("openai/o1", "O1 — Legacy reasoning"),
        // ── Google Gemini (Premium Frontier) ──
        ModelOption::pair("google/gemini-3.5-flash", "Gemini 3.5 Flash — Fast"),
        ModelOption::pair("google/gemini-3.1-pro-preview", "Gemini 3.1 Pro — Frontier"),
        ModelOption::pair(
            "google/gemini-3.1-flash-lite",
            "Gemini 3.1 Flash Lite — Budget",
        ),
        ModelOption::pair(
            "google/gemini-3-flash-preview",
            "Gemini 3 Flash Preview — Fast",
        ),
        ModelOption::pair("google/gemini-2.5-pro", "Gemini 2.5 Pro — Top"),
        ModelOption::pair("google/gemini-2.5-flash", "Gemini 2.5 Flash — Fast"),
        ModelOption::pair(
            "google/gemini-2.5-flash-lite",
            "Gemini 2.5 Flash Lite — Budget",
        ),
        // ── DeepSeek (Premium Frontier) ──
        ModelOption::pair("deepseek/deepseek-v4-pro", "DeepSeek V4 Pro — Frontier"),
        ModelOption::pair("deepseek/deepseek-v4-flash", "DeepSeek V4 Flash — Fast"),
        ModelOption::pair("deepseek/deepseek-v3.2", "DeepSeek V3.2 — Reasoning"),
        ModelOption::pair("deepseek/deepseek-chat-v3.1", "DeepSeek V3.1 — Balanced"),
        ModelOption::pair("deepseek/deepseek-r1", "DeepSeek R1 — Open reasoning"),
        ModelOption::pair(
            "deepseek/deepseek-r1-0528",
            "DeepSeek R1 0528 — Updated reasoning",
        ),
        // ── Google Gemma (Open) ──
        ModelOption::pair("google/gemma-4-31b-it", "Gemma 4 31B — Apache 2.0"),
        ModelOption::pair(
            "google/gemma-4-26b-a4b-it",
            "Gemma 4 26B A4B — Apache 2.0 (MoE)",
        ),
        ModelOption::pair("google/gemma-3-27b-it", "Gemma 3 27B — Open"),
        // ── Best Value (Balanced Price/Performance) ──
        ModelOption::pair("qwen/qwen3.7-max", "Qwen 3.7 Max — Flagship"),
        ModelOption::pair("qwen/qwen3.7-plus", "Qwen 3.7 Plus — Balanced"),
        ModelOption::pair("qwen/qwen3.6-plus", "Qwen 3.6 Plus — Strong coding"),
        ModelOption::pair("qwen/qwen3.6-flash", "Qwen 3.6 Flash — Fast"),
        ModelOption::pair("qwen/qwen3.5-plus", "Qwen 3.5 Plus — Balanced"),
        ModelOption::pair("qwen/qwen3.5-flash", "Qwen 3.5 Flash — Budget"),
        ModelOption::pair("qwen/qwen3-coder-plus", "Qwen 3 Coder Plus — Coding"),
        ModelOption::pair("qwen/qwen3-coder-flash", "Qwen 3 Coder Flash — Coding fast"),
        ModelOption::pair("qwen/qwen3-plus", "Qwen 3 Plus — Balanced"),
        ModelOption::pair("qwen/qwen3-max", "Qwen 3 Max — Open flagship"),
        ModelOption::pair("qwen/qwen3-max-thinking", "Qwen 3 Max Thinking — Reasoning"),
        // ── Mistral AI ──
        ModelOption::pair(
            "mistralai/mistral-large-3-2512",
            "Mistral Large 3 — Apache 2.0",
        ),
        ModelOption::pair("mistralai/mistral-medium-3.5", "Mistral Medium 3.5 — 128B"),
        ModelOption::pair("mistralai/mistral-small-4", "Mistral Small 4"),
        ModelOption::pair(
            "mistralai/mistral-small-3.2-24b-instruct",
            "Mistral Small 3.2 24B",
        ),
        ModelOption::pair(
            "mistralai/codestral-2508",
            "Codestral 2508 — Code-specialized",
        ),
        // ── Meta Llama (Open) ──
        ModelOption::pair("meta-llama/llama-4-maverick", "Llama 4 Maverick — 1M ctx"),
        ModelOption::pair("meta-llama/llama-4-scout", "Llama 4 Scout — 10M ctx"),
        ModelOption::pair("meta-llama/llama-3.3-70b-instruct", "Llama 3.3 70B — Open"),
        ModelOption::pair(
            "meta-llama/llama-3.1-70b-instruct",
            "Llama 3.1 70B — Legacy open",
        ),
        ModelOption::pair(
            "meta-llama/llama-3.1-8b-instruct",
            "Llama 3.1 8B — Lightweight",
        ),
        // ── xAI Grok ──
        ModelOption::pair("x-ai/grok-4.20", "Grok 4.20 — Frontier"),
        ModelOption::pair("x-ai/grok-4.3", "Grok 4.3 — Balanced"),
        ModelOption::pair("x-ai/grok-build-0.1", "Grok Build 0.1 — Coding agent"),
        // ── MiniMax ──
        ModelOption::pair("minimax/minimax-m3", "MiniMax M3 — 1M ctx"),
        ModelOption::pair("minimax/minimax-m2.7", "MiniMax M2.7 — Strong agent"),
        ModelOption::pair("minimax/minimax-m2.5", "MiniMax M2.5 — 80% SWE"),
        ModelOption::pair("minimax/minimax-m2.1", "MiniMax M2.1 — Lightweight"),
        // ── Additional Providers ──
        ModelOption::pair("moonshotai/kimi-k2.5", "Kimi K2.5 — Multimodal coding"),
        ModelOption::pair(
            "moonshotai/kimi-k2-thinking",
            "Kimi K2 Thinking — Reasoning",
        ),
        ModelOption::pair("cohere/command-a", "Command A — 256K ctx, agentic"),
        ModelOption::pair("cohere/command-r-08-2024", "Command R — RAG & Tools"),
        ModelOption::pair("nvidia/nemotron-3-super", "Nemotron 3 Super — 1M ctx"),
        ModelOption::pair(
            "nvidia/nemotron-3-ultra-550b-a55b",
            "Nemotron 3 Ultra — Orchestrator",
        ),
        ModelOption::pair("z-ai/glm-5.2", "GLM 5.2 — 1M ctx, reasoning"),
        ModelOption::pair("z-ai/glm-5", "GLM 5 — Open, system design"),
        ModelOption::pair("amazon/nova-2-lite-v1", "Nova 2 Lite — Amazon budget"),
        ModelOption::pair(
            "ibm-granite/granite-4.1-8b",
            "Granite 4.1 8B — IBM enterprise",
        ),
        // ── Budget / Open Models ──
        ModelOption::pair("qwen/qwen3-32b", "Qwen 3 32B — Budget"),
        ModelOption::pair("qwen/qwen3-14b", "Qwen 3 14B — Budget"),
        ModelOption::pair("qwen/qwen3-8b", "Qwen 3 8B — Budget"),
        ModelOption::pair("google/gemma-3-12b-it", "Gemma 3 12B — Open"),
        ModelOption::pair("google/gemma-3-4b-it", "Gemma 3 4B — Tiny"),
        ModelOption::pair("microsoft/phi-4", "Phi 4 — 14B, reasoning"),
        ModelOption::pair("mistralai/mistral-nemo", "Mistral Nemo — 12B, Apache 2.0"),
        ModelOption::pair("inclusionai/ling-2.6-flash", "Ling 2.6 Flash — Ultra-cheap"),
        ModelOption::pair("meta-llama/llama-3.2-3b-instruct", "Llama 3.2 3B — Tiny"),
    ]
}

fn embedding_models() -> Vec<ModelOption> {
    vec![
        ModelOption::pair(
            "sentence-transformers/all-minilm-l6-v2",
            "all-MiniLM-L6-v2 (384d, default)",
        ),
        ModelOption::pair(
            "sentence-transformers/all-mpnet-base-v2",
            "all-mpnet-base-v2 (768d)",
        ),
        ModelOption::pair(
            "openai/text-embedding-3-small",
            "OpenAI text-embedding-3-small (512-1536d)",
        ),
        ModelOption::pair(
            "openai/text-embedding-3-large",
            "OpenAI text-embedding-3-large (256-3072d)",
        ),
        ModelOption::pair("qwen/qwen3-embedding-8b", "Qwen3 Embedding 8B (32K ctx)"),
        ModelOption::pair("qwen/qwen3-embedding-4b", "Qwen3 Embedding 4B (33K ctx)"),
        ModelOption::pair(
            "google/gemini-embedding-2",
            "Gemini Embedding 2 (128-3072d, multimodal)",
        ),
        ModelOption::pair(
            "google/gemini-embedding-001",
            "Gemini Embedding 001 (768d, top MTEB)",
        ),
        ModelOption::pair("baai/bge-m3", "BGE M3 (1024d, multilingual)"),
        ModelOption::pair("baai/bge-large-en-v1.5", "BGE Large EN v1.5 (1024d)"),
        ModelOption::pair("baai/bge-base-en-v1.5", "BGE Base EN v1.5 (768d)"),
        ModelOption::pair("intfloat/e5-large-v2", "E5 Large V2 (1024d)"),
        ModelOption::pair("intfloat/e5-base-v2", "E5 Base V2 (768d)"),
        ModelOption::pair(
            "intfloat/multilingual-e5-large",
            "Multilingual E5 Large (1024d, 90+ langs)",
        ),
        ModelOption::pair("mistralai/mistral-embed-2312", "Mistral Embed (1024d)"),
        ModelOption::pair(
            "mistralai/codestral-embed-2505",
            "Codestral Embed (1024d, code)",
        ),
        ModelOption::pair("thenlper/gte-base", "GTE Base (768d, efficient)"),
        ModelOption::pair("thenlper/gte-large", "GTE Large (1024d, high quality)"),
        ModelOption::pair(
            "perplexity/pplx-embed-v1-4b",
            "Perplexity Embed v1 4B (variable dims, 32K ctx)",
        ),
        ModelOption::pair(
            "perplexity/pplx-embed-v1-0.6b",
            "Perplexity Embed v1 0.6B (ultra-cheap)",
        ),
    ]
}

fn rerank_models() -> Vec<ModelOption> {
    vec![
        // ── Dedicated Rerankers (best quality) ──
        ModelOption::pair(
            "cohere/rerank-4-pro",
            "Cohere Rerank 4 Pro — 32K ctx, 100+ languages",
        ),
        ModelOption::pair(
            "cohere/rerank-4-fast",
            "Cohere Rerank 4 Fast — 32K ctx, low latency",
        ),
        ModelOption::pair("cohere/rerank-v3.5", "Cohere Rerank v3.5 — 4K ctx, legacy"),
        // ── Frontier (prompt-based) ──
        ModelOption::pair(
            "anthropic/claude-sonnet-5",
            "Claude Sonnet 5 — Frontier (prompt-based)",
        ),
        ModelOption::pair(
            "anthropic/claude-sonnet-4.6",
            "Claude Sonnet 4.6 — Frontier (prompt-based)",
        ),
        ModelOption::pair(
            "anthropic/claude-sonnet-4.5",
            "Claude Sonnet 4.5 — Frontier (prompt-based)",
        ),
        // ── Fast (prompt-based) ──
        ModelOption::pair(
            "anthropic/claude-haiku-4.5",
            "Claude Haiku 4.5 — Fast, 200K ctx (prompt-based)",
        ),
        ModelOption::pair(
            "openai/gpt-4.1-mini",
            "GPT 4.1 Mini — 1M ctx, efficient (prompt-based)",
        ),
        ModelOption::pair("openai/gpt-5.4-nano", "GPT 5.4 Nano — Fast (prompt-based)"),
        ModelOption::pair(
            "deepseek/deepseek-v4-flash",
            "DeepSeek V4 Flash — Fast, 1M ctx (prompt-based)",
        ),
        ModelOption::pair(
            "google/gemini-3.5-flash",
            "Gemini 3.5 Flash — Fast (prompt-based)",
        ),
        ModelOption::pair(
            "google/gemini-2.5-flash",
            "Gemini 2.5 Flash — Fast (prompt-based)",
        ),
        ModelOption::pair(
            "google/gemini-2.5-flash-lite",
            "Gemini 2.5 Flash Lite — Ultra cheap (prompt-based)",
        ),
        ModelOption::pair("qwen/qwen3.6-flash", "Qwen 3.6 Flash — Fast (prompt-based)"),
        // ── Balanced (prompt-based) ──
        ModelOption::pair(
            "openai/gpt-5.4-mini",
            "GPT 5.4 Mini — Balanced (prompt-based)",
        ),
        ModelOption::pair("qwen/qwen3-plus", "Qwen 3 Plus — Balanced (prompt-based)"),
        ModelOption::pair(
            "qwen/qwen3.5-plus",
            "Qwen 3.5 Plus — Balanced (prompt-based)",
        ),
        // ── Budget (prompt-based) ──
        ModelOption::pair(
            "qwen/qwen3.5-flash",
            "Qwen 3.5 Flash — Budget (prompt-based)",
        ),
        ModelOption::pair("qwen/qwen3-32b", "Qwen 3 32B — Budget (prompt-based)"),
        ModelOption::pair(
            "meta-llama/llama-4-scout",
            "Llama 4 Scout — 10M ctx (prompt-based)",
        ),
        ModelOption::pair(
            "openai/gpt-5-nano",
            "GPT 5 Nano — Ultra-fast, ultra-cheap (prompt-based)",
        ),
        ModelOption::pair(
            "mistralai/mistral-small-4",
            "Mistral Small 4 — Budget (prompt-based)",
        ),
        ModelOption::pair(
            "inclusionai/ling-2.6-flash",
            "Ling 2.6 Flash — Ultra-cheap (prompt-based)",
        ),
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
        assert_eq!(
            s.embedding_dimension, None,
            "embedding_dimension should be None by default (backward compat)"
        );
        assert_eq!(s.llm_max_history_messages, 20);
        assert_eq!(s.llm_context_token_budget, 6000);
    }

    #[test]
    fn test_embedding_dimension_roundtrip() {
        // Regression: when embedding_dimension is set, to_map must include it
        // and from_map must restore it.
        let mut settings = RagSettings::default();
        settings.embedding_dimension = Some(1536);

        let map = settings.to_map();
        assert!(
            map.contains_key("embedding_dimension"),
            "to_map must include embedding_dimension when Some"
        );
        assert_eq!(
            map.get("embedding_dimension").and_then(|v| v.as_u64()),
            Some(1536),
            "embedding_dimension value must be 1536"
        );

        let restored = RagSettings::from_map(&map, &settings).unwrap();
        assert_eq!(
            restored.embedding_dimension,
            Some(1536),
            "from_map must restore embedding_dimension"
        );
    }

    #[test]
    fn test_embedding_dimension_excluded_when_none() {
        // Regression: when embedding_dimension is None, to_map must NOT include it
        // so existing admin panels don't see an unexpected null field.
        let settings = RagSettings::default();
        assert_eq!(
            settings.embedding_dimension, None,
            "precondition: default is None"
        );

        let map = settings.to_map();
        assert!(
            !map.contains_key("embedding_dimension"),
            "to_map must omit embedding_dimension when None"
        );
    }

    #[test]
    fn test_embedding_dimension_fallback_on_missing_key() {
        // Regression: from_map must fall back to current value when the key
        // is not present in the incoming map.
        let current = RagSettings {
            embedding_dimension: Some(768),
            ..RagSettings::default()
        };

        let empty = HashMap::new();
        let restored = RagSettings::from_map(&empty, &current).unwrap();
        assert_eq!(
            restored.embedding_dimension,
            Some(768),
            "must fall back to current.embedding_dimension when key is missing"
        );
    }

    #[test]
    fn test_embedding_dimension_override_via_from_map() {
        // Regression: from_map must accept embedding_dimension and override
        // the current value.
        let current = RagSettings::default();
        let mut map = HashMap::new();
        map.insert("embedding_dimension".to_string(), serde_json::json!(384));
        map.insert(
            "embedding_model".to_string(),
            serde_json::json!("sentence-transformers/all-minilm-l6-v2"),
        );

        let restored = RagSettings::from_map(&map, &current).unwrap();
        assert_eq!(
            restored.embedding_dimension,
            Some(384),
            "from_map must override embedding_dimension when key is present"
        );
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
