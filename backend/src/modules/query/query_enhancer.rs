//! Query enhancement module for the advanced RAG pipeline.
//!
//! Provides multi-query expansion and HyDE (Hypothetical Document Embeddings)
//! generation.

use crate::shared::llm::LlmClient;
use std::time::Instant;

/// Parsed result of multi-query expansion.
#[derive(Debug, Clone)]
pub struct MultiQueryResult {
    pub variants: Vec<String>,
    pub latency_ms: u64,
}

/// Single HyDE result for one query variant.
#[derive(Debug, Clone)]
pub struct HydeGenerationResult {
    pub query: String,
    pub hypothetical_doc: String,
    pub latency_ms: u64,
}

/// Generate multi-query variants by asking the LLM to rephrase the original question.
///
/// Calls `llm_client.query_single` with a system prompt requesting `count` variants.
/// Falls back to `vec![original_query]` on any failure.
pub async fn generate_multi_queries(
    llm_client: &LlmClient,
    query: &str,
    count: usize,
) -> MultiQueryResult {
    let start = Instant::now();

    let system_prompt = format!(
        "Ты — помощник, который переформулирует вопрос пользователя в {count} различных вариантов. \
         Каждый вариант должен отражать тот же информационный запрос, но с другой формулировкой. \
         Верни ТОЛЬКО JSON-массив строк без пояснений: [\"variant1\", \"variant2\", ...]"
    );

    let response = llm_client
        .query_single(&system_prompt, query)
        .await
        .unwrap_or_default();

    let variants = parse_multi_query_response(&response, query);
    let latency_ms = start.elapsed().as_millis() as u64;

    tracing::debug!(
        component = "query/query_enhancer",
        variant_count = variants.len(),
        requested = count,
        latency_ms,
        "multi_query.generated"
    );

    if variants.len() < count {
        tracing::warn!(
            component = "query/query_enhancer",
            expected = count,
            actual = variants.len(),
            "multi_query.low_count"
        );
    }

    MultiQueryResult {
        variants,
        latency_ms,
    }
}

/// Generate a hypothetical document for the given query using HyDE.
///
/// Calls `llm_client.query_single` with a HyDE system prompt.
/// Returns the hypothetical document text and latency.
/// On failure, returns a fallback empty string.
pub async fn generate_hyde(llm_client: &LlmClient, query: &str) -> HydeGenerationResult {
    let start = Instant::now();

    let system_prompt = "Ты — эксперт по документации. Напиши гипотетический ответ на вопрос, \
         который содержит ключевые термины и концепции, ожидаемые в документации. \
         Не используй цитаты — просто опиши, как мог бы выглядеть ответ.";

    let hypothetical_doc = llm_client
        .query_single(system_prompt, query)
        .await
        .unwrap_or_default();

    let latency_ms = start.elapsed().as_millis() as u64;

    tracing::debug!(
        component = "query/query_enhancer",
        query_len = query.len(),
        doc_len = hypothetical_doc.len(),
        latency_ms,
        "hyde.generated"
    );

    HydeGenerationResult {
        query: query.to_string(),
        hypothetical_doc,
        latency_ms,
    }
}

/// Parse the LLM response for multi-query expansion.
///
/// Expects a JSON array of strings: `["variant1", "variant2", ...]`
/// Falls back to `vec![original_query]` on parse failure.
pub fn parse_multi_query_response(response: &str, original_query: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(response)
        .map(|variants| {
            if variants.is_empty() {
                vec![original_query.to_string()]
            } else {
                variants
            }
        })
        .unwrap_or_else(|_| vec![original_query.to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_multi_query_response() {
        let response =
            r#"["how to install rust?", "rust installation guide", "setting up rust environment"]"#;
        let variants = parse_multi_query_response(response, "how to install");
        assert_eq!(variants.len(), 3);
        assert_eq!(variants[0], "how to install rust?");
        assert_eq!(variants[1], "rust installation guide");
        assert_eq!(variants[2], "setting up rust environment");
    }

    #[test]
    fn test_parse_malformed_json_falls_back() {
        let response = "this is not valid json at all";
        let variants = parse_multi_query_response(response, "original query");
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0], "original query");
    }

    #[test]
    fn test_parse_single_variant() {
        let response = r#"["only one variant"]"#;
        let variants = parse_multi_query_response(response, "original query");
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0], "only one variant");
    }

    #[test]
    fn test_parse_empty_array_falls_back() {
        let response = r#"[]"#;
        let variants = parse_multi_query_response(response, "original query");
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0], "original query");
    }
}
