use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Bm25Result {
    pub chunk_id: String,
    pub document_name: String,
    pub chunk_index: usize,
    pub score: f64,
    pub text_snippet: String,
}

#[derive(Debug, Clone)]
pub struct Bm25Index {
    doc_count: usize,
    /// Term -> Document frequency (number of docs containing the term)
    doc_freq: HashMap<String, usize>,
    /// Term -> Map of chunk_id to Term Frequency in that document
    term_freq: HashMap<String, HashMap<String, usize>>,
    /// Chunk ID to Document metadata
    doc_meta: HashMap<String, (String, usize, String)>,
    /// Chunk ID to Document length in words
    doc_lengths: HashMap<String, usize>,
    avg_doc_len: f64,
}

impl Default for Bm25Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Bm25Index {
    pub fn new() -> Self {
        Self {
            doc_count: 0,
            doc_freq: HashMap::new(),
            term_freq: HashMap::new(),
            doc_meta: HashMap::new(),
            doc_lengths: HashMap::new(),
            avg_doc_len: 0.0,
        }
    }
}

pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|s| {
            s.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn build_index(
    docs: &[(String, String, usize, String)], // (chunk_id, doc_name, chunk_index, text)
) -> Bm25Index {
    let mut index = Bm25Index::new();
    index.doc_count = docs.len();

    let mut total_len = 0;

    for (chunk_id, doc_name, chunk_index, text) in docs {
        let tokens = tokenize(text);
        let len = tokens.len();
        total_len += len;

        index.doc_lengths.insert(chunk_id.clone(), len);
        index.doc_meta.insert(
            chunk_id.clone(),
            (
                doc_name.clone(),
                *chunk_index,
                text.chars().take(200).collect(),
            ),
        );

        let mut tf_map = HashMap::new();
        for t in &tokens {
            *tf_map.entry(t.clone()).or_insert(0) += 1;
        }

        for (t, count) in tf_map {
            index
                .term_freq
                .entry(t.clone())
                .or_default()
                .insert(chunk_id.clone(), count);

            *index.doc_freq.entry(t).or_insert(0) += 1;
        }
    }

    if index.doc_count > 0 {
        index.avg_doc_len = total_len as f64 / index.doc_count as f64;
    }

    tracing::debug!(
        component = "bm25",
        doc_count = index.doc_count,
        "bm25.index.built"
    );
    index
}

impl Bm25Index {
    /// Search the index with default BM25 parameters (k1=1.2, b=0.75).
    pub fn search(&self, query: &str, top_k: usize) -> Vec<Bm25Result> {
        self.search_with_params(query, top_k, 1.2, 0.75)
    }

    /// Search the index with configurable BM25 parameters.
    ///
    /// # Parameters
    /// * `k1` — Term frequency saturation (typical range 1.2–2.0).
    ///   Higher values increase the impact of term frequency.
    /// * `b` — Length normalization (typical range 0.0–1.0).
    ///   0.0 = no length normalization, 1.0 = full length normalization.
    pub fn search_with_params(
        &self,
        query: &str,
        top_k: usize,
        k1: f64,
        b: f64,
    ) -> Vec<Bm25Result> {
        tracing::trace!(
            component = "bm25",
            query = %query,
            k1 = k1,
            b = b,
            "bm25.search_with_params.started"
        );

        let tokens = tokenize(query);
        let mut scores: HashMap<String, f64> = HashMap::new();

        for token in &tokens {
            if let Some(df) = self.doc_freq.get(token) {
                // IDF formulation
                let idf =
                    ((self.doc_count as f64 - *df as f64 + 0.5) / (*df as f64 + 0.5) + 1.0).ln();

                if let Some(docs) = self.term_freq.get(token) {
                    for (doc_id, tf) in docs {
                        let doc_len = *self.doc_lengths.get(doc_id).unwrap_or(&0) as f64;
                        let tf = *tf as f64;

                        let norm = 1.0 - b + b * (doc_len / self.avg_doc_len);
                        let score = idf * (tf * (k1 + 1.0)) / (tf + k1 * norm);

                        *scores.entry(doc_id.clone()).or_insert(0.0) += score;
                    }
                }
            }
        }

        let mut results: Vec<Bm25Result> = scores
            .into_iter()
            .map(|(chunk_id, score)| {
                let meta = self.doc_meta.get(&chunk_id).cloned().unwrap_or_default();
                Bm25Result {
                    chunk_id,
                    document_name: meta.0,
                    chunk_index: meta.1,
                    score,
                    text_snippet: meta.2,
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.into_iter().take(top_k).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Default / New ──

    #[test]
    fn test_new_index_is_empty() {
        let idx = Bm25Index::new();
        assert_eq!(idx.doc_count, 0);
        assert!(idx.doc_freq.is_empty());
        assert!(idx.term_freq.is_empty());
        assert!(idx.doc_meta.is_empty());
        assert!(idx.doc_lengths.is_empty());
        assert_eq!(idx.avg_doc_len, 0.0);
    }

    #[test]
    fn test_default_equals_new() {
        assert_eq!(Bm25Index::default().doc_count, Bm25Index::new().doc_count);
    }

    #[test]
    fn test_search_on_empty_index_returns_empty() {
        let idx = Bm25Index::new();
        let results = idx.search("hello", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_build_index_empty_slice_returns_empty() {
        let idx = build_index(&[]);
        assert_eq!(idx.doc_count, 0);
        assert_eq!(idx.avg_doc_len, 0.0);
    }

    // ── tokenize ──

    #[test]
    fn test_tokenize_empty() {
        assert!(tokenize("").is_empty());
        assert!(tokenize("   ").is_empty());
    }

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("Hello World");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_lowercase() {
        let tokens = tokenize("BM25 Rust TEST");
        assert_eq!(tokens, vec!["bm25", "rust", "test"]);
    }

    #[test]
    fn test_tokenize_filters_punctuation() {
        let tokens = tokenize("hello, world! rust-is-fun");
        assert_eq!(tokens, vec!["hello", "world", "rustisfun"]);
    }

    #[test]
    fn test_tokenize_filters_only_punctuation() {
        let tokens = tokenize("!@#$ %^&*()");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_cyrillic() {
        let tokens = tokenize("Привет Мир");
        assert_eq!(tokens, vec!["привет", "мир"]);
    }

    #[test]
    fn test_tokenize_alphanumeric_retained() {
        let tokens = tokenize("rust2024 v1.2.3");
        assert_eq!(tokens, vec!["rust2024", "v123"]);
    }

    #[test]
    fn test_tokenize_multiple_whitespace() {
        let tokens = tokenize("a    b   c");
        assert_eq!(tokens, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_tokenize_filters_emoji() {
        // Emoji are not alphanumeric — filtered out by tokenize
        let tokens = tokenize("Hello 😀 World 🚀");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    // ── build_index ──

    #[test]
    fn test_build_index_single_doc() {
        let docs = vec![(
            "chunk-1".to_string(),
            "doc.md".to_string(),
            0,
            "Rust is a systems programming language".to_string(),
        )];
        let idx = build_index(&docs);

        assert_eq!(idx.doc_count, 1);
        assert_eq!(idx.avg_doc_len, 6.0);
        assert!(idx.doc_freq.contains_key("rust"));
        assert_eq!(idx.doc_freq.get("rust"), Some(&1));
        assert!(idx.doc_meta.contains_key("chunk-1"));
        assert_eq!(idx.doc_lengths.get("chunk-1"), Some(&6));
    }

    #[test]
    fn test_build_index_multiple_docs_updates_doc_freq() {
        let docs = vec![
            (
                "c1".to_string(),
                "a.md".to_string(),
                0,
                "rust is fast".to_string(),
            ),
            (
                "c2".to_string(),
                "b.md".to_string(),
                0,
                "python is slow".to_string(),
            ),
            (
                "c3".to_string(),
                "c.md".to_string(),
                0,
                "rust is safe".to_string(),
            ),
        ];
        let idx = build_index(&docs);

        assert_eq!(idx.doc_count, 3);
        // "rust" appears in 2 docs, "python" in 1, "is" in all 3
        assert_eq!(idx.doc_freq.get("rust"), Some(&2));
        assert_eq!(idx.doc_freq.get("python"), Some(&1));
        assert_eq!(idx.doc_freq.get("is"), Some(&3));
        assert_eq!(idx.avg_doc_len, 3.0); // (3 + 3 + 3) / 3
    }

    #[test]
    fn test_build_index_term_frequency_tracks_counts() {
        let docs = vec![(
            "c1".to_string(),
            "doc.md".to_string(),
            0usize,
            "hello hello world".to_string(),
        )];
        let idx = build_index(&docs);

        let c1_tf = idx.term_freq.get("hello").unwrap();
        assert_eq!(c1_tf.get("c1"), Some(&2));

        let world_tf = idx.term_freq.get("world").unwrap();
        assert_eq!(world_tf.get("c1"), Some(&1));
    }

    #[test]
    fn test_build_index_truncates_text_snippet() {
        let long_text = "a".repeat(500);
        let docs = vec![(
            "c1".to_string(),
            "doc.md".to_string(),
            0usize,
            long_text.clone(),
        )];
        let idx = build_index(&docs);
        let snippet = &idx.doc_meta.get("c1").unwrap().2;
        assert_eq!(snippet.len(), 200);
        assert_eq!(snippet, &"a".repeat(200));
    }

    #[test]
    fn test_build_index_skips_empty_doc_text() {
        let docs = vec![("c1".to_string(), "doc.md".to_string(), 0, "".to_string())];
        let idx = build_index(&docs);
        assert_eq!(idx.doc_count, 1);
        assert_eq!(idx.avg_doc_len, 0.0);
        assert!(idx.doc_freq.is_empty());
        assert!(idx.term_freq.is_empty());
    }

    // ── search ──

    fn sample_index() -> Bm25Index {
        let docs = vec![
            (
                "c1".to_string(),
                "rust-book.md".to_string(),
                0,
                "Rust is a systems programming language".to_string(),
            ),
            (
                "c2".to_string(),
                "python-book.md".to_string(),
                0,
                "Python is a high-level interpreted language".to_string(),
            ),
            (
                "c3".to_string(),
                "cargo-book.md".to_string(),
                0,
                "Cargo is the Rust package manager and build system".to_string(),
            ),
        ];
        build_index(&docs)
    }

    #[test]
    fn test_search_returns_relevant_docs() {
        let idx = sample_index();
        let results = idx.search("rust", 10);

        // Both c1 and c3 contain "rust"
        assert!(!results.is_empty());
        let ids: Vec<&str> = results.iter().map(|r| r.chunk_id.as_str()).collect();
        assert!(ids.contains(&"c1"));
        assert!(ids.contains(&"c3"));
    }

    #[test]
    fn test_search_respects_top_k() {
        let idx = sample_index();
        let results = idx.search("rust", 1);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_empty_query_returns_empty() {
        let idx = sample_index();
        let results = idx.search("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_no_match_returns_empty() {
        let idx = sample_index();
        let results = idx.search("supercalifragilisticexpialidocious", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_is_case_insensitive() {
        let idx = sample_index();
        let upper = idx.search("RUST", 10);
        let lower = idx.search("rust", 10);
        let mixed = idx.search("Rust", 10);

        assert_eq!(upper.len(), lower.len());
        assert_eq!(lower.len(), mixed.len());
        for (u, l) in upper.iter().zip(lower.iter()) {
            assert!((u.score - l.score).abs() < 1e-10);
        }
    }

    #[test]
    fn test_search_scores_positive() {
        let idx = sample_index();
        let results = idx.search("rust", 10);
        for r in &results {
            assert!(
                r.score > 0.0,
                "Expected positive score for matching doc, got {}",
                r.score
            );
        }
    }

    #[test]
    fn test_search_ranks_docs_by_relevance() {
        // "rust" appears once in c1 (5 words) and once in c3 (8 words)
        // shorter doc (c1) should score higher for same term frequency
        let idx = sample_index();
        let results = idx.search("rust", 10);

        // c1 ("Rust is a systems programming language" — 6 words)
        // c3 ("Cargo is the Rust package manager and build system" — 9 words)
        // shorter doc with same tf scores higher in BM25
        let c1 = results.iter().find(|r| r.chunk_id == "c1").unwrap();
        let c3 = results.iter().find(|r| r.chunk_id == "c3").unwrap();
        assert!(
                c1.score >= c3.score,
                "c1 (shorter doc) should score >= c3 (longer doc) for same term frequency, got c1={} c3={}",
                c1.score,
                c3.score
            );
    }

    #[test]
    fn test_search_multi_term_query() {
        let idx = sample_index();
        let results = idx.search("rust python", 10);

        // c1 has "rust", c2 has "python", c3 has "rust"
        let ids: Vec<&str> = results.iter().map(|r| r.chunk_id.as_str()).collect();
        assert!(ids.contains(&"c1"));
        assert!(ids.contains(&"c2"));
        assert!(ids.contains(&"c3"));
    }

    #[test]
    fn test_search_results_sorted_by_score_descending() {
        let idx = sample_index();
        let results = idx.search("rust python cargo", 10);

        for w in results.windows(2) {
            assert!(
                w[0].score >= w[1].score,
                "Results must be sorted descending by score: {} < {}",
                w[0].score,
                w[1].score
            );
        }
    }

    #[test]
    fn test_search_punctuation_in_query() {
        // tokenize strips punctuation, so "rust!" should match "rust"
        let idx = sample_index();
        let clean = idx.search("rust", 10);
        let punct = idx.search("rust!", 10);

        assert_eq!(clean.len(), punct.len());
        for (c, p) in clean.iter().zip(punct.iter()) {
            assert!((c.score - p.score).abs() < 1e-10);
        }
    }

    #[test]
    fn test_search_result_contains_metadata() {
        let idx = sample_index();
        let results = idx.search("rust", 10);

        let c1 = results.iter().find(|r| r.chunk_id == "c1").unwrap();
        assert_eq!(c1.document_name, "rust-book.md");
        assert_eq!(c1.chunk_index, 0);
        assert!(!c1.text_snippet.is_empty());
        assert!(c1.text_snippet.len() <= 200);
    }

    #[test]
    fn test_search_duplicate_terms_in_query_accumulate_score() {
        let docs = vec![(
            "c1".to_string(),
            "doc.md".to_string(),
            0usize,
            "hello world".to_string(),
        )];
        let idx = build_index(&docs);

        // Querying "hello hello" has duplicate token "hello"
        // tokenize normalizes to ["hello", "hello"] — both contribute to score
        let single = idx.search("hello", 10);
        let double = idx.search("hello hello", 10);

        assert_eq!(single.len(), 1);
        assert_eq!(double.len(), 1);
        assert!(
            double[0].score > single[0].score,
            "Duplicate query terms should accumulate: single={}, double={}",
            single[0].score,
            double[0].score
        );
    }

    #[test]
    fn test_search_multiple_docs_same_term_different_frequencies() {
        let docs = vec![
            (
                "c1".to_string(),
                "high.md".to_string(),
                0,
                "rust rust rust important".to_string(),
            ),
            (
                "c2".to_string(),
                "low.md".to_string(),
                0,
                "rust only once".to_string(),
            ),
        ];
        let idx = build_index(&docs);

        let results = idx.search("rust", 10);
        assert_eq!(results.len(), 2);
        // c1 has tf=3, c2 has tf=1 — c1 should score higher
        let c1 = results.iter().find(|r| r.chunk_id == "c1").unwrap();
        let c2 = results.iter().find(|r| r.chunk_id == "c2").unwrap();
        assert!(
            c1.score > c2.score,
            "Higher term frequency should yield higher score: c1={}, c2={}",
            c1.score,
            c2.score
        );
    }

    #[test]
    fn test_search_very_long_query_does_not_explode() {
        let docs = vec![
            (
                "c1".to_string(),
                "doc.md".to_string(),
                0,
                "The quick brown fox jumps over the lazy dog".to_string(),
            ),
            (
                "c2".to_string(),
                "other.md".to_string(),
                0,
                "Something completely different".to_string(),
            ),
        ];
        let idx = build_index(&docs);

        // Single-char tokens won't match multi-char words in the index
        let long_query = "a b c d e f g h i j k l m n o p q r s t u v w x y z";
        let results = idx.search(long_query, 10);
        assert!(
            results.is_empty(),
            "single-char tokens don't match multi-char words"
        );

        // A query with actual content words DOES match
        let results = idx.search("quick brown fox", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].chunk_id, "c1");
    }

    #[test]
    fn test_search_bm25_score_reproducible() {
        // Same index and query must produce identical scores
        let docs = vec![(
            "c1".to_string(),
            "doc.md".to_string(),
            0,
            "consistent indexing produces consistent scoring".to_string(),
        )];
        let idx = build_index(&docs);

        let r1 = idx.search("consistent", 10);
        let r2 = idx.search("consistent", 10);
        assert_eq!(r1.len(), r2.len());
        if !r1.is_empty() {
            assert!((r1[0].score - r2[0].score).abs() < 1e-12);
        }
    }

    // ── Edge cases ──

    #[test]
    fn test_search_top_k_larger_than_result_set() {
        let idx = sample_index();
        let all = idx.search("language", 10);
        let oversized = idx.search("language", 100);
        assert_eq!(all.len(), oversized.len());
    }

    #[test]
    fn test_search_top_k_zero_returns_empty() {
        let idx = sample_index();
        let results = idx.search("rust", 0);
        assert!(results.is_empty());
    }

    #[test]
    fn test_build_index_preserves_chunk_order_metadata() {
        let docs = vec![
            (
                "c0".to_string(),
                "doc.md".to_string(),
                0,
                "first chunk".to_string(),
            ),
            (
                "c1".to_string(),
                "doc.md".to_string(),
                1,
                "second chunk".to_string(),
            ),
            (
                "c2".to_string(),
                "doc.md".to_string(),
                2,
                "third chunk".to_string(),
            ),
        ];
        let idx = build_index(&docs);

        assert_eq!(idx.doc_meta.get("c0").unwrap().1, 0);
        assert_eq!(idx.doc_meta.get("c1").unwrap().1, 1);
        assert_eq!(idx.doc_meta.get("c2").unwrap().1, 2);
    }

    #[test]
    fn test_search_non_ascii_query_matches_cyrillic() {
        let docs = vec![(
            "c1".to_string(),
            "doc.md".to_string(),
            0usize,
            "Привет мир".to_string(),
        )];
        let idx = build_index(&docs);

        let results = idx.search("привет", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].chunk_id, "c1");
    }

    #[test]
    fn test_search_emoji_filtered_by_tokenizer() {
        // Emoji are filtered by is_alphanumeric() — not in index, not searchable
        let docs = vec![(
            "c1".to_string(),
            "doc.md".to_string(),
            0usize,
            "Hello 😀 World".to_string(),
        )];
        let idx = build_index(&docs);

        let results = idx.search("😀", 10);
        assert!(results.is_empty(), "emoji filtered out by tokenizer");

        // Regular text still matches
        let results = idx.search("hello", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_build_index_multiple_chunks_same_doc() {
        let docs = vec![
            (
                "c0".to_string(),
                "doc.md".to_string(),
                0,
                "Rust introduction".to_string(),
            ),
            (
                "c1".to_string(),
                "doc.md".to_string(),
                1,
                "Rust advanced concepts".to_string(),
            ),
        ];
        let idx = build_index(&docs);

        // "rust" appears in both chunks, doc_freq should be 2
        assert_eq!(idx.doc_freq.get("rust"), Some(&2));
        assert_eq!(idx.doc_count, 2);
    }
}
