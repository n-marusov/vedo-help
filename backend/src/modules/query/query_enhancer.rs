/// Query enhancement module for the advanced RAG pipeline.
///
/// Provides multi-query expansion and HyDE (Hypothetical Document Embeddings)
/// generation. Implementation follows in Phase 4.

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
