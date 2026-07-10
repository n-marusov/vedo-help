//! Cross-encoder reranker for RAG pipeline.
//!
//! Reranks a set of retrieved chunks by scoring each chunk-query pair
//! using a cross-encoder model. Designed to run on CPU with ONNX Runtime
//! or similar local inference.
//!
//! ## Interface
//!
//! - `RerankItem` — a chunk with its original metadata
//! - `RerankResult` — a scored item after reranking
//! - `rerank_chunks` — main entry point for reranking
//!
//! ## Implementation Plan (Task 8)
//!
//! The actual cross-encoder model inference will replace the mock scorer
//! in this module. The interface and tests are defined here first (Task 3)
//! and implemented in Task 8.

use crate::shared::error::AppError;

/// A chunk to be reranked with its original position and text.
#[derive(Debug, Clone)]
pub struct RerankItem {
    /// Unique identifier for this chunk (e.g., UUID string).
    pub id: String,
    /// The chunk text content.
    pub text: String,
    /// Document name for source attribution.
    pub document_name: String,
    /// Original position/score before reranking.
    pub original_index: usize,
    /// Original similarity score (from vector/keyword search).
    pub original_score: f64,
}

/// A reranked chunk with its new score.
#[derive(Debug, Clone)]
pub struct RerankResult {
    /// The original chunk data.
    pub item: RerankItem,
    /// The reranker score (higher = more relevant).
    pub rerank_score: f64,
    /// Whether the reranker recommends keeping this chunk.
    pub verdict: RerankVerdict,
}

/// Verdict from the reranker for a single chunk.
#[derive(Debug, Clone, PartialEq)]
pub enum RerankVerdict {
    /// Keep this chunk in context.
    Keep,
    /// Discard this chunk as irrelevant.
    Discard,
}

/// Configuration for the reranker.
#[derive(Debug, Clone)]
pub struct RerankerConfig {
    /// Maximum number of chunks to keep after reranking.
    pub top_k: usize,
    /// Minimum rerank score to keep a chunk (0.0 to 1.0).
    pub min_score_threshold: f64,
}

impl Default for RerankerConfig {
    fn default() -> Self {
        Self {
            top_k: 5,
            min_score_threshold: 0.3,
        }
    }
}

