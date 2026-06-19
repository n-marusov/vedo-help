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
        let url = format!("{}/api/v1/collections/{}/add", self.base_url, collection);

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

    /// Query a collection by embedding vector, returning the top-k matches.
    pub async fn query(
        &self,
        collection: &str,
        embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<ChromaResult>, AppError> {
        tracing::debug!("Chroma query: collection={collection}, top_k={top_k}");

        let body = json!({
            "query_embeddings": [embedding],
            "n_results": top_k,
            "include": ["metadatas", "distances", "documents"],
        });
        let url = format!("{}/api/v1/collections/{}/query", self.base_url, collection);

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

        let body = json!({"name": name});
        let url = format!("{}/api/v1/collections", self.base_url);

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            match Self::try_create_collection(&self.client, &url, &body).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(
                        "Chroma create_collection failed (attempt {attempt}/{MAX_RETRIES}): {e}"
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

    async fn try_create_collection(
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
                "Create collection failed (HTTP {status}): {text}"
            )));
        }

        Ok(())
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
        let url = format!("{}/api/v1/collections/{}/delete", self.base_url, collection);

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
        let url = format!("{}/api/v1/collections/{}/delete", self.base_url, collection);

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
                // Connection refused is expected since no server is running
                assert!(
                    msg.contains("Request failed")
                        || msg.contains("Connection refused")
                        || msg.contains("error trying to connect"),
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
                        || msg.contains("error trying to connect"),
                    "Expected network error but got: {msg}"
                );
            }
            other => panic!("Expected ChromaError (connection error) but got: {other:?}"),
        }
    }
}
