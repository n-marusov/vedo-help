use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use lru::LruCache;
use opentelemetry::trace::TraceContextExt;
use reqwest::Client;
use serde_json::json;

use crate::config::AppConfig;
use crate::shared::error::AppError;

/// HTTP client for RouterAI embeddings API.
///
/// Calls `POST {base_url}/embeddings` with OpenAI-compatible format and returns
/// the embedding vectors with retry on 5xx / rate-limit errors.
/// 401/402 errors (auth / insufficient balance) are NOT retried.
///
/// Includes an in-memory LRU cache to avoid redundant API calls for repeated inputs.
#[derive(Debug)]
pub struct EmbeddingClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    /// In-memory LRU cache mapping text-hash → embedding vectors
    cache: Mutex<LruCache<u64, Vec<Vec<f32>>>>,
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

/// Compute a simple hash for cache keying.
fn hash_texts(texts: &[String]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    texts.len().hash(&mut hasher);
    for t in texts {
        t.hash(&mut hasher);
    }
    hasher.finish()
}

// Manual Clone implementation (Mutex<LruCache<...>> does not derive Clone)
impl Clone for EmbeddingClient {
    fn clone(&self) -> Self {
        let cache = self.cache.lock().expect("embedding cache lock").clone();
        Self {
            client: self.client.clone(),
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            cache: Mutex::new(cache),
        }
    }
}

impl EmbeddingClient {
    /// Create a new embedding client pointing at the given RouterAI-compatible base URL.
    ///
    /// # Deprecated
    /// Use `from_config()` instead, which loads API key, model, and cache settings from AppConfig.
    /// This constructor uses the given URL with an empty API key and the default embedding model.
    #[deprecated(note = "Use EmbeddingClient::from_config instead")]
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for embedding service");

        let cache_size = NonZeroUsize::new(1000).expect("1000 is non-zero");

        tracing::warn!(
            component = "embedding_client",
            url = %base_url,
            "EmbeddingClient::new() is deprecated, use from_config() instead"
        );

