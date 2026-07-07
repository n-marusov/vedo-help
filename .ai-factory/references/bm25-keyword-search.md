# BM25 Keyword Search Reference

> Source: https://en.wikipedia.org/wiki/Okapi_BM25; https://lucene.apache.org/core/9_9_1/core/org/apache/lucene/search/similarities/BM25Similarity.html; https://www.elastic.co/blog/practical-bm25-part-2-the-bm25-algorithm-and-its-variables; https://www.elastic.co/docs/reference/elasticsearch/index-settings/similarity
> Created: 2026-07-08
> Updated: 2026-07-08

## Overview

Okapi BM25, usually shortened to BM25, is a bag-of-words ranking function for keyword search. It ranks documents by summing per-query-term contributions based on three main signals: inverse document frequency (rarer terms matter more), term frequency saturation (repeated occurrences help but with diminishing returns), and document-length normalization (a match in a shorter field/document is usually more concentrated than the same match in a much longer one).

BM25 comes from the probabilistic retrieval framework and was introduced through the Okapi information retrieval system. In modern search engines it is a practical successor to plain TF-IDF: it keeps the intuition of TF-IDF while adding bounded term-frequency growth and explicit length normalization. Elasticsearch uses BM25 as its default similarity, and Lucene exposes it as `BM25Similarity`.

## Core Concepts

`Q`: The query, treated as a set or sequence of analyzed query terms `q1 ... qn`. BM25 is bag-of-words: term proximity and order are not part of the base formula.

`D`: A candidate document or field being scored.

`f(qi, D)`: Term frequency, the number of times query term `qi` occurs in document `D`.

`|D|`: Document length, usually measured in analyzed terms/tokens rather than characters.

`avgdl`: Average document length in the collection or field.

`N`: Total number of documents in the collection, or in Lucene/Elasticsearch field statistics, the number of documents that have a value for the field.

`n(qi)` / `docFreq`: Number of documents containing query term `qi`.

`IDF(qi)`: Inverse document frequency. BM25 gives more weight to rare terms and less weight to common terms.

`k1`: Non-linear term-frequency saturation parameter. Higher `k1` lets repeated occurrences keep adding more score; lower `k1` saturates faster. Lucene and Elasticsearch default to `1.2`. Wikipedia notes a common range of `1.2` to `2.0` when no tuning data is available.

`b`: Document-length normalization parameter. `b = 0` disables length normalization; `b = 1` fully normalizes by `|D| / avgdl`. Lucene and Elasticsearch default to `0.75`.

`discount_overlaps` / `discountOverlaps`: Lucene/Elasticsearch option controlling whether overlap tokens, such as tokens with position increment `0`, are ignored when computing length norms. Defaults to `true` in Lucene and Elasticsearch.

## API / Interface

### Canonical BM25 score

A common BM25 scoring function is:

```text
score(D, Q) = sum over qi in Q of
  IDF(qi) * ( f(qi, D) * (k1 + 1) ) /
             ( f(qi, D) + k1 * (1 - b + b * |D| / avgdl) )
```

A widely used non-negative IDF variant, also used by Lucene, is:

```text
IDF(qi) = log(1 + (N - n(qi) + 0.5) / (n(qi) + 0.5))
```

Lucene documents this as:

```text
idf(docFreq, docCount) = log(1 + (docCount - docFreq + 0.5) / (docFreq + 0.5))
```

### Lucene `BM25Similarity`

Lucene class:

```text
org.apache.lucene.search.similarities.BM25Similarity
```

Constructors documented by Lucene:

```text
BM25Similarity()
BM25Similarity(boolean discountOverlaps)
BM25Similarity(float k1, float b)
BM25Similarity(float k1, float b, boolean discountOverlaps)
```

Defaults:

```text
k1 = 1.2
b = 0.75
discountOverlaps = true
```

Parameter constraints:

```text
k1: must not be infinite or negative
b: must be within [0..1]
```

Selected Lucene methods:

```text
protected float idf(long docFreq, long docCount)
protected float avgFieldLength(CollectionStatistics collectionStats)
final float getK1()
final float getB()
boolean getDiscountOverlaps()
```

Lucene's default average field length implementation computes:

```text
sumTotalTermFreq / docCount
```

### Elasticsearch similarity settings

Elasticsearch identifies the built-in BM25 similarity with type name:

```text
BM25
```

Documented options:

| Option | Meaning | Default |
|--------|---------|---------|
| `k1` | Controls non-linear term-frequency normalization/saturation | `1.2` |
| `b` | Controls how strongly document length normalizes term-frequency values | `0.75` |
| `discount_overlaps` | Whether overlap tokens are ignored when computing norms | `true` |

Elasticsearch states that similarity is configured per field and applies to `text` and `keyword` field types. Custom similarity configuration is considered an expert feature; built-in similarities are usually sufficient.

Example shape from Elasticsearch documentation for configuring similarities:

```json
PUT /index
{
  "settings": {
    "index": {
      "similarity": {
        "my_similarity": {
          "type": "DFR",
          "basic_model": "g",
          "after_effect": "l",
          "normalization": "h2",
          "normalization.h2.c": "3.0"
        }
      }
    }
  }
}
```

For BM25, the analogous custom similarity would use `"type": "BM25"` plus BM25 options.

## Usage Patterns

### 1. Minimal scoring pipeline

```text
Input:
  - analyzed query terms
  - inverted index postings with term frequencies per document
  - collection statistics: docCount, docFreq per term, avgdl

For each query term qi:
  compute idf(qi)
  fetch postings list for qi
  for each matching document D:
    compute tf_component = (f(qi,D) * (k1 + 1)) /
                           (f(qi,D) + k1 * (1 - b + b * |D| / avgdl))
    add idf(qi) * tf_component to score[D]

Return documents ordered by descending score.
```

