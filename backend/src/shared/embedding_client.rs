use std::time::Duration;

use opentelemetry::trace::TraceContextExt;
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

impl EmbeddingClient {
    /// Create a new embedding client pointing at the given base URL.
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for embedding service");

        tracing::debug!(component = "embedding_client", url = %base_url, "client.initialized");

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
            component = "embedding_client",
            text_count = texts.len(),
            total_chars = texts.iter().map(|t| t.len()).sum::<usize>(),
            "embed.request"
        );

        let body = json!({ "texts": &texts });
        let url = format!("{}/embed", self.base_url);

        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            match self.try_embed(&url, &body).await {
                Ok(embeddings) => {
                    tracing::debug!(
                        component = "embedding_client",
                        count = embeddings.len(),
                        dimension = embeddings.first().map(|v| v.len()).unwrap_or(0),
                        "embed.response"
                    );
                    return Ok(embeddings);
                }
                Err(e) => {
                    tracing::warn!(
                        component = "embedding_client",
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        "embed.retry"
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
        let response = self
            .client
            .post(url)
            .headers(inject_trace_headers())
            .json(body)
            .send()
            .await
            .map_err(|e| {
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
