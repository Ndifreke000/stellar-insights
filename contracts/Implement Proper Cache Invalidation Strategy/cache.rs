/// cache.rs
///
/// Enhanced `CacheManager` that adds:
///
///   - LRU eviction backed by an `IndexMap` (insertion-ordered hash map)
///   - Per-entry TTL with lazy expiry on read
///   - `invalidate(key)` and `invalidate_matching(predicate)` for event-driven cleanup
///   - `evict_lru(n)` and `evict_older_than(age)` called by the invalidation worker
///   - `warm()` for startup cache preloading
///   - Integration with `CacheMetrics` from `cache::invalidation`
///
/// # Concurrency
/// All state is protected by a single `tokio::sync::Mutex`.  For production
/// use with high concurrency you may want to shard the map; the API surface
/// here is unchanged.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use indexmap::IndexMap;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::cache::invalidation::CacheMetrics;

// ---------------------------------------------------------------------------
// Cache entry
// ---------------------------------------------------------------------------

struct Entry {
    /// Serialised value (JSON bytes).
    data: Vec<u8>,
    /// When the entry expires.
    expires_at: Instant,
    /// Last access time, used for LRU ordering.
    last_used: Instant,
}

impl Entry {
    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

// ---------------------------------------------------------------------------
// CacheManager
// ---------------------------------------------------------------------------

/// Thread-safe in-process cache with TTL + LRU eviction.
pub struct CacheManager {
    /// `IndexMap` preserves insertion order so we can implement LRU cheaply by
    /// iterating from the front.
    store: Mutex<IndexMap<String, Entry>>,
    /// Hard upper bound on the number of entries.
    capacity: usize,
    /// Default TTL applied when the caller does not supply one.
    default_ttl: Duration,
    /// Shared metrics (optional – pass `Arc::default()` if not needed).
    metrics: Arc<CacheMetrics>,
}

impl CacheManager {
    /// Create a new `CacheManager`.
    ///
    /// * `capacity` – maximum number of entries before LRU eviction kicks in.
    /// * `default_ttl` – TTL used by `set`.
    pub fn new(capacity: usize, default_ttl: Duration, metrics: Arc<CacheMetrics>) -> Self {
        Self {
            store: Mutex::new(IndexMap::with_capacity(capacity)),
            capacity,
            default_ttl,
            metrics,
        }
    }

    // -----------------------------------------------------------------------
    // Core get / set
    // -----------------------------------------------------------------------

    /// Insert `value` under `key` with the default TTL.
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()> {
        let ttl = ttl.unwrap_or(self.default_ttl);
        let data = serde_json::to_vec(value)?;
        let now = Instant::now();
        let entry = Entry {
            data,
            expires_at: now + ttl,
            last_used: now,
        };

        let mut store = self.store.lock().await;

        // Evict one LRU entry if we are at capacity and the key is not already
        // present (existing keys are updates, not new entries).
        if store.len() >= self.capacity && !store.contains_key(key) {
            if let Some(lru_key) = store.keys().next().map(|k| k.clone()) {
                store.shift_remove(&lru_key);
                self.metrics.record_eviction();
                debug!("Capacity eviction of LRU key '{}'", lru_key);
            }
        }

        store.insert(key.to_owned(), entry);
        Ok(())
    }

