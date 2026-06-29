/// LLM reranking module for the advanced RAG pipeline.
///
/// Uses the LLM to score and filter chunks by relevance to the user's query.
/// Implementation follows in Phase 5.
use serde::{Deserialize, Serialize};

/// A single chunk reranking result with LLM verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankVerdict {
    pub score: u32,
    pub verdict: String,
    pub comment: String,
}

/// Parse the LLM rerank response JSON into a `RerankVerdict`.
///
/// Expected format: `{"score": <int 1-10>, "verdict": "брать"|"не брать", "comment": "<reason>"}`
/// Falls back to `RerankVerdict { score: 1, verdict: "не брать".to_string(), comment: String::new() }`
/// on parse failure. Scores outside 1-10 are clamped.
pub fn parse_rerank_response(response: &str) -> RerankVerdict {
    match serde_json::from_str::<serde_json::Value>(response) {
        Ok(val) => {
            let score = val.get("score").and_then(|s| s.as_u64()).unwrap_or(1) as u32;
            let verdict = val
                .get("verdict")
                .and_then(|v| v.as_str())
                .unwrap_or("не брать")
                .to_string();
            let comment = val
                .get("comment")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let clamped_score = score.clamp(1, 10);
            RerankVerdict {
                score: clamped_score,
                verdict,
                comment,
            }
        }
        Err(_) => RerankVerdict {
            score: 1,
            verdict: "не брать".to_string(),
            comment: String::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_verdict() {
        let response = r#"{"score": 8, "verdict": "брать", "comment": "relevant"}"#;
        let result = parse_rerank_response(response);
        assert_eq!(result.score, 8);
        assert_eq!(result.verdict, "брать");
        assert_eq!(result.comment, "relevant");
    }

    #[test]
    fn test_parse_reject_verdict() {
        let response = r#"{"score": 2, "verdict": "не брать"}"#;
        let result = parse_rerank_response(response);
        assert_eq!(result.score, 2);
        assert_eq!(result.verdict, "не брать");
    }

    #[test]
    fn test_parse_malformed() {
        let response = "this is not valid json at all";
        let result = parse_rerank_response(response);
        assert_eq!(result.score, 1);
        assert_eq!(result.verdict, "не брать");
        assert_eq!(result.comment, "");
    }

    #[test]
    fn test_parse_score_out_of_range() {
        let response = r#"{"score": 15, "verdict": "брать", "comment": "too high"}"#;
        let result = parse_rerank_response(response);
        assert_eq!(result.score, 10);
        assert_eq!(result.verdict, "брать");
    }
}
