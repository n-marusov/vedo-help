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

/// Multi-query expansion step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiQueryStep {
    pub original_query: String,
    pub variants: Vec<String>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydeResult {
    pub query: String,
    pub hypothetical_doc: String,
    pub latency_ms: u64,
}

/// Hypothetical Document Embeddings step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydeStep {
    pub per_query: Vec<HydeResult>,
}

/// Keyword/BM25 search step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordSearchStep {
    pub query_tokens: Vec<String>,
    pub total_matches: usize,
    pub results: Vec<SearchResultItem>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeSourceBreakdown {
    pub vector_chunks: usize,
    pub keyword_chunks: usize,
}

/// Merge and deduplication step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeDedupStep {
    pub input_chunks: usize,
    pub after_dedup: usize,
    pub source_breakdown: MergeSourceBreakdown,
    pub results: Vec<SearchResultItem>,
    /// Chunk IDs that appeared in BOTH vector and BM25 search results
    pub deduped_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    pub chunk_id: String,
    pub document_name: String,
    pub chunk_index: usize,
    pub text_snippet: String,
    pub score: f64,
    pub verdict: String,
    pub comment: String,
}

/// Re-ranking step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankingStep {
    pub input_count: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub results: Vec<RerankResult>,
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
    fn merge_dedup_step_with_results() {
        let step = MergeDedupStep {
            input_chunks: 10,
            after_dedup: 7,
            source_breakdown: MergeSourceBreakdown {
                vector_chunks: 5,
                keyword_chunks: 5,
            },
            results: vec![
                SearchResultItem {
                    chunk_id: "c1".to_string(),
                    document_name: "doc-a.pdf".to_string(),
                    chunk_index: 2,
                    score: 0.91,
                    text_snippet: "First chunk text".to_string(),
                },
                SearchResultItem {
                    chunk_id: "c2".to_string(),
                    document_name: "doc-b.pdf".to_string(),
                    chunk_index: 5,
                    score: 0.72,
                    text_snippet: "Second chunk text".to_string(),
                },
            ],
            deduped_ids: vec![],
        };
        assert_eq!(step.input_chunks, 10);
        assert_eq!(step.after_dedup, 7);
        assert_eq!(step.results.len(), 2);
        assert_eq!(step.results[0].document_name, "doc-a.pdf");
        assert_eq!(step.results[0].chunk_index, 2);
        assert_eq!(step.results[0].score, 0.91);
        assert_eq!(step.results[1].chunk_index, 5);
        assert_eq!(step.results[1].text_snippet, "Second chunk text");

        // Verify serialization includes results
        let json = serde_json::to_value(&step).expect("serialization should succeed");
        assert!(json["results"].is_array());
        assert_eq!(json["results"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn rerank_result_with_text() {
        let result = RerankResult {
            chunk_id: "chunk-42".to_string(),
            document_name: "guide.pdf".to_string(),
            chunk_index: 7,
            text_snippet: "Relevant passage about configuration".to_string(),
            score: 1.0,
            verdict: "брать".to_string(),
            comment: "Contains relevant info".to_string(),
        };

        assert_eq!(result.document_name, "guide.pdf");
        assert_eq!(result.chunk_index, 7);
        assert_eq!(result.text_snippet, "Relevant passage about configuration");

        let json = serde_json::to_value(&result).expect("serialization should succeed");
        assert_eq!(json["document_name"], "guide.pdf");
        assert_eq!(json["chunk_index"], 7);
        assert_eq!(json["text_snippet"], "Relevant passage about configuration");
    }

    #[test]
    fn embedding_search_step_dynamic_dimension() {
        let step = EmbeddingSearchStep {
            query_snippet: "custom query".to_string(),
            embedding_dimension: 768,
            latency_ms: 50,
            collection_name: "my-collection".to_string(),
            top_k: 10,
            result_count: 8,
            retries: 1,
            results: vec![],
        };

        assert_eq!(step.embedding_dimension, 768);
        assert_ne!(step.embedding_dimension, 384);

        let json = serde_json::to_value(&step).expect("serialization should succeed");
        assert_eq!(json["embedding_dimension"], 768);
    }

    #[test]
    fn final_answer_step_all_fields() {
        let step = FinalAnswerStep {
            model: "claude-v3".to_string(),
            max_retries: 5,
            chunks_in_context: 10,
            history_message_count: 4,
            history_token_estimate: 800,
            token_budget: 8000,
            total_tokens_estimate: 4200,
            latency_ms: 2500,
            prompt_preview: "[{\"role\":\"system\",\"content\":\"Answer...\"}]".to_string(),
        };

        assert_eq!(step.max_retries, 5);
        assert_eq!(step.history_message_count, 4);
        assert_eq!(step.token_budget, 8000);
        assert!(
            !step.prompt_preview.is_empty(),
            "prompt_preview should be a non-empty string"
        );
    }

    #[test]
    fn debug_watermark_check() {
        let dd = DebugData {
            query_text: "watermark test".to_string(),
            multi_query: Some(MultiQueryStep {
                original_query: "test".to_string(),
                variants: vec!["var1".to_string()],
                latency_ms: 10,
            }),
            hyde: Some(HydeStep {
                per_query: vec![HydeResult {
                    query: "test".to_string(),
                    hypothetical_doc: "hypo doc".to_string(),
                    latency_ms: 5,
                }],
            }),
            embedding_search: Some(EmbeddingSearchStep {
                query_snippet: "embed query".to_string(),
                embedding_dimension: 384,
                latency_ms: 20,
                collection_name: "col".to_string(),
                top_k: 5,
                result_count: 3,
                retries: 0,
                results: vec![SearchResultItem {
                    chunk_id: "c1".to_string(),
                    document_name: "doc".to_string(),
                    chunk_index: 1,
                    score: 0.9,
                    text_snippet: "snippet".to_string(),
                }],
            }),
            keyword_search: Some(KeywordSearchStep {
                query_tokens: vec!["test".to_string()],
                total_matches: 2,
                results: vec![SearchResultItem {
                    chunk_id: "c2".to_string(),
                    document_name: "doc2".to_string(),
                    chunk_index: 2,
                    score: 0.0,
                    text_snippet: "kw snippet".to_string(),
                }],
                latency_ms: 15,
            }),
            merge_dedup: Some(MergeDedupStep {
                input_chunks: 5,
                after_dedup: 3,
                source_breakdown: MergeSourceBreakdown {
                    vector_chunks: 3,
                    keyword_chunks: 2,
                },
                results: vec![SearchResultItem {
                    chunk_id: "c3".to_string(),
                    document_name: "doc-c.txt".to_string(),
                    chunk_index: 2,
                    score: 0.65,
                    text_snippet: "def sort(arr):...".to_string(),
                }],
                deduped_ids: vec![],
            }),
            reranking: Some(RerankingStep {
                input_count: 3,
                accepted: 2,
                rejected: 1,
                results: vec![RerankResult {
                    chunk_id: "c3".to_string(),
                    document_name: "doc3".to_string(),
                    chunk_index: 3,
                    text_snippet: "reranked snippet".to_string(),
                    score: 1.0,
                    verdict: "брать".to_string(),
                    comment: "good".to_string(),
                }],
            }),
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

        // Serialize and deserialize — no data loss
        let json = serde_json::to_value(&dd).expect("serialization should succeed");
        let deserialized: DebugData =
            serde_json::from_value(json.clone()).expect("deserialization should succeed");

        assert_eq!(deserialized.query_text, "watermark test");
        assert!(deserialized.multi_query.is_some());
        assert!(deserialized.hyde.is_some());
        assert!(deserialized.embedding_search.is_some());
        assert!(deserialized.keyword_search.is_some());
        assert!(deserialized.merge_dedup.is_some());
        assert!(deserialized.reranking.is_some());
        assert!(deserialized.final_answer.is_some());

        // Verify all new fields survive roundtrip
        let merge_dedup = deserialized.merge_dedup.unwrap();
        assert_eq!(merge_dedup.results.len(), 1);
        assert_eq!(merge_dedup.results[0].chunk_id, "c3");

        let reranking = deserialized.reranking.unwrap();
        assert_eq!(reranking.results[0].document_name, "doc3");
        assert_eq!(reranking.results[0].chunk_index, 3);
        assert_eq!(reranking.results[0].text_snippet, "reranked snippet");

        let final_answer = deserialized.final_answer.unwrap();
        assert_eq!(final_answer.prompt_preview, "Prompt...");
        assert!(!final_answer.prompt_preview.is_empty());

        // Verify JSON structure matches expected key names
        assert!(json["merge_dedup"]["results"].is_array());
        assert!(json["reranking"]["results"][0]["document_name"].is_string());
        assert!(json["reranking"]["results"][0]["chunk_index"].is_number());
        assert!(json["reranking"]["results"][0]["text_snippet"].is_string());
    }
}
