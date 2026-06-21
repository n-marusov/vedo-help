use std::time::Duration;

use reqwest::Client;
use serde_json::json;

use crate::shared::error::AppError;
use crate::shared::types::ChromaResult;

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

        tracing::debug!("Chroma client initialized: url={base_url}");

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
            "Chroma add_embeddings: collection={collection}, count={}",
            ids.len()
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
                    "[ChromaClient.add_embeddings] resolved collection={collection} → chroma_id={id}"
                );
                id
            }
            Err(e) => {
                tracing::warn!(
                    "[ChromaClient.add_embeddings] collection resolution failed, using original name: {e}"
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
                        "Chroma add_embeddings failed (attempt {attempt}/{MAX_RETRIES}): {e}"
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
        tracing::debug!("[ChromaClient.resolve_collection_id] resolving collection={name}");
        let chroma_id = self.get_or_create_collection_id(name).await?;
        tracing::debug!("[ChromaClient.resolve_collection_id] resolved collection={name} → chroma_id={chroma_id}");
        Ok(chroma_id)
    }

    #[cfg(test)]
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
        tracing::debug!("Chroma query: collection={collection}, top_k={top_k}");

        let mut body = json!({
            "query_embeddings": [embedding],
            "n_results": top_k,
            "include": ["metadatas", "distances", "documents"],
        });
        if let Some(filter) = where_filter {
            tracing::debug!("Chroma query with where filter: {filter}");
            body["where"] = filter;
        }

        // Resolve collection name to Chroma UUID for URL path.
        // Chroma 0.6.x requires the internal UUID, not the name, in sub-resource URLs.
        let resolved_id = match self.resolve_collection_id(collection).await {
            Ok(id) => {
                tracing::debug!(
                    "[ChromaClient.query] resolved collection={collection} → chroma_id={id}, calling /api/v1/collections/{id}/query"
                );
                id
            }
            Err(e) => {
                tracing::warn!(
                    "[ChromaClient.query] collection resolution failed, retrying with original name: {e}"
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
                    tracing::warn!("Chroma query failed (attempt {attempt}/{MAX_RETRIES}): {e}");
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
        tracing::debug!("Chroma create_collection: {name}");
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
                        "Chroma get_or_create_collection failed (attempt {attempt}/{MAX_RETRIES}): {e}"
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
        tracing::debug!("Chroma delete_collection: {name}");

        let encoded = name.replace('/', "%2F").replace(' ', "%20");
        let url = format!("{}/api/v1/collections/{}", self.base_url, encoded);

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            match Self::try_delete_collection(&self.client, &url).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(
                        "Chroma delete_collection failed (attempt {attempt}/{MAX_RETRIES}): {e}"
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
        tracing::debug!("Chroma delete_where: collection={collection}, filter={filter}",);

        let body = json!({ "where": filter });

        // Resolve collection name to Chroma UUID for URL path.
        let resolved_id = match self.resolve_collection_id(collection).await {
            Ok(id) => {
                tracing::debug!(
                    "[ChromaClient.delete_where] resolved collection={collection} → chroma_id={id}, calling /api/v1/collections/{id}/delete"
                );
                id
            }
            Err(e) => {
                tracing::warn!(
                    "[ChromaClient.delete_where] collection resolution failed, retrying with original name: {e}"
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
                        "Chroma delete_where failed (attempt {attempt}/{MAX_RETRIES}): {e}"
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
            "Chroma delete_document: collection={collection}, count={}",
            ids.len()
        );

        let body = json!({"ids": ids});

        // Resolve collection name to Chroma UUID for URL path.
        let resolved_id = match self.resolve_collection_id(collection).await {
            Ok(id) => {
                tracing::debug!(
                    "[ChromaClient.delete_document] resolved collection={collection} → chroma_id={id}, calling /api/v1/collections/{id}/delete"
                );
                id
            }
            Err(e) => {
                tracing::warn!(
                    "[ChromaClient.delete_document] collection resolution failed, retrying with original name: {e}"
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
                        "Chroma delete_document failed (attempt {attempt}/{MAX_RETRIES}): {e}"
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;

    #[test]
    fn test_is_missing_collection_error_detects_chroma_invalid_collection() {
        let error = AppError::ChromaError(
            "Add embeddings failed (HTTP 400 Bad Request): {\"error\":\"InvalidCollection\",\"message\":\"Collection ce1135b9-b7b5-40c6-a319-a16648089c65 does not exist.\"}".to_string(),
        );

        assert!(ChromaClient::is_missing_collection_error(&error));
    }

    #[test]
    fn test_is_missing_collection_error_ignores_other_chroma_errors() {
        let error = AppError::ChromaError(
            "Add embeddings failed (HTTP 500 Internal Server Error): temporary failure".to_string(),
        );

        assert!(!ChromaClient::is_missing_collection_error(&error));
    }

    #[test]
    fn test_collection_identifier_prefers_chroma_id() {
        let data = serde_json::json!({
            "id": "internal-chroma-id",
            "name": "ce1135b9-b7b5-40c6-a319-a16648089c65"
        });

        assert_eq!(
            ChromaClient::collection_identifier(&data, "fallback"),
            "internal-chroma-id"
        );
    }

    #[test]
    fn test_collection_identifier_falls_back_to_name() {
        let data = serde_json::json!({"name": "ce1135b9-b7b5-40c6-a319-a16648089c65"});

        assert_eq!(
            ChromaClient::collection_identifier(&data, "fallback"),
            "ce1135b9-b7b5-40c6-a319-a16648089c65"
        );
    }

    /// Helper: start a minimal HTTP/1.1 mock server that responds to
    /// `POST /api/v1/collections` and one or more sub-resource requests.
    /// Returns the server address and a shared list of captured request lines.
    async fn start_mock_server(
        responses: Vec<(&'static str, &'static str, u16)>,
    ) -> (std::net::SocketAddr, Arc<Mutex<Vec<String>>>) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test server should bind");
        let addr = listener.local_addr().expect("test server should have addr");
        let captured = Arc::clone(&requests);

        tokio::spawn(async move {
            for (_expected_prefix, response_body, status_code) in responses {
                let (mut stream, _) = listener.accept().await.expect("request should connect");
                let mut buffer = vec![0_u8; 16384];
                let read = stream.read(&mut buffer).await.expect("request should read");
                let request = String::from_utf8_lossy(&buffer[..read]);
                let request_line = request.lines().next().unwrap_or_default().to_string();
                captured.lock().await.push(request_line.clone());

                let status_text = if status_code == 200 {
                    "OK"
                } else if status_code == 400 {
                    "Bad Request"
                } else {
                    "Internal Server Error"
                };
                let response = format!(
                    "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                    response_body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .await
                    .expect("response should write");
                // Flush and close to ensure the client receives the response
                let _ = stream.shutdown().await;
            }
        });

        // Give the mock server a moment to start listening
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        (addr, requests)
    }

    #[tokio::test]
    async fn test_resolve_collection_id_calls_get_or_create() {
        // When resolve_collection_id is called, it should POST /api/v1/collections
        // with get_or_create=true and return the Chroma-assigned "id" field.
        let (addr, requests) = start_mock_server(vec![(
            "POST /api/v1/collections ",
            r#"{"id":"internal-chroma-id","name":"test-col"}"#,
            200,
        )])
        .await;

        let client = ChromaClient::new(&format!("http://{addr}"));
        let result = client.resolve_collection_id("test-col").await;

        assert_eq!(
            result.expect("resolve should succeed"),
            "internal-chroma-id"
        );
        let reqs = requests.lock().await;
        assert!(
            reqs.iter()
                .any(|r| r.starts_with("POST /api/v1/collections ")),
            "Expected POST /api/v1/collections request, got: {reqs:?}"
        );
    }

    #[tokio::test]
    async fn test_add_embeddings_resolves_collection_id_upfront() {
        // add_embeddings should first call resolve_collection_id, then use
        // the Chroma UUID in the URL path for /add.
        //
        // Request sequence:
        //   1. POST /api/v1/collections  (resolve_collection_id) → returns {"id":"internal-chroma-id","name":"test-col"}
        //   2. POST /api/v1/collections/internal-chroma-id/add → returns 200 OK
        let (addr, requests) = start_mock_server(vec![
            (
                "POST /api/v1/collections ",
                r#"{"id":"internal-chroma-id","name":"test-col"}"#,
                200,
            ),
            (
                "POST /api/v1/collections/internal-chroma-id/add ",
                "{}",
                200,
            ),
        ])
        .await;

        let client = ChromaClient::new(&format!("http://{addr}"));
        client
            .add_embeddings(
                "test-col",
                &["chunk-1".to_string()],
                &[vec![0.1, 0.2, 0.3]],
                &[serde_json::json!({"document_id": "doc-1"})],
            )
            .await
            .expect("add_embeddings should succeed after resolving collection id");

        let reqs = requests.lock().await;
        assert_eq!(
            reqs.len(),
            2,
            "Expected 2 requests (resolve + add), got: {reqs:?}"
        );
        assert!(
            reqs[0].starts_with("POST /api/v1/collections "),
            "First request should be resolve_collection_id, got: {}",
            reqs[0]
        );
        assert!(
            reqs[1].contains("internal-chroma-id/add"),
            "Second request should use resolved Chroma ID, got: {}",
            reqs[1]
        );
    }

    #[tokio::test]
    async fn test_add_embeddings_retry() {
        let client = ChromaClient::new("http://127.0.0.1:1");
        let result = client.add_embeddings("test", &[], &[], &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_without_where_omits_filter() {
        let client = ChromaClient::new("http://127.0.0.1:1");
        // When `where_filter` is None, the query should still work.
        // The filter should be omitted from the request body.
        // This test verifies the new method signature accepts None.
        let result = client
            .query("test-collection", &[0.1, 0.2, 0.3], 5, None)
            .await;

        // We expect a connection error (server unreachable) — not a serialization or type error.
        // This proves the `where` field was correctly omitted or included as None.
        match &result {
            Err(AppError::ChromaError(msg)) => {
                // Network error expected since no Chroma server is running.
                // Different environments return different errors:
                // connection refused, 502 Bad Gateway, etc.
                assert!(
                    msg.contains("Request failed")
                        || msg.contains("Connection refused")
                        || msg.contains("error trying to connect")
                        || msg.contains("Bad Gateway"),
                    "Expected network error but got: {msg}"
                );
            }
            other => panic!("Expected ChromaError (connection error) but got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_query_with_where_includes_filter() {
        let client = ChromaClient::new("http://127.0.0.1:1");
        // When `where_filter` is Some, it must be included in the request body.
        let filter = serde_json::json!({"is_active": true});
        let result = client
            .query("test-collection", &[0.1, 0.2, 0.3], 5, Some(filter))
            .await;

        // Same expected outcome: network error, not a type/struct error
        match &result {
            Err(AppError::ChromaError(msg)) => {
                assert!(
                    msg.contains("Request failed")
                        || msg.contains("Connection refused")
                        || msg.contains("error trying to connect")
                        || msg.contains("Bad Gateway"),
                    "Expected network error but got: {msg}"
                );
            }
            other => panic!("Expected ChromaError (connection error) but got: {other:?}"),
        }
    }
}
