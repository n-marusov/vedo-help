use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::RwLock;

use crate::modules::settings::models::ModelOption;

/// A single model entry from the RouterAI /api/v1/models endpoint.
#[derive(Debug, Deserialize)]
struct RouterAIModel {
    id: String,
    pricing: serde_json::Value,
}

/// Response wrapper from the RouterAI models endpoint.
#[derive(Debug, Deserialize)]
struct RouterAIModelsResponse {
    data: Vec<RouterAIModel>,
}

/// Parsed pricing info for a single model.
#[derive(Debug, Clone)]
pub struct ModelPricing {
    /// Formatted price string for display, e.g. "301 ₽/1M input, 1,506 ₽/1M output"
    /// or "0.25 ₽/search unit" for dedicated rerankers.
    pub display: String,
}

/// Thread-safe cache of model pricing fetched asynchronously from RouterAI.
#[derive(Debug, Clone)]
pub struct PricingCache {
    inner: Arc<RwLock<HashMap<String, ModelPricing>>>,
    client: reqwest::Client,
    base_url: String,
}

impl PricingCache {
    /// Create a new empty pricing cache.
    /// Call `start_background_refresh` to begin periodic fetching.
    pub fn new(base_url: &str) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create reqwest client for pricing"),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Fetch pricing from RouterAI and update the cache.
    pub async fn refresh(&self) {
        let url = format!("{}/models", self.base_url);
        tracing::debug!(component = "pricing", url = %url, "pricing.refresh.start");

        match self.client.get(&url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    tracing::warn!(
                        component = "pricing",
                        status = %response.status(),
                        "pricing.refresh.http_error"
                    );
                    return;
                }
                match response.json::<RouterAIModelsResponse>().await {
                    Ok(api_response) => {
                        let mut new_cache = HashMap::new();
                        for model in api_response.data {
                            if let Some(pricing) = parse_pricing(&model.pricing) {
                                new_cache.insert(model.id, pricing);
                            }
                        }
                        let count = new_cache.len();
                        *self.inner.write().await = new_cache;
                        tracing::info!(
                            component = "pricing",
                            model_count = count,
                            "pricing.refresh.completed"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            component = "pricing",
                            error = %e,
                            "pricing.refresh.parse_error"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    component = "pricing",
                    error = %e,
                    "pricing.refresh.failed"
                );
            }
        }
    }

    /// Format a price-per-token value (RUB) into ₽/1M tokens.
    fn format_per_token(prompt: f64, completion: f64) -> String {
        let prompt_price = prompt * 1_000_000.0;
        let completion_price = completion * 1_000_000.0;
        format!(
            "{} ₽/1M input, {} ₽/1M output",
            Self::fmt_price(prompt_price),
            Self::fmt_price(completion_price),
        )
    }

    /// Format a price number with comma thousands separator.
    fn fmt_price(price: f64) -> String {
        if price < 0.01 {
            // Essentially zero: just show 0
            "0".to_string()
        } else if price < 1.0 {
            // Very small prices: show one decimal, e.g. 0.5 ₽/1M
            format!("{:.1}", price)
        } else {
            let rounded = price.round() as i64;
            let s = rounded.to_string();
            let mut result = String::new();
            for (i, c) in s.chars().rev().enumerate() {
                if i > 0 && i % 3 == 0 {
                    result.push(',');
                }
                result.push(c);
            }
            result.chars().rev().collect()
        }
    }

    /// Get pricing display for a model by its ID.
    pub async fn get_display(&self, model_id: &str) -> Option<String> {
        let cache = self.inner.read().await;
        cache.get(model_id).map(|p| p.display.clone())
    }

    /// Enrich a slice of ModelOption with pricing from the cache.
    /// Non-blocking: returns immediately with current cache state.
    pub async fn enrich_options(&self, options: &mut [ModelOption]) {
        let cache = self.inner.read().await;
        for opt in options.iter_mut() {
            if let Some(pricing) = cache.get(&opt.value) {
                opt.pricing = Some(pricing.display.clone());
            }
        }
    }

    /// Spawn a background task that refreshes pricing periodically.
    /// The first refresh happens after `initial_delay` to avoid blocking app startup.
    /// Subsequent refreshes happen every `interval`.
    pub fn start_background_refresh(
        self,
        initial_delay: std::time::Duration,
        interval: std::time::Duration,
    ) {
        tokio::spawn(async move {
            // Wait before first fetch so the app can start quickly
            tokio::time::sleep(initial_delay).await;
            self.refresh().await;

            // Periodic refresh
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                self.refresh().await;
            }
        });
    }
}

