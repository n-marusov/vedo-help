use std::time::Duration;

use reqwest::Client;
use serde_json::json;

use crate::shared::error::AppError;

/// HTTP client for the Python embedding service.
///
/// Calls `POST {base_url}/embed` with `{"texts": [...]}` and returns
/// the embedding vectors with retry on 5xx / connection errors.
#[derive(Clone, Debug)]
pub struct EmbeddingClient {
    client: Client,
    base_url: String,
}

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 500;

impl EmbeddingClient {
    /// Create a new embedding client pointing at the given base URL.
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for embedding service");

        tracing::debug!("Embedding client initialized: url={base_url}");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Embed one or more text strings, returning a vector of embedding vectors.
    ///
    /// Retries on 5xx responses and connection errors up to `MAX_RETRIES` times.
    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        tracing::debug!(
            "Embedding request: count={}, chars={}",
            texts.len(),
            texts.iter().map(|t| t.len()).sum::<usize>()
        );

        let body = json!({ "texts": &texts });
        let url = format!("{}/embed", self.base_url);

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            match self.try_embed(&url, &body).await {
                Ok(embeddings) => {
                    tracing::debug!(
                        "Embedding response: count={}, dim={}",
                        embeddings.len(),
                        embeddings.first().map(|v| v.len()).unwrap_or(0)
                    );
                    return Ok(embeddings);
                }
                Err(e) => {
                    tracing::warn!(
                        "Embedding request failed (attempt {attempt}/{MAX_RETRIES}): {e}"
                    );
                    last_error = Some(e);
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            AppError::EmbeddingError("All retry attempts exhausted".to_string())
        }))
    }

    async fn try_embed(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<Vec<Vec<f32>>, AppError> {
        let response = self.client.post(url).json(body).send().await.map_err(|e| {
            AppError::EmbeddingError(format!("Connection to embedding service failed: {e}"))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::EmbeddingError(format!(
                "Embedding service returned HTTP {status}: {text}"
            )));
        }

        let data: serde_json::Value = response.json().await.map_err(|e| {
            AppError::EmbeddingError(format!("Failed to parse embedding response: {e}"))
        })?;

        let embeddings: Vec<Vec<f32>> = data["embeddings"]
            .as_array()
            .ok_or_else(|| {
                AppError::EmbeddingError(
                    "Embedding response missing 'embeddings' field".to_string(),
                )
            })?
            .iter()
            .map(|arr| {
                arr.as_array()
                    .ok_or_else(|| {
                        AppError::EmbeddingError("Invalid embedding vector format".to_string())
                    })?
                    .iter()
                    .map(|v| {
                        v.as_f64()
                            .ok_or_else(|| {
                                AppError::EmbeddingError(
                                    "Non-numeric value in embedding vector".to_string(),
                                )
                            })
                            .map(|f| f as f32)
                    })
                    .collect::<Result<Vec<f32>, AppError>>()
            })
            .collect::<Result<Vec<Vec<f32>>, AppError>>()?;

        Ok(embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_client_creation() {
        let client = EmbeddingClient::new("http://localhost:8001");
        assert_eq!(client.base_url, "http://localhost:8001");
    }
}
