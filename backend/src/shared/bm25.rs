/// BM25 keyword search index.
///
/// Implements the BM25 ranking function with standard parameters:
/// - k1 = 1.2 (term saturation)
/// - b = 0.75 (length normalization)
use std::collections::HashMap;

/// A single BM25 search result.
#[derive(Debug, Clone)]
pub struct Bm25Result {
    pub doc_id: String,
    pub text: String,
    pub score: f64,
}

/// BM25 inverted index for keyword search.
#[derive(Debug, Clone)]
pub struct Bm25Index {
    /// Inverted index: term → Vec<(doc_id, term_frequency)>
    inverted_index: HashMap<String, Vec<(usize, usize)>>,
    /// Document lengths (in tokens) per document index.
    doc_lengths: Vec<usize>,
    /// Document texts stored as (doc_id, text) pairs ordered by index.
    docs: Vec<(String, String)>,
    /// Total number of documents.
    num_docs: usize,
    /// Average document length across the corpus.
    avg_doc_length: f64,
}

/// Tokenize text into lowercase tokens, splitting on whitespace and punctuation.
pub fn tokenize(text: &str) -> Vec<String> {
    // Stub — implementation follows in Phase 3.
    if text.is_empty() {
        return vec![];
    }
    text.split_whitespace()
        .flat_map(|word| {
            word.split(|c: char| !c.is_alphanumeric())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_lowercase())
        })
        .collect()
}

/// Build a BM25 index from (doc_id, text) pairs.
pub fn build_index(docs: &[(String, String)]) -> Bm25Index {
    let num_docs = docs.len();
    if num_docs == 0 {
        return Bm25Index {
            inverted_index: HashMap::new(),
            doc_lengths: vec![],
            docs: vec![],
            num_docs: 0,
            avg_doc_length: 0.0,
        };
    }
    let mut inverted_index: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
    let mut doc_lengths = Vec::with_capacity(num_docs);

    for (doc_idx, (_doc_id, text)) in docs.iter().enumerate() {
        let tokens = tokenize(text);
        doc_lengths.push(tokens.len());
        let mut term_freq: HashMap<String, usize> = HashMap::new();
        for token in &tokens {
            *term_freq.entry(token.clone()).or_insert(0) += 1;
        }
        for (term, freq) in term_freq {
            inverted_index
                .entry(term)
                .or_default()
                .push((doc_idx, freq));
        }
    }

    let total_tokens: usize = doc_lengths.iter().sum();
    let avg_doc_length = if num_docs > 0 {
        total_tokens as f64 / num_docs as f64
    } else {
        0.0
    };

    Bm25Index {
        inverted_index,
        doc_lengths,
        docs: docs.to_vec(),
        num_docs,
        avg_doc_length,
    }
}

impl Bm25Index {
    /// Search the index for the given query, returning up to `top_k` results ranked by BM25 score.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<Bm25Result> {
        if self.num_docs == 0 || query.is_empty() {
            return vec![];
        }

        let query_tokens = tokenize(query);
        if query_tokens.is_empty() {
            return vec![];
        }

        let k1 = 1.2_f64;
        let b = 0.75_f64;
        let mut scores: Vec<f64> = vec![0.0; self.num_docs];

        for token in &query_tokens {
            let posting = self.inverted_index.get(token);
            if let Some(postings) = posting {
                let df = postings.len() as f64;
                let idf = ((self.num_docs as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
                for &(doc_idx, tf) in postings {
                    let doc_len = self.doc_lengths[doc_idx] as f64;
                    let denom = tf as f64 + k1 * (1.0 - b + b * doc_len / self.avg_doc_length);
                    scores[doc_idx] += idf * (tf as f64 * (k1 + 1.0)) / denom;
                }
            }
        }

        let mut results: Vec<(usize, f64)> = scores
            .into_iter()
            .enumerate()
            .filter(|(_, s)| *s > 0.0)
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);

        results
            .into_iter()
            .map(|(idx, score)| {
                let (doc_id, text) = &self.docs[idx];
                Bm25Result {
                    doc_id: doc_id.clone(),
                    text: text.clone(),
                    score,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_splits_whitespace_and_punctuation() {
        let tokens = tokenize("Hello, World!");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_empty_input() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_inverted_index_construction() {
        let docs = vec![
            ("doc1".to_string(), "hello world".to_string()),
            ("doc2".to_string(), "hello foo".to_string()),
        ];
        let index = build_index(&docs);
        // "hello" appears in both docs (index 0 and 1)
        let hello_postings = index
            .inverted_index
            .get("hello")
            .expect("hello should be indexed");
        assert_eq!(hello_postings.len(), 2);
        // "world" appears only in doc0
        let world_postings = index
            .inverted_index
            .get("world")
            .expect("world should be indexed");
        assert_eq!(world_postings.len(), 1);
        assert_eq!(world_postings[0].0, 0);
    }

    #[test]
    fn test_bm25_score_increases_with_matches() {
        let docs = vec![
            ("doc1".to_string(), "rust programming language".to_string()),
            (
                "doc2".to_string(),
                "rust is great for systems programming".to_string(),
            ),
            (
                "doc3".to_string(),
                "python is a programming language".to_string(),
            ),
        ];
        let index = build_index(&docs);
        let results = index.search("rust programming", 3);
        assert!(!results.is_empty(), "should return at least one result");
        // doc1 and doc2 should rank higher than doc3 for "rust programming"
        let doc_ids: Vec<&str> = results.iter().map(|r| r.doc_id.as_str()).collect();
        assert!(
            doc_ids[0] == "doc1" || doc_ids[0] == "doc2",
            "most relevant doc should be doc1 or doc2, got {:?}",
            doc_ids
        );
    }

    #[test]
    fn test_search_returns_ranked_docs() {
        let docs = vec![
            ("doc1".to_string(), "the cat sat on the mat".to_string()),
            ("doc2".to_string(), "the dog played in the yard".to_string()),
            ("doc3".to_string(), "cats and dogs are pets".to_string()),
        ];
        let index = build_index(&docs);
        let results = index.search("cat mat", 3);
        assert!(!results.is_empty());
        // doc1 should be the top result for "cat mat"
        assert_eq!(results[0].doc_id, "doc1");
    }

    #[test]
    fn test_search_empty_corpus() {
        let index = build_index(&[]);
        let results = index.search("hello", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_bm25_with_repeated_terms() {
        let docs = vec![
            ("doc1".to_string(), "rust rust rust".to_string()),
            ("doc2".to_string(), "rust is nice".to_string()),
        ];
        let index = build_index(&docs);
        let results = index.search("rust", 2);
        assert_eq!(results.len(), 2);
        // doc1 has higher term frequency, so it should score higher
        assert!(results[0].score > results[1].score);
    }
}
