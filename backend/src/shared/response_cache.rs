use chrono::{DateTime, Utc};
use lru::LruCache;
use sha2::{Digest, Sha256};
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::modules::query::models::SourceRef;

/// A cached LLM query response with its sources and metadata.
#[derive(Debug, Clone)]
pub struct CachedResponse {
    /// The full LLM answer text.
    pub answer: String,
    /// Source references used to generate the answer.
    pub sources: Vec<SourceRef>,
    /// ISO 8601 timestamp when this entry was cached.
    pub cached_at: DateTime<Utc>,
}

/// In-memory LRU cache for LLM query responses.
///
/// Wraps an `lru::LruCache` behind a `tokio::sync::Mutex` for thread-safe
/// concurrent access. Entries are evicted by TTL expiry (checked on lookup)
/// or by LRU order when the capacity is reached.
#[derive(Clone, Debug)]
pub struct ResponseCache {
    inner: Arc<Mutex<LruCache<String, CachedResponse>>>,
    ttl_secs: u64,
}

impl ResponseCache {
    /// Create a new `ResponseCache` with the given capacity and TTL.
    ///
    /// `max_entries` sets the LRU cache capacity. `ttl_secs` controls how long
    /// entries remain valid before they are considered expired.
    pub fn new(max_entries: usize, ttl_secs: u64) -> Self {
        let capacity =
            NonZeroUsize::new(max_entries.max(1)).unwrap_or(NonZeroUsize::new(100).unwrap());
        tracing::info!(
            component = "response_cache",
            capacity = max_entries,
            ttl_secs = ttl_secs,
            "cache.initialized"
        );
        Self {
            inner: Arc::new(Mutex::new(LruCache::new(capacity))),
            ttl_secs,
        }
    }

    /// Compute a normalized cache key from a query string and collection ID.
    ///
    /// The key is a SHA-256 hex hash of `"{normalized_query}:{collection_id}"`
    /// where the query is trimmed, lowercased, and runs of whitespace collapsed.
    pub fn make_key(query: &str, collection_id: &uuid::Uuid) -> String {
        let normalized = query
            .trim()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let input = format!("{}:{}", normalized, collection_id);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Look up a cached response by key.
    ///
    /// Returns `Some(CachedResponse)` if the entry exists and its TTL has not
    /// expired. Returns `None` if the entry is missing or expired (expired
    /// entries are evicted automatically).
    pub async fn get(&self, key: &str) -> Option<CachedResponse> {
        let mut cache = self.inner.lock().await;
        let cache_size = cache.len();

        if let Some(entry) = cache.get(key) {
            let age_secs = (Utc::now() - entry.cached_at).num_seconds() as u64;
            if age_secs < self.ttl_secs {
                tracing::debug!(
                    component = "response_cache",
                    cache_size = cache_size,
                    age_secs = age_secs,
                    "cache.hit"
                );
                return Some(entry.clone());
            }
            // Entry expired — evict it
            let key_for_evict = key.to_string();
            cache.pop(&key_for_evict);
            tracing::warn!(
                component = "response_cache",
                key = %key,
                age_secs = age_secs,
                "cache.evict_expired"
            );
        } else {
            tracing::debug!(
                component = "response_cache",
                cache_size = cache_size,
                "cache.miss"
            );
        }

        None
    }

    /// Store a response in the cache.
    ///
    /// If the cache is at capacity, the least recently used entry is evicted
    /// automatically by `LruCache`.
    pub async fn set(&self, key: String, value: CachedResponse) {
        let mut cache = self.inner.lock().await;
        let cache_size = cache.len();
        let is_evicting = cache_size >= cache.cap().get();

        cache.put(key.clone(), value);

        if is_evicting {
            tracing::info!(
                component = "response_cache",
                cache_size = cache_size,
                "cache.evict_lru"
            );
        }
        tracing::info!(
            component = "response_cache",
            key = %key,
            "cache.store"
        );
    }

    /// Returns the current number of entries in the cache.
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    /// Returns `true` if the cache is empty.
    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }

    /// Returns the maximum capacity of the cache.
    pub fn capacity(&self) -> usize {
        self.inner.try_lock().map(|c| c.cap().get()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_response(text: &str) -> CachedResponse {
        CachedResponse {
            answer: text.to_string(),
            sources: Vec::new(),
            cached_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let cache = ResponseCache::new(100, 300);
        let collection_id = Uuid::new_v4();
        let key = ResponseCache::make_key("What is Rust?", &collection_id);
        cache
            .set(key.clone(), make_response("Rust is a systems language."))
            .await;
        let result = cache.get(&key).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().answer, "Rust is a systems language.");
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = ResponseCache::new(100, 300);
        let result = cache.get("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_ttl_expiry() {
        let cache = ResponseCache::new(100, 0); // TTL = 0 means immediate expiry
        let collection_id = Uuid::new_v4();
        let key = ResponseCache::make_key("test", &collection_id);
        cache.set(key.clone(), make_response("test answer")).await;
        // Small sleep to ensure TTL has elapsed
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let result = cache.get(&key).await;
        assert!(result.is_none(), "Entry should have expired");
    }

    #[tokio::test]
    async fn test_cache_lru_eviction() {
        let cache = ResponseCache::new(2, 300); // Only 2 entries
        let collection_id = Uuid::new_v4();
        let key1 = ResponseCache::make_key("query1", &collection_id);
        let key2 = ResponseCache::make_key("query2", &collection_id);
        let key3 = ResponseCache::make_key("query3", &collection_id);

        cache.set(key1.clone(), make_response("answer1")).await;
        cache.set(key2.clone(), make_response("answer2")).await;
        cache.set(key3.clone(), make_response("answer3")).await;

        // key1 should be evicted (oldest)
        assert!(cache.get(&key1).await.is_none());
        // key2 and key3 should still exist
        assert!(cache.get(&key2).await.is_some());
        assert!(cache.get(&key3).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_key_normalization() {
        let collection_id = Uuid::new_v4();
        let key1 = ResponseCache::make_key("  What   is   Rust?  ", &collection_id);
        let key2 = ResponseCache::make_key("what is rust?", &collection_id);
        assert_eq!(
            key1, key2,
            "Same query with different whitespace/case should produce same key"
        );
    }

    #[tokio::test]
    async fn test_cache_len_and_capacity() {
        let cache = ResponseCache::new(10, 300);
        assert_eq!(cache.capacity(), 10);
        assert_eq!(cache.len().await, 0);

        let collection_id = Uuid::new_v4();
        for i in 0..5 {
            let key = ResponseCache::make_key(&format!("query{i}"), &collection_id);
            cache.set(key, make_response(&format!("answer{i}"))).await;
        }
        assert_eq!(cache.len().await, 5);
    }
}
