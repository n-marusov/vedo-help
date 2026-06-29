/// Hybrid search orchestrator module.
///
/// Merges and deduplicates results from Chroma vector search and BM25 keyword search.
/// Implementation follows in Phase 3.
use crate::shared::bm25::Bm25Result;
use crate::shared::types::ChromaResult;

/// A merged chunk from vector and/or keyword search.
#[derive(Debug, Clone)]
pub struct MergedChunk {
    pub chunk_id: String,
    pub text: String,
    pub document_id: String,
    pub document_name: String,
    pub chunk_index: usize,
    /// Score from Chroma vector search (if present).
    pub vector_score: Option<f64>,
    /// Score from BM25 keyword search (if present).
    pub keyword_score: Option<f64>,
    /// How many search sources matched this chunk.
    pub match_count: usize,
}

/// Merge and deduplicate results from Chroma vector search and BM25 keyword search.
///
/// Chroma results take priority (appear first), followed by BM25 results.
/// Duplicates are removed by `chunk_id` — the first occurrence (from Chroma) is kept.
pub fn merge_and_dedup(
    chroma_results: Vec<ChromaResult>,
    bm25_results: Vec<Bm25Result>,
) -> Vec<MergedChunk> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut merged: Vec<MergedChunk> = Vec::new();

    for r in chroma_results {
        if seen.insert(r.id.clone()) {
            merged.push(MergedChunk {
                chunk_id: r.id,
                text: r.text,
                document_id: r.document_id,
                document_name: String::new(),
                chunk_index: r.chunk_index,
                vector_score: Some(r.score),
                keyword_score: None,
                match_count: 1,
            });
        }
    }

    for r in bm25_results {
        if seen.insert(r.doc_id.clone()) {
            merged.push(MergedChunk {
                chunk_id: r.doc_id,
                text: r.text,
                document_id: String::new(),
                document_name: String::new(),
                chunk_index: 0,
                vector_score: None,
                keyword_score: Some(r.score),
                match_count: 1,
            });
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chroma_result(chunk_id: &str, score: f64) -> ChromaResult {
        ChromaResult {
            id: chunk_id.to_string(),
            document_id: "doc-1".to_string(),
            chunk_index: 0,
            score,
            text: format!("content of {chunk_id}"),
        }
    }

    fn make_bm25_result(doc_id: &str, score: f64) -> Bm25Result {
        Bm25Result {
            doc_id: doc_id.to_string(),
            text: format!("content of {doc_id}"),
            score,
        }
    }

    #[test]
    fn test_merge_no_overlap() {
        let chroma = vec![
            make_chroma_result("chunk-a", 0.95),
            make_chroma_result("chunk-b", 0.90),
        ];
        let bm25 = vec![
            make_bm25_result("chunk-c", 8.5),
            make_bm25_result("chunk-d", 7.2),
        ];
        let merged = merge_and_dedup(chroma, bm25);
        assert_eq!(merged.len(), 4);
        // All should be unique
        let ids: Vec<&str> = merged.iter().map(|m| m.chunk_id.as_str()).collect();
        assert_eq!(ids, vec!["chunk-a", "chunk-b", "chunk-c", "chunk-d"]);
    }

    #[test]
    fn test_merge_with_duplicates() {
        let chroma = vec![
            make_chroma_result("chunk-a", 0.95),
            make_chroma_result("chunk-b", 0.90),
        ];
        let bm25 = vec![
            make_bm25_result("chunk-a", 9.0), // duplicate
            make_bm25_result("chunk-c", 7.5),
        ];
        let merged = merge_and_dedup(chroma, bm25);
        assert_eq!(merged.len(), 3);
        // Chroma result should come first for the duplicate
        assert_eq!(merged[0].chunk_id, "chunk-a");
        assert!(merged[0].vector_score.is_some());
    }

    #[test]
    fn test_merge_empty_inputs() {
        let merged = merge_and_dedup(vec![], vec![]);
        assert!(merged.is_empty());
    }

    #[test]
    fn test_merge_ordering() {
        let chroma = vec![
            make_chroma_result("chunk-a", 0.95),
            make_chroma_result("chunk-b", 0.90),
        ];
        let bm25 = vec![
            make_bm25_result("chunk-c", 8.5),
            make_bm25_result("chunk-d", 7.2),
        ];
        let merged = merge_and_dedup(chroma, bm25);
        assert_eq!(merged.len(), 4);
        // Chroma results first, then BM25 results
        assert!(merged[0].vector_score.is_some());
        assert!(merged[1].vector_score.is_some());
        assert!(merged[2].keyword_score.is_some());
        assert!(merged[3].keyword_score.is_some());
    }
}
