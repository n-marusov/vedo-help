use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// 7-step RAG pipeline debug data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebugData {
    pub query_text: String,
    /// Multi-query expansion step
    pub multi_query: Option<MultiQueryStep>,
    /// Hypothetical Document Embeddings step
    pub hyde: Option<HydeStep>,
    /// Active step — embedding/vector search results
    pub embedding_search: Option<EmbeddingSearchStep>,
    /// Keyword/BM25 search step
    pub keyword_search: Option<KeywordSearchStep>,
    /// Merge and deduplication step
    pub merge_dedup: Option<MergeDedupStep>,
    /// Re-ranking step
    pub reranking: Option<RerankingStep>,
    /// Active step — final LLM answer metadata
    pub final_answer: Option<FinalAnswerStep>,
}

impl DebugData {
    /// Create a new DebugData with the given query text.
    /// All steps default to `None`.
    pub fn new(query_text: impl Into<String>) -> Self {
        let qt = query_text.into();
        debug!(
            component = "query/debug_models",
            query_text = %qt,
            "Constructing DebugData"
        );
        Self {
            query_text: qt,
            ..Default::default()
        }
    }
}

/// Multi-query expansion step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiQueryStep {
    pub original_query: String,
    pub variants: Vec<String>,
    pub latency_ms: u64,
}

impl MultiQueryStep {
    pub fn new(original_query: String, variants: Vec<String>, latency_ms: u64) -> Self {
        debug!(
            component = "query/debug_models",
            variant_count = variants.len(),
            latency_ms,
            "Constructing MultiQueryStep"
        );
        Self {
            original_query,
            variants,
            latency_ms,
        }
    }
}

/// Hypothetical Document Embeddings step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydeStep {
    pub per_query: Vec<HydeResult>,
}

impl HydeStep {
    pub fn new(per_query: Vec<HydeResult>) -> Self {
        debug!(
            component = "query/debug_models",
            result_count = per_query.len(),
            "Constructing HydeStep"
        );
        Self { per_query }
    }
}

/// A single HyDE result for one query variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydeResult {
    pub query: String,
    pub hypothetical_doc: String,
    pub latency_ms: u64,
}

/// Keyword/BM25 search step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordSearchStep {
    pub query_tokens: Vec<String>,
    pub total_matches: usize,
    pub results: Vec<SearchResultItem>,
    pub latency_ms: u64,
}

impl KeywordSearchStep {
    pub fn new(
        query_tokens: Vec<String>,
        total_matches: usize,
        results: Vec<SearchResultItem>,
        latency_ms: u64,
    ) -> Self {
        debug!(
            component = "query/debug_models",
            token_count = query_tokens.len(),
            total_matches,
            result_count = results.len(),
            latency_ms,
            "Constructing KeywordSearchStep"
        );
        Self {
            query_tokens,
            total_matches,
            results,
            latency_ms,
        }
    }
}

/// Merge and deduplication step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeDedupStep {
    pub input_chunks: usize,
    pub after_dedup: usize,
    pub source_breakdown: MergeSourceBreakdown,
}

impl MergeDedupStep {
    pub fn new(
        input_chunks: usize,
        after_dedup: usize,
        source_breakdown: MergeSourceBreakdown,
    ) -> Self {
        debug!(
            component = "query/debug_models",
            input_chunks,
            after_dedup,
            dedup_removed = input_chunks.saturating_sub(after_dedup),
            "Constructing MergeDedupStep"
        );
        Self {
            input_chunks,
            after_dedup,
            source_breakdown,
        }
    }
}

/// Breakdown of which sources contributed to the merged result set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeSourceBreakdown {
    pub vector_chunks: usize,
    pub keyword_chunks: usize,
}

/// Re-ranking step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankingStep {
    pub input_count: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub results: Vec<RerankResult>,
}

impl RerankingStep {
    pub fn new(
        input_count: usize,
        accepted: usize,
        rejected: usize,
        results: Vec<RerankResult>,
    ) -> Self {
        debug!(
            component = "query/debug_models",
            input_count, accepted, rejected, "Constructing RerankingStep"
        );
        Self {
            input_count,
            accepted,
            rejected,
            results,
        }
    }
}

