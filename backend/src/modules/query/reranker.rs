/// LLM reranking module for the advanced RAG pipeline.
///
/// Uses the LLM to score and filter chunks by relevance to the user's query.
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::modules::query::hybrid_service::MergedChunk;
use crate::shared::error::AppError;
use crate::shared::llm::LlmClient;

/// A single chunk reranking result with LLM verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankVerdict {
    pub score: u32,
    pub verdict: String,
    pub comment: String,
}

/// Result of reranking a single chunk, including the chunk data and the LLM verdict.
#[derive(Debug, Clone)]
pub struct RerankedChunk {
    pub chunk: MergedChunk,
    pub verdict: RerankVerdict,
}

/// Rerank merged chunks by asking the LLM to score each one.
///
/// For each chunk, calls the LLM with a reranking prompt and parses the JSON verdict.
/// Returns chunks sorted by score descending, limited to `top_k`.
pub async fn rerank_chunks(
    llm_client: &LlmClient,
    model: &str,
    query: &str,
    chunks: &[MergedChunk],
    top_k: usize,
) -> Result<Vec<RerankedChunk>, AppError> {
    let start = Instant::now();
    let mut reranked = Vec::with_capacity(chunks.len());

    tracing::info!(
        component = "query/reranker",
        chunk_count = chunks.len(),
        query_length = query.len(),
        model = %model,
        top_k,
        "rerank.start"
    );

    for chunk in chunks {
        let chunk_start = Instant::now();

        let user_prompt = format!(
            "Вопрос: {question}\nФрагмент: {chunk_text}",
            question = query,
            chunk_text = chunk.text
        );

        let system_prompt = "Оцени релевантность следующего фрагмента документа для ответа на вопрос пользователя.\n\
             Верни ТОЛЬКО JSON: {\"score\": <int 1-10>, \"verdict\": \"брать\"|\"не брать\", \"comment\": \"<brief reason>\"}".to_string();

        let response = llm_client
            .query_single_with_model(&system_prompt, &user_prompt, model)
            .await?;

        let verdict = parse_rerank_response(&response);

        let chunk_latency = chunk_start.elapsed().as_millis() as u64;

        tracing::debug!(
            component = "query/reranker",
            chunk_id = %chunk.chunk_id,
            score = verdict.score,
            verdict = %verdict.verdict,
            comment = %verdict.comment,
            latency_ms = chunk_latency,
            "rerank.chunk_done"
        );

        if verdict.score < 1 || verdict.score > 10 {
            tracing::warn!(
                component = "query/reranker",
                chunk_id = %chunk.chunk_id,
                original_score = verdict.score,
                clamped = verdict.score.clamp(1, 10),
                "rerank.score_out_of_range"
            );
        }

        reranked.push(RerankedChunk {
            chunk: chunk.clone(),
            verdict,
        });
    }

    // Sort by score descending (worst score first = 1, best = 10)
    reranked.sort_by_key(|b| std::cmp::Reverse(b.verdict.score));

    // Accept only "брать" verdicts, up to top_k
    let accepted: Vec<RerankedChunk> = reranked
        .into_iter()
        .filter(|r| r.verdict.verdict == "брать")
        .take(top_k)
        .collect();

    let total_latency = start.elapsed().as_millis() as u64;

    tracing::info!(
        component = "query/reranker",
        total_chunks = chunks.len(),
        accepted = accepted.len(),
        rejected = chunks.len() - accepted.len(),
        total_latency_ms = total_latency,
        "rerank.complete"
    );

    Ok(accepted)
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
