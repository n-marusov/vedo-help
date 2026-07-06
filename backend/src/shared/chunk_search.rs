use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::shared::chroma_client::ChromaClient;
use crate::shared::embedding_client::EmbeddingClient;
use crate::shared::error::AppError;

/// A single chunk search result, enriched with document metadata from PostgreSQL.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChunkSearchResult {
    pub chunk_id: Uuid,
    pub document_id: Uuid,
    pub document_name: String,
    pub chunk_index: usize,
    pub text: String,
    pub source: String,
    pub score: Option<f64>,
    pub file_path: Option<String>,
}

/// Which search method to use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchMode {
    Text,
    Semantic,
}

/// Search chunks by text content using PostgreSQL ILIKE.
///
/// JOINs `chunks` + `documents` for a complete result. Supports filtering by
/// source (`upload` / `git`) and pagination via LIMIT/OFFSET.
pub async fn search_chunks_text(
    db: &PgPool,
    collection_id: Uuid,
    query: &str,
    source: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<Vec<ChunkSearchResult>, AppError> {
    tracing::debug!(
        component = "chunk_search",
        collection_id = %collection_id,
        query = %query,
        source = ?source,
        limit = limit,
        offset = offset,
        "search_chunks_text"
    );

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"SELECT c.id, c.document_id, d.name, c."index", c.text, d.source
           FROM chunks c
           JOIN documents d ON d.id = c.document_id
           WHERE c.is_active = TRUE
             AND d.is_active = TRUE
             AND d.collection_id = "#,
    );
    query_builder.push_bind(collection_id);

    if let Some(src) = source {
        if !src.is_empty() {
            query_builder.push(" AND d.source = ");
            query_builder.push_bind(src);
        }
    }

    if !query.is_empty() {
        // Use ILIKE for case-insensitive matching
        query_builder.push(" AND c.text ILIKE ");
        query_builder.push_bind(format!("%{query}%"));
    }

    query_builder.push(r#" ORDER BY d.name, c."index""#);
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit as i32);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset as i32);

    let rows = query_builder
        .build_query_as::<(Uuid, Uuid, String, i32, String, String)>()
        .fetch_all(db)
        .await
        .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?;

    let results: Vec<ChunkSearchResult> = rows
        .into_iter()
        .map(|row| ChunkSearchResult {
            chunk_id: row.0,
            document_id: row.1,
            document_name: row.2.clone(),
            chunk_index: row.3 as usize,
            text: row.4,
            source: row.5.clone(),
            score: None,
            file_path: if row.5 == "git" { Some(row.2) } else { None },
        })
        .collect();

    tracing::info!(
        component = "chunk_search",
        result_count = results.len(),
        "search_chunks_text.found"
    );

    if results.is_empty() {
        tracing::warn!(
            component = "chunk_search",
            collection_id = %collection_id,
            "search_chunks_text.empty"
        );
    }

    Ok(results)
}

/// Search chunks by semantic similarity using Chroma.
///
/// Embeds the query, retrieves top-k from Chroma, then fetches document metadata
/// from PostgreSQL by matching on `document_id` (stored as string in Chroma metadata).
pub async fn search_chunks_semantic(
    chroma: &ChromaClient,
    embedding_client: &EmbeddingClient,
    db: &PgPool,
    collection_id: Uuid,
    query: &str,
    source: Option<&str>,
    top_k: usize,
    model: &str,
) -> Result<Vec<ChunkSearchResult>, AppError> {
    tracing::debug!(
        component = "chunk_search",
        collection_id = %collection_id,
        query = %query,
        source = ?source,
        top_k = top_k,
        "search_chunks_semantic"
    );

    // 1. Embed the query
    let embeddings = embedding_client
        .embed(model, vec![query.to_string()])
        .await
        .map_err(|e| {
            tracing::error!(
                component = "chunk_search",
                error = %e,
                "search_chunks_semantic.embedding_failed"
            );
            e
        })?;

    if embeddings.is_empty() {
        return Err(AppError::InternalError(
            "Embedding returned empty result".to_string(),
        ));
    }

    // 2. Build Chroma where filter
    let mut where_filter = serde_json::json!({
        "is_active": true,
    });
    if let Some(src) = source {
        if !src.is_empty() {
            where_filter["source"] = serde_json::Value::String(src.to_string());
        }
    }

    // 3. Query Chroma
    let collection_name = collection_id.to_string();
    let chroma_results = chroma
        .query(&collection_name, &embeddings[0], top_k, Some(where_filter))
        .await?;

    if chroma_results.is_empty() {
        tracing::warn!(
            component = "chunk_search",
            collection_id = %collection_id,
            "search_chunks_semantic.empty"
        );
        return Ok(Vec::new());
    }

    // 4. Fetch document metadata from PostgreSQL
    let doc_ids: Vec<Uuid> = chroma_results
        .iter()
        .filter_map(|r| Uuid::parse_str(&r.document_id).ok())
        .collect();

    let docs = if doc_ids.is_empty() {
        Vec::new()
    } else {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT id, name, source FROM documents WHERE id IN (");
        let mut separated = query_builder.separated(", ");
        for id in &doc_ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");

        query_builder
            .build_query_as::<(Uuid, String, String)>()
            .fetch_all(db)
            .await
            .map_err(|e| AppError::InternalError(format!("Database error: {e}")))?
    };

    let doc_map: std::collections::HashMap<Uuid, (String, String)> = docs
        .into_iter()
        .map(|(id, name, source)| (id, (name, source)))
        .collect();

    // 5. Build results
    let results: Vec<ChunkSearchResult> = chroma_results
        .into_iter()
        .filter_map(|cr| {
            let doc_uuid = Uuid::parse_str(&cr.document_id).ok()?;
            let (doc_name, source) = doc_map.get(&doc_uuid).cloned().unwrap_or_default();

            Some(ChunkSearchResult {
                chunk_id: Uuid::parse_str(&cr.id).unwrap_or_default(),
                document_id: doc_uuid,
                document_name: doc_name.clone(),
                chunk_index: cr.chunk_index,
                text: cr.text,
                source: source.clone(),
                score: Some(cr.score),
                file_path: if source == "git" {
                    Some(doc_name)
                } else {
                    None
                },
            })
        })
        .collect();

    tracing::info!(
        component = "chunk_search",
        result_count = results.len(),
        "search_chunks_semantic.found"
    );

    Ok(results)
}