    /// Retrieve and deserialise the value stored under `key`.
    ///
    /// Returns `None` on a miss or if the entry has expired (lazy expiry).
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut store = self.store.lock().await;
        match store.get_mut(key) {
            None => {
                self.metrics.record_miss();
                None
            }
            Some(entry) if entry.is_expired() => {
                store.shift_remove(key);
                self.metrics.record_miss();
                None
            }
            Some(entry) => {
                entry.last_used = Instant::now();
                match serde_json::from_slice(&entry.data) {
                    Ok(v) => {
                        self.metrics.record_hit();
                        Some(v)
                    }
                    Err(e) => {
                        tracing::error!("Deserialisation error for key '{}': {}", key, e);
                        self.metrics.record_miss();
                        None
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Invalidation
    // -----------------------------------------------------------------------

    /// Remove a single key.  Returns `true` if the key was present.
    pub async fn invalidate(&self, key: &str) -> Result<bool> {
        let mut store = self.store.lock().await;
        Ok(store.shift_remove(key).is_some())
    }

    /// Remove all entries for which `predicate(key)` returns `true`.
    /// Returns the number of entries removed.
    pub async fn invalidate_matching<F>(&self, predicate: F) -> Result<usize>
    where
        F: Fn(&str) -> bool,
    {
        let mut store = self.store.lock().await;
        let keys_to_remove: Vec<String> = store
            .keys()
            .filter(|k| predicate(k.as_str()))
            .cloned()
            .collect();
        let n = keys_to_remove.len();
        for k in keys_to_remove {
            store.shift_remove(&k);
        }
        Ok(n)
    }

    // -----------------------------------------------------------------------
    // Eviction
    // -----------------------------------------------------------------------

    /// Evict up to `count` least-recently-used entries.
    /// Returns the number of entries actually evicted.
    pub async fn evict_lru(&self, count: usize) -> Result<usize> {
        let mut store = self.store.lock().await;
        // Sort by `last_used` ascending → front = oldest.
        let mut pairs: Vec<(String, Instant)> = store
            .iter()
            .map(|(k, e)| (k.clone(), e.last_used))
            .collect();
        pairs.sort_by_key(|(_, t)| *t);

        let to_remove: Vec<String> = pairs.into_iter().take(count).map(|(k, _)| k).collect();
        let n = to_remove.len();
        for k in to_remove {
            store.shift_remove(&k);
        }
        Ok(n)
    }

    /// Remove all entries inserted / last refreshed more than `max_age` ago.
    /// Returns the number of entries removed.
    pub async fn evict_older_than(&self, max_age: Duration) -> Result<usize> {
        let cutoff = Instant::now() - max_age;
        let mut store = self.store.lock().await;
        let stale: Vec<String> = store
            .iter()
            .filter(|(_, e)| e.last_used < cutoff || e.is_expired())
            .map(|(k, _)| k.clone())
            .collect();
        let n = stale.len();
        for k in stale {
            store.shift_remove(&k);
        }
        Ok(n)
    }

    // -----------------------------------------------------------------------
    // Introspection
    // -----------------------------------------------------------------------

    /// Number of live (possibly stale) entries.
    pub async fn len(&self) -> usize {
        self.store.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Keys currently in the cache (may include expired entries not yet evicted).
    pub async fn keys(&self) -> Vec<String> {
        self.store.lock().await.keys().cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// Cache warming
// ---------------------------------------------------------------------------

/// Trait that data-source adapters implement so `warm_cache` stays generic.
#[async_trait::async_trait]
pub trait CacheWarmer {
    /// Identifier used in log messages.
    fn name(&self) -> &str;

    /// Produce the set of (key, serialised-JSON-bytes, ttl) triples to load.
    async fn entries(&self) -> Result<Vec<(String, serde_json::Value, Duration)>>;
}

/// Preload the cache with data from one or more warmers.
///
/// Called once on application startup; errors in individual warmers are
/// logged but do not abort the warm-up of other warmers.
pub async fn warm_cache(cache: &CacheManager, warmers: &[Box<dyn CacheWarmer + Send + Sync>]) {
    for warmer in warmers {
        info!("Cache warming: starting '{}'", warmer.name());
        match warmer.entries().await {
            Err(e) => {
                tracing::error!("Cache warmer '{}' failed: {}", warmer.name(), e);
            }
            Ok(entries) => {
                let mut loaded = 0usize;
                for (key, value, ttl) in entries {
                    if let Err(e) = cache.set(&key, &value, Some(ttl)).await {
                        tracing::error!("Warm load failed for key '{}': {}", key, e);
                    } else {
                        cache.metrics.record_warm_load();
                        loaded += 1;
                    }
                }
                info!(
                    "Cache warming: '{}' loaded {} entries",
                    warmer.name(),
                    loaded
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in warmers (examples / stubs the application layer fills in)
// ---------------------------------------------------------------------------

/// Warms the top-N corridor entries from the database and Stellar RPC.
pub struct TopCorridorsWarmer {
    pub top_n: usize,
    pub ttl: Duration,
    /// Closure / async fn that returns (corridor_id, serialised data).
    pub fetch: Arc<dyn Fn(usize) -> futures::future::BoxFuture<'static, Result<Vec<(String, serde_json::Value)>>> + Send + Sync>,
}

#[async_trait::async_trait]
impl CacheWarmer for TopCorridorsWarmer {
    fn name(&self) -> &str {
        "top-corridors"
    }

    async fn entries(&self) -> Result<Vec<(String, serde_json::Value, Duration)>> {
        let raw = (self.fetch)(self.top_n).await?;
        Ok(raw
            .into_iter()
            .map(|(id, data)| (format!("corridor:{}", id), data, self.ttl))
            .collect())
    }
}

/// Warms all active anchors.
pub struct ActiveAnchorsWarmer {
    pub ttl: Duration,
    pub fetch: Arc<dyn Fn() -> futures::future::BoxFuture<'static, Result<Vec<(String, serde_json::Value)>>> + Send + Sync>,
}

#[async_trait::async_trait]
impl CacheWarmer for ActiveAnchorsWarmer {
    fn name(&self) -> &str {
        "active-anchors"
    }

    async fn entries(&self) -> Result<Vec<(String, serde_json::Value, Duration)>> {
        let raw = (self.fetch)().await?;
        Ok(raw
            .into_iter()
            .map(|(id, data)| (format!("anchor:{}", id), data, self.ttl))
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    fn make_cache(capacity: usize) -> CacheManager {
        CacheManager::new(
            capacity,
            Duration::from_secs(60),
            Arc::new(CacheMetrics::default()),
        )
    }

    #[tokio::test]
    async fn set_and_get() {
        let c = make_cache(10);
        c.set("k1", &json!({"v": 1}), None).await.unwrap();
        let v: serde_json::Value = c.get("k1").await.unwrap();
        assert_eq!(v["v"], 1);
    }

    #[tokio::test]
    async fn expired_entry_is_miss() {
        let c = make_cache(10);
        c.set("k1", &json!(1), Some(Duration::from_millis(1))).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        let v: Option<serde_json::Value> = c.get("k1").await;
        assert!(v.is_none());
    }

    #[tokio::test]
    async fn invalidate_removes_key() {
        let c = make_cache(10);
        c.set("k1", &json!(1), None).await.unwrap();
        assert!(c.invalidate("k1").await.unwrap());
        assert!(c.get::<serde_json::Value>("k1").await.is_none());
        assert!(!c.invalidate("k1").await.unwrap());
    }

    #[tokio::test]
    async fn invalidate_matching_pattern() {
        let c = make_cache(10);
        c.set("corridor:1", &json!(1), None).await.unwrap();
        c.set("corridor:2", &json!(2), None).await.unwrap();
        c.set("anchor:1", &json!(3), None).await.unwrap();
        let n = c
            .invalidate_matching(|k| k.starts_with("corridor:"))
            .await
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(c.len().await, 1);
    }

    #[tokio::test]
    async fn lru_capacity_eviction() {
        let c = make_cache(3);
        for i in 0..4u32 {
            c.set(&format!("k{}", i), &json!(i), None).await.unwrap();
        }
        // After inserting 4 items into a capacity-3 cache, oldest should be gone.
        assert_eq!(c.len().await, 3);
    }

    #[tokio::test]
    async fn evict_lru_explicit() {
        let c = make_cache(10);
        for i in 0..5u32 {
            c.set(&format!("k{}", i), &json!(i), None).await.unwrap();
        }
        let evicted = c.evict_lru(2).await.unwrap();
        assert_eq!(evicted, 2);
        assert_eq!(c.len().await, 3);
    }

    #[tokio::test]
    async fn evict_older_than() {
        let c = make_cache(10);
        c.set("old", &json!(0), Some(Duration::from_secs(300))).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        c.set("new", &json!(1), Some(Duration::from_secs(300))).await.unwrap();
        let removed = c.evict_older_than(Duration::from_millis(30)).await.unwrap();
        assert_eq!(removed, 1);
        assert!(c.get::<serde_json::Value>("new").await.is_some());
    }
}