/// Parse pricing JSON from the RouterAI API response.
/// Handles both token-based pricing (prompt/completion) and search-unit pricing.
fn parse_pricing(pricing: &serde_json::Value) -> Option<ModelPricing> {
    // Try search_unit pricing first (dedicated rerankers)
    if let Some(price) = pricing.get("search_units").and_then(|v| v.as_f64()) {
        return Some(ModelPricing {
            display: format!("{:.2} ₽/search unit", price),
        });
    }

    // Token-based pricing
    let prompt = pricing.get("prompt").and_then(|v| v.as_f64())?;
    let completion = pricing
        .get("completion")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    Some(ModelPricing {
        display: PricingCache::format_per_token(prompt, completion),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pricing_token_based() {
        // Claude Sonnet 4.6 prices from RouterAI API
        let json: serde_json::Value =
            serde_json::from_str(r#"{"prompt": 0.00029689062, "completion": 0.0014844531}"#)
                .unwrap();
        let pricing = parse_pricing(&json).expect("should parse");
        // Price should be "297 ₽/1M input, 1,484 ₽/1M output" (rounded from 296.89 and 1484.45)
        assert!(pricing.display.contains("₽/1M input"));
        assert!(pricing.display.contains("₽/1M output"));
        assert!(pricing.display.contains("297"));
        assert!(pricing.display.contains("1,484"));
    }

    #[test]
    fn test_parse_pricing_embedding() {
        // all-MiniLM-L6-v2: 0.4948 ₽/1M → "0.5"
        let json: serde_json::Value =
            serde_json::from_str(r#"{"prompt": 4.948177e-7, "completion": 0.0}"#).unwrap();
        let pricing = parse_pricing(&json).expect("should parse");
        // "0.5 ₽/1M input" (only prompt, no completion for embeddings)
        assert!(pricing.display.contains("0.5"));
        assert!(pricing.display.contains("₽/1M input"));
    }

    #[test]
    fn test_parse_pricing_search_unit() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"search_units": 0.24740885}"#).unwrap();
        let pricing = parse_pricing(&json).expect("should parse");
        assert!(pricing.display.contains("search unit"));
        assert!(pricing.display.contains("0.25"));
    }

    #[test]
    fn test_format_per_token() {
        // Claude Sonnet 4.6
        let result = PricingCache::format_per_token(0.00029689062, 0.0014844531);
        assert_eq!(result, "297 ₽/1M input, 1,484 ₽/1M output");
    }

    #[test]
    fn test_format_per_token_small() {
        // DeepSeek V4 Flash
        let result = PricingCache::format_per_token(8.9067186e-6, 0.0000178134372);
        assert_eq!(result, "9 ₽/1M input, 18 ₽/1M output");
    }

    #[test]
    fn test_format_per_token_embedding() {
        // all-MiniLM-L6-v2: tiny price, should show decimal
        let result = PricingCache::format_per_token(4.948177e-7, 0.0);
        assert_eq!(result, "0.5 ₽/1M input, 0 ₽/1M output");
    }

    #[test]
    fn test_enrich_options_empty_cache() {
        // When cache is empty, enrich_options should leave pricing as None
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let cache = PricingCache::new("https://routerai.ru/api/v1");
            let mut opts = vec![
                ModelOption::pair("test/model-a", "Model A"),
                ModelOption::pair("test/model-b", "Model B"),
            ];
            cache.enrich_options(&mut opts).await;
            assert!(opts[0].pricing.is_none());
            assert!(opts[1].pricing.is_none());
        });
    }

    #[test]
    fn test_enrich_options_with_pricing() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let cache = PricingCache::new("https://routerai.ru/api/v1");
            // Manually insert pricing into cache
            {
                let mut inner = cache.inner.write().await;
                inner.insert(
                    "test/model-a".to_string(),
                    ModelPricing {
                        display: "10 ₽/1M input, 20 ₽/1M output".to_string(),
                    },
                );
            }
            let mut opts = vec![
                ModelOption::pair("test/model-a", "Model A"),
                ModelOption::pair("test/model-b", "Model B"),
            ];
            cache.enrich_options(&mut opts).await;
            // Model A should have pricing from cache
            assert_eq!(
                opts[0].pricing.as_deref(),
                Some("10 ₽/1M input, 20 ₽/1M output")
            );
            // Model B not in cache, should be None
            assert!(opts[1].pricing.is_none());
        });
    }
}
