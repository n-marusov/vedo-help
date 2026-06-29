use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Incoming request to ask a question against a collection.
#[derive(Debug, Clone, Deserialize)]
pub struct QueryRequest {
    /// The collection to search.
    pub collection_id: Uuid,
    /// The user's question.
    pub query: String,
    /// Optional session for conversation continuity.
    pub session_id: Option<Uuid>,
    /// Enable debug data collection (admin users).
    #[serde(default)]
    pub debug: bool,
}

/// A single source reference returned in the SSE "sources" event.
#[derive(Debug, Clone, Serialize)]
pub struct SourceRef {
    pub document_id: Uuid,
    pub document_name: String,
    pub chunk_index: usize,
    pub text: String,
    pub relevance: f64,
    /// Which pipeline stage produced this source: "embedding", "keyword", or "reranked".
    pub stage: Option<String>,
    /// LLM rerank score (1-10) if this source went through reranking.
    pub rerank_score: Option<f64>,
    /// Rerank verdict: "брать" (accept) or "не брать" (reject).
    pub rerank_verdict: Option<String>,
    /// Rerank comment explaining the verdict.
    pub rerank_comment: Option<String>,
    /// Matched BM25 keywords for keyword-stage sources.
    pub keyword_matches: Option<Vec<String>>,
}

/// Internal event type yielded by the query stream before conversion to SSE.
#[derive(Debug, Clone, Serialize)]
pub struct StreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

/// Data included in the `done` SSE event payload.
#[derive(Debug, Clone, Serialize)]
pub struct DonePayload {
    #[serde(rename = "type")]
    pub event_type: String,
    pub user_message_id: Option<Uuid>,
    pub assistant_message_id: Option<Uuid>,
}

/// An SSE event emitted during a specific advanced RAG pipeline stage.
#[derive(Debug, Clone, Serialize)]
pub struct PipelineStageEvent {
    /// Pipeline stage identifier:
    /// "expanded_questions" | "hyde_docs" | "keyword_matches" |
    /// "merged_chunks" | "reranked_chunks" | "pipeline_metric"
    pub stage: String,
    /// Per-stage payload as raw JSON.
    pub data: serde_json::Value,
    /// Time taken for this stage in milliseconds.
    pub latency_ms: u64,
}

/// Aggregated timing metrics for the full advanced RAG pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineMetricStep {
    /// Total pipeline wall-clock time in ms.
    pub total_ms: u64,
    /// Multi-query expansion latency in ms.
    pub multi_query_ms: u64,
    /// HyDE generation latency in ms.
    pub hyde_ms: u64,
    /// Embedding search latency in ms (sum of all embedding searches).
    pub embedding_search_ms: u64,
    /// BM25 keyword search latency in ms.
    pub keyword_search_ms: u64,
    /// Merge + dedup latency in ms.
    pub merge_dedup_ms: u64,
    /// LLM reranking latency in ms.
    pub reranking_ms: u64,
    /// Final LLM answer generation latency in ms.
    pub final_answer_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_stage_serialization() {
        let event = PipelineStageEvent {
            stage: "expanded_questions".to_string(),
            data: serde_json::json!(["variant1", "variant2"]),
            latency_ms: 150,
        };
        let json = serde_json::to_value(&event).expect("serialization should succeed");
        assert_eq!(json["stage"], "expanded_questions");
        assert_eq!(json["data"], serde_json::json!(["variant1", "variant2"]));
        assert_eq!(json["latency_ms"], 150);
    }

    #[test]
    fn test_pipeline_stage_deserialization() {
        let stage_event = PipelineStageEvent {
            stage: "hyde_docs".to_string(),
            data: serde_json::json!({"query": "test", "doc": "hypothetical"}),
            latency_ms: 200,
        };
        let wrapped = StreamEvent {
            event_type: "pipeline_stage".to_string(),
            data: serde_json::to_value(&stage_event).unwrap(),
        };
        let json = serde_json::to_value(&wrapped).expect("serialization should succeed");
        assert_eq!(json["type"], "pipeline_stage");
        assert_eq!(json["data"]["stage"], "hyde_docs");
        assert_eq!(json["data"]["latency_ms"], 200);
    }

    #[test]
    fn test_source_ref_with_stage_metadata() {
        let source = SourceRef {
            document_id: Uuid::nil(),
            document_name: "test.pdf".to_string(),
            chunk_index: 3,
            text: "some content".to_string(),
            relevance: 0.95,
            stage: Some("reranked".to_string()),
            rerank_score: Some(8.0),
            rerank_verdict: Some("брать".to_string()),
            rerank_comment: Some("highly relevant".to_string()),
            keyword_matches: Some(vec!["rust".to_string(), "install".to_string()]),
        };
        let json = serde_json::to_value(&source).expect("serialization should succeed");
        assert_eq!(json["document_name"], "test.pdf");
        assert_eq!(json["relevance"], 0.95);
        assert_eq!(json["stage"], "reranked");
        assert_eq!(json["rerank_score"], 8.0);
        assert_eq!(json["rerank_verdict"], "брать");
        assert_eq!(json["rerank_comment"], "highly relevant");
        assert_eq!(
            json["keyword_matches"],
            serde_json::json!(["rust", "install"])
        );
    }
}
