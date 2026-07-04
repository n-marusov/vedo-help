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
    pub fn search(&self, query: &str, top_k: usize) -> Vec<Bm25Result> {
        tracing::trace!(component = "bm25", query = %query, "bm25.search.started");

        let k1 = 1.5;
        let b = 0.75;
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