/// A single reranking result with LLM verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    pub chunk_id: String,
    pub score: u32,
    pub verdict: String,
    pub comment: String,
}

impl RerankResult {
    pub fn new(chunk_id: String, score: u32, verdict: String, comment: String) -> Self {
        if !(1..=10).contains(&score) {
            warn!(
                component = "query/debug_models",
                chunk_id = %chunk_id,
                score,
                "RerankResult score is outside 1-10 range"
            );
        }
        let clamped_score = score.clamp(1, 10);
        if clamped_score != score {
            warn!(
                component = "query/debug_models",
                original_score = score,
                clamped_score,
                chunk_id = %chunk_id,
                "RerankResult score clamped to 1-10 range"
            );
        }
        debug!(
            component = "query/debug_models",
            chunk_id = %chunk_id,
            score = clamped_score,
            verdict = %verdict,
            "Constructing RerankResult"
        );
        Self {
            chunk_id,
            score: clamped_score,
            verdict,
            comment,
        }
    }
}

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

    #[test]
    fn multi_query_step_constructs() {
        let step = MultiQueryStep::new(
            "how to install".to_string(),
            vec![
                "how to install".to_string(),
                "installation guide".to_string(),
                "setup instructions".to_string(),
            ],
            150,
        );
        assert_eq!(step.original_query, "how to install");
        assert_eq!(step.variants.len(), 3);
        assert_eq!(step.latency_ms, 150);
    }

    #[test]
    fn hyde_step_constructs() {
        let step = HydeStep::new(vec![HydeResult {
            query: "how to install".to_string(),
            hypothetical_doc: "To install the software, run...".to_string(),
            latency_ms: 200,
        }]);
        assert_eq!(step.per_query.len(), 1);
        assert_eq!(step.per_query[0].query, "how to install");
    }

    #[test]
    fn keyword_search_step_constructs() {
        let step = KeywordSearchStep::new(
            vec!["how".to_string(), "install".to_string()],
            10,
            vec![SearchResultItem {
                chunk_id: "chunk-1".to_string(),
                document_name: "docs.pdf".to_string(),
                chunk_index: 1,
                score: 2.5,
                text_snippet: "Install guide".to_string(),
            }],
            50,
        );
        assert_eq!(step.query_tokens.len(), 2);
        assert_eq!(step.total_matches, 10);
        assert_eq!(step.latency_ms, 50);
    }

    #[test]
    fn merge_dedup_step_constructs() {
        let step = MergeDedupStep::new(
            10,
            7,
            MergeSourceBreakdown {
                vector_chunks: 5,
                keyword_chunks: 2,
            },
        );
        assert_eq!(step.input_chunks, 10);
        assert_eq!(step.after_dedup, 7);
        assert_eq!(step.source_breakdown.vector_chunks, 5);
        assert_eq!(step.source_breakdown.keyword_chunks, 2);
    }

    #[test]
    fn reranking_step_constructs() {
        let step = RerankingStep::new(
            7,
            3,
            4,
            vec![RerankResult::new(
                "chunk-1".to_string(),
                8,
                "брать".to_string(),
                "highly relevant".to_string(),
            )],
        );
        assert_eq!(step.input_count, 7);
        assert_eq!(step.accepted, 3);
        assert_eq!(step.rejected, 4);
        assert_eq!(step.results.len(), 1);
        assert_eq!(step.results[0].score, 8);
    }

    #[test]
    fn rerank_result_clamps_score() {
        let result = RerankResult::new(
            "chunk-1".to_string(),
            15,
            "брать".to_string(),
            "too high".to_string(),
        );
        assert_eq!(result.score, 10);
    }

    #[test]
    fn rerank_result_default_score_ok() {
        let result = RerankResult::new(
            "chunk-1".to_string(),
            5,
            "брать".to_string(),
            "ok".to_string(),
        );
        assert_eq!(result.score, 5);
    }
}