### 2. RAG retrieval usage

BM25 is useful as a sparse lexical retriever in Retrieval-Augmented Generation systems:

1. Tokenize and normalize documents consistently at indexing and query time.
2. Index document chunks, not only full documents, when answers need localized evidence.
3. Score query terms with BM25.
4. Return top-k chunks/documents with source metadata.
5. Optionally combine BM25 with dense vector retrieval in a hybrid retriever.

BM25 is especially valuable when exact words, identifiers, error messages, API names, class names, command names, and rare technical terms are important.

### 3. Tuning `k1`

Use `k1` to control how much repeated occurrences matter:

- Lower `k1`: term frequency saturates quickly; extra repetitions add little.
- Higher `k1`: extra repetitions continue adding more score.
- `k1 = 0`: the term-frequency portion cancels out and only IDF-like contribution remains, as shown in Elastic's practical example.

### 4. Tuning `b`

Use `b` to control length normalization:

- `b = 0`: document/field length has no effect; only term counts and IDF matter.
- `b = 1`: full length normalization by `|D| / avgdl`.
- `b = 0.75`: common default in Lucene and Elasticsearch.

Elastic's explanation: if a document is longer than average, the denominator grows and score decreases; if shorter than average, the denominator shrinks and score increases.

### 5. Field-aware search

Base BM25 treats each scored text stream as one bag of words. For structured documents with title, body, headings, anchor text, or metadata fields, use field-specific weighting or a BM25 variant such as BM25F. Wikipedia describes BM25F as a modification where a document is composed of several weighted fields/streams with potentially different importance and normalization.

## Configuration

| Setting | Typical value | Source notes | Practical effect |
|---------|---------------|--------------|------------------|
| `k1` | `1.2` | Lucene/Elasticsearch default | Balanced term-frequency saturation |
| `k1` | `1.2`–`2.0` | Wikipedia notes common range without advanced optimization | Slower saturation as value rises |
| `b` | `0.75` | Lucene/Elasticsearch default | Partial length normalization |
| `b` | `0` | BM15-like extreme | No document-length normalization |
| `b` | `1` | BM11-like extreme | Full document-length normalization |
| `discount_overlaps` | `true` | Lucene/Elasticsearch default | Overlap tokens do not increase field length |

## Best Practices

1. Use the same analyzer at indexing and query time when possible, because BM25 relies on token counts, term frequencies, and document frequencies after analysis.
2. Start with defaults (`k1 = 1.2`, `b = 0.75`) unless you have relevance judgments, because Lucene and Elasticsearch use these defaults and Elasticsearch describes built-in similarities as usually sufficient.
3. Tune `k1` when repeated terms are over- or under-valued. Lower it if repetition causes spammy documents to rank too highly; raise it if multiple mentions should matter more.
4. Tune `b` when field length has the wrong effect. Lower it for fields where length should not penalize much; raise it when concise fields should be rewarded more strongly.
5. Score comparable fields together. BM25 length statistics and term statistics are field-sensitive in Lucene/Elasticsearch; mixing very different content types in one field can produce unintuitive scores.
6. Use BM25 for exact lexical evidence in hybrid retrieval. Dense embeddings can capture semantic similarity, while BM25 strongly handles exact tokens, rare identifiers, logs, code symbols, and product names.
7. Keep source metadata with retrieved chunks. BM25 returns relevance-ranked text, but downstream answer generation still needs citations and provenance.
8. Use explain/debug tooling when tuning. Elasticsearch examples use `_search?explain=true` to inspect score components such as `doc.freq`, `doc.length`, `term.docFreq`, and field statistics.

## Common Pitfalls

1. Assuming BM25 understands meaning or proximity. Base BM25 is bag-of-words; it does not model word order, phrase proximity, synonyms, or semantic similarity unless the analyzer/query layer adds them.
2. Treating raw document length as characters. Lucene/Elasticsearch length normalization is based on analyzed terms/tokens, with overlap-token handling controlled by `discount_overlaps`.
3. Over-tuning without judgments. `k1` and `b` can change rankings dramatically; tune against representative queries and relevance labels where available.
4. Indexing very long heterogeneous documents as one unit. A single long document can dilute exact matches; chunking or field separation usually works better for RAG and documentation search.
5. Removing useful rare terms during analysis. BM25 depends heavily on rare terms via IDF; aggressive normalization or stopword rules can remove discriminative terms.
6. Comparing scores across different indices/shards without understanding statistics. Elastic's practical BM25 series notes that document frequency and field statistics affect scores; distributed search may need special handling such as DFS in some cases.
7. Expecting repeated keywords to scale linearly. BM25 intentionally saturates term frequency: each additional occurrence contributes less than the previous one.

## Version Notes

Lucene 9.9.1 documents `BM25Similarity` defaults as `k1 = 1.2`, `b = 0.75`, and `discountOverlaps = true`.

Elasticsearch documentation lists BM25 as the default similarity and exposes `k1`, `b`, and `discount_overlaps` options. Similarity is per field and applies to `text` and `keyword` field types.

BM25 variants mentioned in the sources:

- `BM25F`: field-aware BM25 for documents composed of multiple weighted fields or streams.
- `BM25+`: adds a lower-bounding correction to address a deficiency where long matching documents may be scored too similarly to shorter non-matching documents; Wikipedia lists an additional free parameter `δ`, commonly defaulted to `1.0` in absence of training data.
