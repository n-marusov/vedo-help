use std::time::Duration;

use async_trait::async_trait;
use opentelemetry::trace::TraceContextExt;
use reqwest::Client;
use serde_json::json;

use crate::shared::error::AppError;
use crate::shared::types::ChromaResult;

/// Inject OpenTelemetry trace context headers into a HeaderMap.
///
/// Returns an empty HeaderMap when there is no sampled span context.
fn inject_trace_headers() -> reqwest::header::HeaderMap {
    let cx = opentelemetry::Context::current();
    let span = cx.span();
    let span_context = span.span_context();
    if span_context.is_sampled() {
        let mut headers = reqwest::header::HeaderMap::new();
        opentelemetry::global::get_text_map_propagator(|propagator| {
            propagator.inject(&mut opentelemetry_http::HeaderInjector(&mut headers))
        });
        headers
    } else {
        reqwest::header::HeaderMap::new()
    }
}

/// HTTP client for Chroma vector database REST API.
#[derive(Clone, Debug)]
pub struct ChromaClient {
    client: Client,
    base_url: String,
}

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 500;

impl ChromaClient {
    /// Create a new Chroma client from the given base URL.
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for Chroma");

        tracing::debug!(component = "chroma_client", url = %base_url, "client.initialized");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Add embeddings to a collection.
    pub async fn add_embeddings(
        &self,
        collection: &str,
        ids: &[String],
        embeddings: &[Vec<f32>],
        metadatas: &[serde_json::Value],
    ) -> Result<(), AppError> {
        tracing::debug!(
            component = "chroma_client",
            collection = %collection,
            count = ids.len(),
            "add_embeddings"
        );

        let body = json!({
            "ids": ids,
            "embeddings": embeddings,
            "metadatas": metadatas,
        });

        // Resolve collection name to Chroma UUID for URL path.
        // Chroma 0.6.x requires the internal UUID, not the name, in sub-resource URLs.
        // Fall back to original name if resolution fails — retry logic below
        // will attempt resolution again on InvalidCollection errors.
        let resolved_id = match self.resolve_collection_id(collection).await {
            Ok(id) => {
                tracing::debug!(
                    component = "chroma_client",
                    collection = %collection,
                    chroma_id = %id,
                    "add_embeddings.resolved_collection"
                );
                id
            }
            Err(e) => {
                tracing::warn!(
                    component = "chroma_client",
                    error = %e,
                    "add_embeddings.resolution_failed"
                );
                collection.to_string()
            }
        };
        let url = format!("{}/api/v1/collections/{}/add", self.base_url, resolved_id);

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            match Self::try_add(&self.client, &url, &body).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(
                        component = "chroma_client",
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        "add_embeddings.retry"
                    );
                    last_error = Some(e);
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| AppError::ChromaError("All retry attempts exhausted".to_string())))
    }

    async fn try_add(client: &Client, url: &str, body: &serde_json::Value) -> Result<(), AppError> {
        let response = client
            .post(url)
            .headers(inject_trace_headers())
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::ChromaError(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::ChromaError(format!(
                "Add embeddings failed (HTTP {status}): {text}"
            )));
        }

        Ok(())
    }

    /// Resolve a collection name to its Chroma-assigned UUID.
    ///
    /// Chroma 0.6.x requires the internal UUID (not the collection name) in
    /// URL paths for sub-resource operations like `/add`, `/query`, `/delete`.
    /// This method calls `POST /api/v1/collections` with `get_or_create: true`
    /// to look up the collection and return its Chroma `id` field.
    pub async fn resolve_collection_id(&self, name: &str) -> Result<String, AppError> {
        tracing::debug!(component = "chroma_client", collection = %name, "resolve_collection_id.resolving");
        let chroma_id = self.get_or_create_collection_id(name).await?;
        tracing::debug!(component = "chroma_client", collection = %name, chroma_id = %chroma_id, "resolve_collection_id.resolved");
        Ok(chroma_id)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn is_missing_collection_error(error: &AppError) -> bool {
        match error {
            AppError::ChromaError(message) => {
                message.contains("InvalidCollection") && message.contains("does not exist")
            }
            _ => false,
        }
    }

    /// Query a collection by embedding vector, returning the top-k matches.
    ///
    /// If `where_filter` is `Some`, it is included as the `"where"` field in
    /// the request body to filter results by metadata attributes.
    pub async fn query(
        &self,
        collection: &str,
        embedding: &[f32],
        top_k: usize,
        where_filter: Option<serde_json::Value>,
    ) -> Result<Vec<ChromaResult>, AppError> {
        tracing::debug!(
            component = "chroma_client",
            collection = %collection,
            top_k = top_k,
            "query"
        );

        let mut body = json!({
            "query_embeddings": [embedding],
            "n_results": top_k,
            "include": ["metadatas", "distances", "documents"],
        });
        if let Some(filter) = where_filter {
            tracing::debug!(component = "chroma_client", filter = %filter, "query.with_filter");
            body["where"] = filter;
        }

        // Resolve collection name to Chroma UUID for URL path.
        // Chroma 0.6.x requires the internal UUID, not the name, in sub-resource URLs.
        let resolved_id = match self.resolve_collection_id(collection).await {
            Ok(id) => {
                tracing::debug!(
                    component = "chroma_client",
                    collection = %collection,
                    chroma_id = %id,
                    "query.resolved_collection"
                );
                id
            }
            Err(e) => {
                tracing::warn!(
                    component = "chroma_client",
                    error = %e,
                    "query.resolution_failed"
                );
                collection.to_string()
            }
        };
        let url = format!("{}/api/v1/collections/{}/query", self.base_url, resolved_id);

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            match Self::try_query(&self.client, &url, &body).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(
                        component = "chroma_client",
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        "query.retry"
                    );
                    last_error = Some(e);
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| AppError::ChromaError("All retry attempts exhausted".to_string())))
    }

    async fn try_query(
        client: &Client,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<Vec<ChromaResult>, AppError> {
        let response = client
            .post(url)
            .headers(inject_trace_headers())
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::ChromaError(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::ChromaError(format!(
                "Query failed (HTTP {status}): {text}"
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ChromaError(format!("Parse error: {e}")))?;

        let mut results = Vec::new();

        if let Some(ids) = data["ids"][0].as_array() {
            for (i, id) in ids.iter().enumerate() {
                let distance = data["distances"][0][i].as_f64().unwrap_or(1.0);
                let score = 1.0 - distance;
                let metadata = &data["metadatas"][0][i];
                let text = data["documents"][0][i].as_str().unwrap_or("").to_string();

                results.push(ChromaResult {
                    id: id.as_str().unwrap_or("").to_string(),
                    text,
                    document_id: metadata["document_id"].as_str().unwrap_or("").to_string(),
                    chunk_index: metadata["chunk_index"].as_u64().unwrap_or(0) as usize,
                    score,
                });
            }
        }

        Ok(results)
    }

    /// Create a new collection.
    pub async fn create_collection(&self, name: &str) -> Result<(), AppError> {
        tracing::debug!(component = "chroma_client", collection = %name, "create_collection");
        self.get_or_create_collection_id(name).await.map(|_| ())
    }

    async fn get_or_create_collection_id(&self, name: &str) -> Result<String, AppError> {
        let body = json!({"name": name, "get_or_create": true});
        let url = format!("{}/api/v1/collections", self.base_url);

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            match Self::try_get_or_create_collection(&self.client, &url, &body, name).await {
                Ok(collection_id) => return Ok(collection_id),
                Err(e) => {
                    tracing::warn!(
                        component = "chroma_client",
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        "get_or_create_collection.retry"
                    );
                    last_error = Some(e);
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| AppError::ChromaError("All retry attempts exhausted".to_string())))
    }

    async fn try_get_or_create_collection(
        client: &Client,
        url: &str,
        body: &serde_json::Value,
        fallback_name: &str,
    ) -> Result<String, AppError> {
        let response = client
            .post(url)
            .headers(inject_trace_headers())
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::ChromaError(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::ChromaError(format!(
                "Create collection failed (HTTP {status}): {text}"
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ChromaError(format!("Parse error: {e}")))?;

        Ok(Self::collection_identifier(&data, fallback_name))
    }

    fn collection_identifier(data: &serde_json::Value, fallback_name: &str) -> String {
        data.get("id")
            .and_then(|id| id.as_str())
            .filter(|id| !id.is_empty())
            .or_else(|| data.get("name").and_then(|name| name.as_str()))
            .unwrap_or(fallback_name)
            .to_string()
    }

    /// Delete a collection.
    pub async fn delete_collection(&self, name: &str) -> Result<(), AppError> {
        tracing::debug!(component = "chroma_client", collection = %name, "delete_collection");

        let encoded = name.replace('/', "%2F").replace(' ', "%20");
        let url = format!("{}/api/v1/collections/{}", self.base_url, encoded);

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            match Self::try_delete_collection(&self.client, &url).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(
                        component = "chroma_client",
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        "delete_collection.retry"
                    );
                    last_error = Some(e);
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| AppError::ChromaError("All retry attempts exhausted".to_string())))
    }

    async fn try_delete_collection(client: &Client, url: &str) -> Result<(), AppError> {
        let response = client
            .delete(url)
            .headers(inject_trace_headers())
            .send()
            .await
            .map_err(|e| AppError::ChromaError(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::ChromaError(format!(
                "Delete collection failed (HTTP {status}): {text}"
            )));
        }

        Ok(())
    }

    /// Delete documents from a collection by a metadata filter.
    ///
    /// Uses Chroma's `where` filter to delete only entries matching the criteria.
    pub async fn delete_where(
        &self,
        collection: &str,
        filter: &serde_json::Value,
    ) -> Result<(), AppError> {
        tracing::debug!(
            component = "chroma_client",
            collection = %collection,
            filter = %filter,
            "delete_where"
        );

        let body = json!({ "where": filter });

        // Resolve collection name to Chroma UUID for URL path.
        let resolved_id = match self.resolve_collection_id(collection).await {
            Ok(id) => {
                tracing::debug!(
                    component = "chroma_client",
                    collection = %collection,
                    chroma_id = %id,
                    "delete_where.resolved_collection"
                );
                id
            }
            Err(e) => {
                tracing::warn!(
                    component = "chroma_client",
                    error = %e,
                    "delete_where.resolution_failed"
                );
                collection.to_string()
            }
        };
        let url = format!(
            "{}/api/v1/collections/{}/delete",
            self.base_url, resolved_id
        );

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            match Self::try_delete_document(&self.client, &url, &body).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(
                        component = "chroma_client",
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        "delete_where.retry"
                    );
                    last_error = Some(e);
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| AppError::ChromaError("All retry attempts exhausted".to_string())))
    }

    /// Delete documents from a collection by their IDs.
    pub async fn delete_document(&self, collection: &str, ids: &[String]) -> Result<(), AppError> {
        tracing::debug!(
            component = "chroma_client",
            collection = %collection,
            count = ids.len(),
            "delete_document"
        );

        let body = json!({"ids": ids});

        // Resolve collection name to Chroma UUID for URL path.
        let resolved_id = match self.resolve_collection_id(collection).await {
            Ok(id) => {
                tracing::debug!(
                    component = "chroma_client",
                    collection = %collection,
                    chroma_id = %id,
                    "delete_document.resolved_collection"
                );
                id
            }
            Err(e) => {
                tracing::warn!(
                    component = "chroma_client",
                    error = %e,
                    "delete_document.resolution_failed"
                );
                collection.to_string()
            }
        };
        let url = format!(
            "{}/api/v1/collections/{}/delete",
            self.base_url, resolved_id
        );

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            match Self::try_delete_document(&self.client, &url, &body).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(
                        component = "chroma_client",
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        "delete_document.retry"
                    );
                    last_error = Some(e);
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| AppError::ChromaError("All retry attempts exhausted".to_string())))
    }

    async fn try_delete_document(
        client: &Client,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<(), AppError> {
        let response = client
            .post(url)
            .headers(inject_trace_headers())
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::ChromaError(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::ChromaError(format!(
                "Delete failed (HTTP {status}): {text}"
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl super::health::HealthProbe for ChromaClient {
    fn name(&self) -> &'static str {
        "Chroma"
    }

    async fn probe(&self) -> Result<(), AppError> {
        self.health().await
    }
}

impl ChromaClient {
    /// Quick health check — pings the Chroma heartbeat endpoint.
    ///
    /// Single attempt (no retry), 5-second timeout.
    /// Returns `Ok(())` on success, `AppError::ChromaError` on failure.
    pub async fn health(&self) -> Result<(), AppError> {
        tracing::debug!(component = "chroma_client", "health.probe_start");

        let url = format!("{}/api/v1/heartbeat", self.base_url);

        let health_client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| AppError::ChromaError(format!("Failed to build health client: {e}")))?;

        let response = health_client
            .get(&url)
            .headers(inject_trace_headers())
            .send()
            .await
            .map_err(|e| AppError::ChromaError(format!("Chroma heartbeat failed: {e}")))?;

        if response.status().is_success() {
            tracing::debug!(component = "chroma_client", "health.probe_ok");
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            tracing::warn!(
                component = "chroma_client",
                status = %status,
                body = %text,
                "health.probe_error"
            );
            Err(AppError::ChromaError(format!(
                "Chroma heartbeat returned HTTP {status}: {text}"
            )))
        }
    }
}