        Self {
            client,
            api_key: String::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: "sentence-transformers/all-minilm-l6-v2".to_string(),
            cache: Mutex::new(LruCache::new(cache_size)),
        }
    }
    pub fn from_config(config: &AppConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for embedding service");

        let cache_size = NonZeroUsize::new(config.embedding_cache_size)
            .unwrap_or(NonZeroUsize::new(1000).expect("1000 is non-zero"));

        tracing::debug!(
            component = "embedding_client",
            model = %config.embedding_model,
            url = %config.embedding_base_url,
            cache_size = cache_size.get(),
            "embedding_client.from_config"
        );

        Self {
            client,
            api_key: config.embedding_api_key.clone(),
            base_url: config.embedding_base_url.trim_end_matches('/').to_string(),
            model: config.embedding_model.clone(),
            cache: Mutex::new(LruCache::new(cache_size)),
        }
    }

    /// Embed one or more text strings, returning a vector of embedding vectors.
    ///
    /// Uses an in-memory LRU cache to avoid redundant API calls.
    /// Retries on 5xx / 429 responses and connection errors up to `MAX_RETRIES` times.
    /// 401/402 errors are NOT retried (auth / balance).
    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        tracing::debug!(
            component = "embedding_client",
            text_count = texts.len(),
            total_chars = texts.iter().map(|t| t.len()).sum::<usize>(),
            model = %self.model,
            "embed.request"
        );

        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Check cache
        let cache_key = hash_texts(&texts);
        {
            let mut cache = self.cache.lock().expect("embedding cache lock");
            if let Some(cached) = cache.get(&cache_key) {
                let result = cached.clone();
                let cache_size = cache.len();
                // Drop cache lock before logging
                drop(cache);
                tracing::debug!(
                    component = "embedding_client",
                    text_count = texts.len(),
                    cache_size = cache_size,
                    "embed.cache_hit"
                );
                return Ok(result);
            }

            tracing::debug!(
                component = "embedding_client",
                text_count = texts.len(),
                cache_size = cache.len(),
                "embed.cache_miss"
            );
        }

        // Build OpenAI-compatible request
        let body = json!({
            "model": self.model,
            "input": &texts,
        });
        let url = format!("{}/embeddings", self.base_url);

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

                    // Store in cache
                    {
                        let mut cache = self.cache.lock().expect("embedding cache lock");
                        cache.put(cache_key, embeddings.clone());
                    }

                    return Ok(embeddings);
                }
                Err(e) => {
                    let is_retryable = matches!(&e, AppError::EmbeddingError(msg) if {
                        !msg.contains("401") && !msg.contains("402")
                    });
                    if !is_retryable {
                        tracing::error!(
                            component = "embedding_client",
                            error = %e,
                            attempt = attempt,
                            "embed.non_retryable_error"
                        );
                        return Err(e);
                    }

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
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .headers(inject_trace_headers())
            .json(body)
            .send()
            .await
            .map_err(|e| {
                AppError::EmbeddingError(format!("Connection to embedding API failed: {e}"))
            })?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::EmbeddingError(format!(
                "Embedding API returned HTTP {status}: {text}"
            )));
        }

        // Parse OpenAI-compatible response:
        // {
        //   "data": [
        //     { "embedding": [0.xx, ...], "index": 0 },
        //     { "embedding": [0.xx, ...], "index": 1 }
        //   ],
        //   "model": "...",
        //   "usage": { "prompt_tokens": N, "total_tokens": N }
        // }
        let data: serde_json::Value = response.json().await.map_err(|e| {
            AppError::EmbeddingError(format!("Failed to parse embedding response: {e}"))
        })?;

        let data_array = data["data"].as_array().ok_or_else(|| {
            AppError::EmbeddingError("Embedding response missing 'data' array".to_string())
        })?;

        // Collect into (index, embedding) pairs and sort by index
        let mut indexed: Vec<(usize, Vec<f32>)> = data_array
            .iter()
            .map(|item| {
                let index = item["index"].as_u64().unwrap_or(0) as usize;
                let embedding: Vec<f32> = item["embedding"]
                    .as_array()
                    .ok_or_else(|| {
                        AppError::EmbeddingError(
                            "Invalid embedding vector format in data item".to_string(),
                        )
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
                    .collect::<Result<Vec<f32>, AppError>>()?;
                Ok((index, embedding))
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        // Sort by index to ensure consistent ordering
        indexed.sort_by_key(|(idx, _)| *idx);

        let embeddings: Vec<Vec<f32>> = indexed.into_iter().map(|(_, emb)| emb).collect();

        Ok(embeddings)
    }
}

#[async_trait]
impl super::health::HealthProbe for EmbeddingClient {
    fn name(&self) -> &'static str {
        "Embedding"
    }

    async fn probe(&self) -> Result<(), AppError> {
        self.health().await
    }
}

impl EmbeddingClient {
    /// Quick health check — pings the RouterAI API base URL.
    ///
    /// Single attempt (no retry), 10-second timeout.
    /// Returns `Ok(())` on any response (server reachable),
    /// `AppError::EmbeddingError` on connection failure.
    pub async fn health(&self) -> Result<(), AppError> {
        tracing::debug!(component = "embedding_client", "health.probe_start");

        let health_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| AppError::EmbeddingError(format!("Failed to build health client: {e}")))?;

        let response = health_client
            .get(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .headers(inject_trace_headers())
            .send()
            .await;

        match response {
            Ok(_) => {
                tracing::debug!(component = "embedding_client", "health.probe_ok");
                Ok(())
            }
            Err(e) => {
                tracing::warn!(
                    component = "embedding_client",
                    error = %e,
                    "health.probe_error"
                );
                Err(AppError::EmbeddingError(format!(
                    "Embedding health check failed: {e}"
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_client_from_config() {
        let config = AppConfig::from_env();
        let client = EmbeddingClient::from_config(&config);
        assert_eq!(client.model, config.embedding_model);
        assert_eq!(client.base_url, config.embedding_base_url);
        assert_eq!(client.api_key, config.embedding_api_key);
    }

    #[test]
    fn test_hash_texts_consistency() {
        let texts = vec!["hello".to_string(), "world".to_string()];
        let h1 = hash_texts(&texts);
        let h2 = hash_texts(&texts);
        assert_eq!(h1, h2, "Same input should produce same hash");
    }

    #[test]
    fn test_hash_texts_different_inputs() {
        let t1 = hash_texts(&["hello".to_string()]);
        let t2 = hash_texts(&["world".to_string()]);
        assert_ne!(t1, t2, "Different inputs should produce different hashes");
    }

    #[test]
    fn test_client_has_cache() {
        let config = AppConfig::from_env();
        let client = EmbeddingClient::from_config(&config);
        assert_eq!(
            client.cache.lock().expect("lock").len(),
            0,
            "Cache should start empty"
        );
    }

    #[tokio::test]
    async fn test_embed_empty_texts() {
        let config = AppConfig::from_env();
        let client = EmbeddingClient::from_config(&config);

        // This should succeed quickly without any API call (empty check)
        let result = client.embed(Vec::new()).await;
        assert!(result.is_ok());
        let embeddings = result.unwrap();
        assert!(
            embeddings.is_empty(),
            "Empty input should return empty result"
        );
    }
}