/// Rerank a list of chunks using a cross-encoder model.
///
/// Takes a query and a list of candidate chunks, scores each pair,
/// and returns the top-k results sorted by relevance.
///
/// # Arguments
///
/// * `query` - The user's search query
/// * `items` - The candidate chunks to rerank
/// * `config` - Reranker configuration
///
/// # Returns
///
/// A vector of `RerankResult` items sorted by score (highest first),
/// filtered by the configured `top_k` and `min_score_threshold`.
///
/// # Errors
///
/// Returns `AppError` if the underlying model fails to load or run.
pub fn rerank_chunks(
    query: &str,
    items: &[RerankItem],
    config: &RerankerConfig,
) -> Result<Vec<RerankResult>, AppError> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    if query.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Query cannot be empty for reranking".to_string(),
        ));
    }

    // Score each item
    let mut results: Vec<RerankResult> = items
        .iter()
        .map(|item| {
            let score = score_pair(query, item);
            let verdict = if score >= config.min_score_threshold {
                RerankVerdict::Keep
            } else {
                RerankVerdict::Discard
            };
            RerankResult {
                item: item.clone(),
                rerank_score: score,
                verdict,
            }
        })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| {
        b.rerank_score
            .partial_cmp(&a.rerank_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Filter out discarded items first
    results.retain(|r| r.verdict == RerankVerdict::Keep);

    // Apply top_k
    results.truncate(config.top_k);

    Ok(results)
}

/// Score a single query-chunk pair using a cross-encoder model.
///
/// Currently uses a simple keyword overlap heuristic as a placeholder.
/// In Task 8 this will be replaced with actual ONNX cross-encoder inference.
fn score_pair(query: &str, item: &RerankItem) -> f64 {
    if query.is_empty() || item.text.is_empty() {
        return 0.0;
    }

    // Simple heuristic: count query word overlap in chunk text
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    let text_lower = item.text.to_lowercase();

    if query_words.is_empty() {
        return 0.0;
    }

    let matched = query_words
        .iter()
        .filter(|word| text_lower.contains(*word))
        .count();

    matched as f64 / query_words.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: &str, text: &str, score: f64) -> RerankItem {
        RerankItem {
            id: id.to_string(),
            text: text.to_string(),
            document_name: "test.md".to_string(),
            original_index: 0,
            original_score: score,
        }
    }

    // ── Basic reranking tests ──

    #[test]
    fn test_rerank_empty_items() {
        let result = rerank_chunks("test", &[], &RerankerConfig::default()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_rerank_empty_query() {
        let items = vec![make_item("1", "some content", 0.8)];
        let result = rerank_chunks("", &items, &RerankerConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_rerank_single_item() {
        let items = vec![make_item("1", "Rust is a systems language", 0.9)];
        let result = rerank_chunks("Rust", &items, &RerankerConfig::default()).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].rerank_score > 0.0);
        assert_eq!(result[0].verdict, RerankVerdict::Keep);
    }

    #[test]
    fn test_rerank_sorts_by_score() {
        let items = vec![
            make_item("1", "Python is a programming language", 0.9),
            make_item("2", "Rust is a systems programming language", 0.8),
            make_item("3", "JavaScript runs in the browser", 0.7),
        ];
        let result = rerank_chunks("Rust programming", &items, &RerankerConfig::default()).unwrap();
        // Most relevant should be first
        assert!(!result.is_empty());
        // The "Rust" item should have highest score
        assert_eq!(result[0].item.id, "2");
        // Scores should be descending
        for i in 1..result.len() {
            assert!(result[i - 1].rerank_score >= result[i].rerank_score);
        }
    }

    #[test]
    fn test_rerank_respects_top_k() {
        let items: Vec<RerankItem> = (0..10)
            .map(|i| {
                make_item(
                    &format!("{i}"),
                    &format!("content about Rust programming item {i}"),
                    0.5,
                )
            })
            .collect();
        let config = RerankerConfig {
            top_k: 3,
            ..Default::default()
        };
        let result = rerank_chunks("Rust programming", &items, &config).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_rerank_discards_below_threshold() {
        let items = vec![
            make_item("1", "Rust is fast", 0.9),
            make_item("2", "The weather is nice today", 0.5),
        ];
        let config = RerankerConfig {
            top_k: 10,
            min_score_threshold: 0.5,
            ..Default::default()
        };
        let result = rerank_chunks("Rust", &items, &config).unwrap();
        // Item 1 has "Rust" match (score 1.0), item 2 has no match (score 0.0)
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].item.id, "1");
        assert_eq!(result[0].verdict, RerankVerdict::Keep);
    }

    // ── score_pair tests ──

    #[test]
    fn test_score_pair_exact_match() {
        let item = make_item("1", "Rust programming language", 0.0);
        let score = score_pair("Rust programming", &item);
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_score_pair_partial_match() {
        let item = make_item("1", "Rust is a systems language", 0.0);
        let score = score_pair("Rust programming", &item);
        // "Rust" matches, "programming" doesn't → 0.5
        assert!((score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_score_pair_no_match() {
        let item = make_item("1", "Python is interpreted", 0.0);
        let score = score_pair("Rust systems", &item);
        // Neither "Rust" nor "systems" appear → 0.0
        assert!(score.abs() < 0.001);
    }

    #[test]
    fn test_score_pair_empty_query() {
        let item = make_item("1", "some content", 0.0);
        let score = score_pair("", &item);
        assert!(score.abs() < 0.001);
    }

    #[test]
    fn test_score_pair_empty_text() {
        let item = make_item("1", "", 0.0);
        let score = score_pair("test", &item);
        assert!(score.abs() < 0.001);
    }

    #[test]
    fn test_score_pair_case_insensitive() {
        let item = make_item("1", "RUST is a systems language", 0.0);
        let score = score_pair("rust", &item);
        // Case-insensitive match
        assert!((score - 1.0).abs() < 0.001);
    }

    // ── RerankVerdict tests ──

    #[test]
    fn test_verdict_keep_when_above_threshold() {
        let items = vec![make_item("1", "Rust programming", 0.9)];
        let config = RerankerConfig {
            top_k: 10,
            min_score_threshold: 0.5,
            ..Default::default()
        };
        let result = rerank_chunks("Rust", &items, &config).unwrap();
        assert_eq!(result[0].verdict, RerankVerdict::Keep);
    }

    #[test]
    fn test_verdict_discard_when_below_threshold() {
        let items = vec![make_item("1", "unrelated content", 0.1)];
        let config = RerankerConfig {
            top_k: 10,
            min_score_threshold: 0.5,
            ..Default::default()
        };
        let result = rerank_chunks("Rust", &items, &config).unwrap();
        // The item has no keyword overlap, so score is 0.0, and it's filtered out
        assert!(result.is_empty());
    }

    // ── Edge cases ──

    #[test]
    fn test_rerank_multiple_identical_scores() {
        let items = vec![
            make_item("1", "Rust", 0.9),
            make_item("2", "Rust", 0.8),
            make_item("3", "Rust", 0.7),
        ];
        let result = rerank_chunks("Rust", &items, &RerankerConfig::default()).unwrap();
        // All items have the same content, so all should match "Rust"
        assert_eq!(result.len(), 3);
        for r in &result {
            assert_eq!(r.verdict, RerankVerdict::Keep);
        }
    }

    #[test]
    fn test_rerank_with_special_characters() {
        let items = vec![
            make_item("1", "C++ is fast", 0.9),
            make_item("2", "C# is modern", 0.8),
        ];
        let result = rerank_chunks("C++", &items, &RerankerConfig::default()).unwrap();
        // Split on whitespace: "C++" is one token
        assert!(!result.is_empty());
    }

    #[test]
    fn test_rerank_config_defaults() {
        let config = RerankerConfig::default();
        assert_eq!(config.top_k, 5);
        assert!((config.min_score_threshold - 0.3).abs() < 0.001);
    }
}
