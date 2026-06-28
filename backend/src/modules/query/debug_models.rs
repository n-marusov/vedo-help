use serde::{Deserialize, Serialize};

/// 7-step RAG pipeline debug data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebugData {
    pub query_text: String,
    /// v0.5 — not yet active
    pub multi_query: Option<MultiQueryStep>,
    /// v0.5 — not yet active
    pub hyde: Option<HydeStep>,
    /// Active step — embedding/vector search results
    pub embedding_search: Option<EmbeddingSearchStep>,
    /// v0.5 — not yet active
    pub keyword_search: Option<KeywordSearchStep>,
    /// v0.5 — not yet active
    pub merge_dedup: Option<MergeDedupStep>,
    /// v0.5 — not yet active
    pub reranking: Option<RerankingStep>,
    /// Active step — final LLM answer metadata
    pub final_answer: Option<FinalAnswerStep>,
}

impl DebugData {
    /// Create a new DebugData with the given query text.
    /// All steps default to `None`.
    pub fn new(query_text: impl Into<String>) -> Self {
        Self {
            query_text: query_text.into(),
            ..Default::default()
        }
    }
}

/// Multi-query expansion step (v0.5 — placeholder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiQueryStep;

/// Hypothetical Document Embeddings step (v0.5 — placeholder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydeStep;

/// Keyword/BM25 search step (v0.5 — placeholder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordSearchStep;

/// Merge and deduplication step (v0.5 — placeholder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeDedupStep;

/// Re-ranking step (v0.5 — placeholder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankingStep;

/// Embedding/vector search step — active, shows real results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingSearchStep {
    pub query_snippet: String,
    pub embedding_dimension: usize,
    pub latency_ms: u64,
    pub collection_name: String,
    pub top_k: usize,
    pub result_count: usize,
    pub retries: u32,
    pub results: Vec<SearchResultItem>,
}

/// A single search result item from Chroma.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub chunk_id: String,
    pub document_name: String,
    pub chunk_index: usize,
    pub score: f64,
    pub text_snippet: String,
}

/// Final answer step — active, shows LLM response metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalAnswerStep {
    pub model: String,
    pub max_retries: u32,
    pub chunks_in_context: usize,
    pub history_message_count: usize,
    pub history_token_estimate: usize,
    pub token_budget: usize,
    pub total_tokens_estimate: usize,
    pub latency_ms: u64,
    pub prompt_preview: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_data_defaults_are_none() {
        let dd = DebugData::default();
        assert_eq!(dd.query_text, "");
        assert!(dd.multi_query.is_none());
        assert!(dd.hyde.is_none());
        assert!(dd.embedding_search.is_none());
        assert!(dd.keyword_search.is_none());
        assert!(dd.merge_dedup.is_none());
        assert!(dd.reranking.is_none());
        assert!(dd.final_answer.is_none());
    }

    #[test]
    fn embedding_search_step_stores_metadata() {
        let step = EmbeddingSearchStep {
            query_snippet: "how to install".to_string(),
            embedding_dimension: 384,
            latency_ms: 45,
            collection_name: "abc-123".to_string(),
            top_k: 5,
            result_count: 5,
            retries: 0,
            results: vec![SearchResultItem {
                chunk_id: "chunk-1".to_string(),
                document_name: "docs.pdf".to_string(),
                chunk_index: 3,
                score: 0.92,
                text_snippet: "To install, run docker compose up".to_string(),
            }],
        };
        assert_eq!(step.query_snippet, "how to install");
        assert_eq!(step.embedding_dimension, 384);
        assert_eq!(step.latency_ms, 45);
        assert_eq!(step.collection_name, "abc-123");
        assert_eq!(step.top_k, 5);
        assert_eq!(step.result_count, 5);
        assert_eq!(step.retries, 0);
        assert_eq!(step.results.len(), 1);
        assert_eq!(step.results[0].document_name, "docs.pdf");
    }

    #[test]
    fn final_answer_step_stores_llm_info() {
        let step = FinalAnswerStep {
            model: "gpt-4".to_string(),
            max_retries: 3,
            chunks_in_context: 5,
            history_message_count: 2,
            history_token_estimate: 500,
            token_budget: 4000,
            total_tokens_estimate: 3500,
            latency_ms: 1200,
            prompt_preview: "Answer the question based on the context...".to_string(),
        };
        assert_eq!(step.model, "gpt-4");
        assert_eq!(step.max_retries, 3);
        assert_eq!(step.chunks_in_context, 5);
        assert_eq!(step.history_message_count, 2);
        assert_eq!(step.history_token_estimate, 500);
        assert_eq!(step.token_budget, 4000);
        assert_eq!(step.total_tokens_estimate, 3500);
        assert_eq!(step.latency_ms, 1200);
        assert_eq!(
            step.prompt_preview,
            "Answer the question based on the context..."
        );
    }

    #[test]
    fn debug_data_serializes_correctly() {
        let dd = DebugData {
            query_text: "test query".to_string(),
            multi_query: None,
            hyde: None,
            embedding_search: Some(EmbeddingSearchStep {
                query_snippet: "test".to_string(),
                embedding_dimension: 384,
                latency_ms: 30,
                collection_name: "col".to_string(),
                top_k: 5,
                result_count: 3,
                retries: 0,
                results: vec![],
            }),
            keyword_search: None,
            merge_dedup: None,
            reranking: None,
            final_answer: Some(FinalAnswerStep {
                model: "gpt-4".to_string(),
                max_retries: 3,
                chunks_in_context: 3,
                history_message_count: 0,
                history_token_estimate: 0,
                token_budget: 4000,
                total_tokens_estimate: 2000,
                latency_ms: 800,
                prompt_preview: "Prompt...".to_string(),
            }),
        };

        let json = serde_json::to_value(&dd).expect("serialization should succeed");
        assert_eq!(json["query_text"], "test query");
        assert!(json["multi_query"].is_null());
        assert!(json["hyde"].is_null());
        assert!(json["embedding_search"].is_object());
        assert_eq!(json["embedding_search"]["query_snippet"], "test");
        assert_eq!(json["embedding_search"]["embedding_dimension"], 384);
        assert!(json["keyword_search"].is_null());
        assert!(json["merge_dedup"].is_null());
        assert!(json["reranking"].is_null());
        assert!(json["final_answer"].is_object());
        assert_eq!(json["final_answer"]["model"], "gpt-4");
    }

    #[test]
    fn debug_data_new_stores_query_text() {
        let dd = DebugData::new("What is VEDO?");
        assert_eq!(dd.query_text, "What is VEDO?");
        assert!(dd.multi_query.is_none());
        assert!(dd.hyde.is_none());
        assert!(dd.embedding_search.is_none());
        assert!(dd.keyword_search.is_none());
        assert!(dd.merge_dedup.is_none());
        assert!(dd.reranking.is_none());
        assert!(dd.final_answer.is_none());
    }
}
