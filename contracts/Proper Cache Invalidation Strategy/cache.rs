use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;
use tracing::{info, warn};

pub mod invalidation;

/// Default TTL for cache entries (5 minutes)
pub const DEFAULT_TTL: Duration = Duration::from_secs(300);
/// Default capacity (number of entries) before LRU eviction kicks in
pub const DEFAULT_CAPACITY: usize = 1_000;

// ────────────────────────────────────────────────────────────────
// Cache invalidation events
// ────────────────────────────────────────────────────────────────

/// Events that trigger cache invalidation.
#[derive(Debug, Clone)]
pub enum CacheInvalidationEvent {
    /// A new payment was detected for a specific corridor id.
    PaymentDetected { corridor_id: String },
    /// An anchor's status changed.
    AnchorStatusChanged { anchor_id: String },
    /// An admin manually invalidates entries matching a key pattern.
    AdminInvalidate { pattern: String },
    /// Periodic TTL sweep (fired internally by the background task).
    TtlSweep,
    /// Memory pressure: evict LRU entries down to `target_size`.
    MemoryPressure { target_size: usize },
}

// ────────────────────────────────────────────────────────────────
// Cache entry
// ────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct CacheEntry<V: Clone> {
    value: V,
    expires_at: Instant,
    /// Monotonically increasing counter used for LRU ordering.
    last_used: u64,
}

impl<V: Clone> CacheEntry<V> {
    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

// ────────────────────────────────────────────────────────────────
// Cache metrics
// ────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub invalidations: u64,
    pub evictions: u64,
    pub warm_ups: u64,
    pub current_size: usize,
}

impl CacheMetrics {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

// ────────────────────────────────────────────────────────────────
// CacheManager
// ────────────────────────────────────────────────────────────────

/// Thread-safe cache manager with TTL, LRU eviction, pattern-based
/// invalidation, metrics, and event-driven invalidation.
pub struct CacheManager<V: Clone + Send + Sync + 'static> {
    store: Arc<RwLock<HashMap<String, CacheEntry<V>>>>,
    metrics: Arc<RwLock<CacheMetrics>>,
    capacity: usize,
    /// Logical clock for LRU ordering (incremented on every access).
    clock: Arc<std::sync::atomic::AtomicU64>,
    /// Sender for invalidation events.
    event_tx: broadcast::Sender<CacheInvalidationEvent>,
}

impl<V: Clone + Send + Sync + 'static> CacheManager<V> {
    /// Create a new `CacheManager` and spawn the background invalidation task.
    pub fn new(capacity: usize) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let manager = Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
            capacity,
            clock: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            event_tx,
        };
        manager.spawn_background_task();
        manager
    }

    // ── Public API ──────────────────────────────────────────────

    /// Insert or update a key with a specific TTL.
    pub async fn set(&self, key: impl Into<String>, value: V, ttl: Duration) {
        let key = key.into();
        let seq = self
            .clock
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + ttl,
            last_used: seq,
        };
        let mut store = self.store.write().await;
        store.insert(key, entry);
        let size = store.len();
        drop(store);

        let mut m = self.metrics.write().await;
        m.current_size = size;

        if size > self.capacity {
            self.evict_lru().await;
        }
    }

    /// Retrieve a value by key; returns `None` if absent or expired.
    pub async fn get(&self, key: &str) -> Option<V> {
        let mut store = self.store.write().await;
        if let Some(entry) = store.get_mut(key) {
            if entry.is_expired() {
                store.remove(key);
                drop(store);
                let mut m = self.metrics.write().await;
                m.misses += 1;
                return None;
            }
            let seq = self
                .clock
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            entry.last_used = seq;
            let value = entry.value.clone();
            drop(store);
            let mut m = self.metrics.write().await;
            m.hits += 1;
            Some(value)
        } else {
            drop(store);
            let mut m = self.metrics.write().await;
            m.misses += 1;
            None
        }
    }

    /// Remove a single key.
    pub async fn invalidate(&self, key: &str) {
        let mut store = self.store.write().await;
        store.remove(key);
        let size = store.len();
        drop(store);
        let mut m = self.metrics.write().await;
        m.invalidations += 1;
        m.current_size = size;
    }

    /// Remove all keys whose names contain `pattern` as a substring.
    pub async fn invalidate_pattern(&self, pattern: &str) {
        let mut store = self.store.write().await;
        let before = store.len();
        store.retain(|k, _| !k.contains(pattern));
        let removed = before - store.len();
        let size = store.len();
        drop(store);
        if removed > 0 {
            info!("Cache: invalidated {} entries matching pattern '{}'", removed, pattern);
        }
        let mut m = self.metrics.write().await;
        m.invalidations += removed as u64;
        m.current_size = size;
    }

    /// Flush the entire cache.
    pub async fn flush(&self) {
        let mut store = self.store.write().await;
        let n = store.len();
        store.clear();
        drop(store);
        let mut m = self.metrics.write().await;
        m.invalidations += n as u64;
        m.current_size = 0;
        info!("Cache: flushed {} entries", n);
    }

    /// Get a snapshot of current metrics.
    pub async fn metrics(&self) -> CacheMetrics {
        self.metrics.read().await.clone()
    }

    /// Publish an invalidation event to all subscribers (including the
    /// internal background task).
    pub fn publish_event(&self, event: CacheInvalidationEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Subscribe to invalidation events (useful for composed managers).
    pub fn subscribe(&self) -> broadcast::Receiver<CacheInvalidationEvent> {
        self.event_tx.subscribe()
    }

    // ── Internal helpers ─────────────────────────────────────────

    async fn evict_lru(&self) {
        let mut store = self.store.write().await;
        if store.len() <= self.capacity {
            return;
        }
        // Find the entry with the smallest `last_used` value.
        if let Some(lru_key) = store
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(k, _)| k.clone())
        {
            store.remove(&lru_key);
            info!("Cache: LRU evicted key '{}'", lru_key);
        }
        let size = store.len();
        drop(store);
        let mut m = self.metrics.write().await;
        m.evictions += 1;
        m.current_size = size;
    }

    async fn sweep_expired(&self) {
        let mut store = self.store.write().await;
        let before = store.len();
        store.retain(|_, e| !e.is_expired());
        let removed = before - store.len();
        let size = store.len();
        drop(store);
        if removed > 0 {
            info!("Cache: TTL sweep removed {} expired entries", removed);
            let mut m = self.metrics.write().await;
            m.invalidations += removed as u64;
            m.current_size = size;
        }
    }

    fn spawn_background_task(&self) {
        let store = Arc::clone(&self.store);
        let metrics = Arc::clone(&self.metrics);
        let event_tx = self.event_tx.clone();
        let capacity = self.capacity;

        tokio::spawn(async move {
            let mut rx = event_tx.subscribe();
            // Periodic TTL sweep every 60 s.
            let mut sweep_ticker = interval(Duration::from_secs(60));

            loop {
                tokio::select! {
                    _ = sweep_ticker.tick() => {
                        // TTL sweep
                        let mut s = store.write().await;
                        let before = s.len();
                        s.retain(|_, e| !e.is_expired());
                        let removed = before - s.len();
                        let size = s.len();
                        drop(s);
                        if removed > 0 {
                            info!("Cache bg: TTL sweep removed {} entries", removed);
                            let mut m = metrics.write().await;
                            m.invalidations += removed as u64;
                            m.current_size = size;
                        }
                    }
                    Ok(event) = rx.recv() => {
                        match event {
                            CacheInvalidationEvent::PaymentDetected { corridor_id } => {
                                let pattern = format!("corridor:{}", corridor_id);
                                let mut s = store.write().await;
                                let before = s.len();
                                s.retain(|k, _| !k.contains(&pattern));
                                let removed = before - s.len();
                                let size = s.len();
                                drop(s);
                                info!("Cache bg: payment event invalidated {} corridor entries for '{}'", removed, corridor_id);
                                let mut m = metrics.write().await;
                                m.invalidations += removed as u64;
                                m.current_size = size;
                            }
                            CacheInvalidationEvent::AnchorStatusChanged { anchor_id } => {
                                let pattern = format!("anchor:{}", anchor_id);
                                let mut s = store.write().await;
                                let before = s.len();
                                s.retain(|k, _| !k.contains(&pattern));
                                let removed = before - s.len();
                                let size = s.len();
                                drop(s);
                                info!("Cache bg: anchor status change invalidated {} entries for '{}'", removed, anchor_id);
                                let mut m = metrics.write().await;
                                m.invalidations += removed as u64;
                                m.current_size = size;
                            }
                            CacheInvalidationEvent::AdminInvalidate { pattern } => {
                                let mut s = store.write().await;
                                let before = s.len();
                                s.retain(|k, _| !k.contains(&pattern));
                                let removed = before - s.len();
                                let size = s.len();
                                drop(s);
                                info!("Cache bg: admin invalidated {} entries matching '{}'", removed, pattern);
                                let mut m = metrics.write().await;
                                m.invalidations += removed as u64;
                                m.current_size = size;
                            }
                            CacheInvalidationEvent::TtlSweep => {
                                let mut s = store.write().await;
                                s.retain(|_, e| !e.is_expired());
                                let size = s.len();
                                drop(s);
                                let mut m = metrics.write().await;
                                m.current_size = size;
                            }
                            CacheInvalidationEvent::MemoryPressure { target_size } => {
                                let mut s = store.write().await;
                                while s.len() > target_size {
                                    if let Some(lru_key) = s
                                        .iter()
                                        .min_by_key(|(_, e)| e.last_used)
                                        .map(|(k, _)| k.clone())
                                    {
                                        s.remove(&lru_key);
                                        warn!("Cache bg: memory pressure evicted '{}'", lru_key);
                                    } else {
                                        break;
                                    }
                                }
                                let size = s.len();
                                drop(s);
                                let mut m = metrics.write().await;
                                m.evictions += (capacity - target_size) as u64;
                                m.current_size = size;
                            }
                        }
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_set_and_get() {
        let cache: CacheManager<String> = CacheManager::new(100);
        cache.set("key1", "value1".to_string(), DEFAULT_TTL).await;
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache: CacheManager<String> = CacheManager::new(100);
        cache.set("key1", "value1".to_string(), Duration::from_millis(50)).await;
        sleep(Duration::from_millis(100)).await;
        assert_eq!(cache.get("key1").await, None);
    }

    #[tokio::test]
    async fn test_invalidate() {
        let cache: CacheManager<String> = CacheManager::new(100);
        cache.set("key1", "value1".to_string(), DEFAULT_TTL).await;
        cache.invalidate("key1").await;
        assert_eq!(cache.get("key1").await, None);
    }

    #[tokio::test]
    async fn test_invalidate_pattern() {
        let cache: CacheManager<String> = CacheManager::new(100);
        cache.set("corridor:abc:rates", "v1".to_string(), DEFAULT_TTL).await;
        cache.set("corridor:abc:fees", "v2".to_string(), DEFAULT_TTL).await;
        cache.set("anchor:xyz", "v3".to_string(), DEFAULT_TTL).await;
        cache.invalidate_pattern("corridor:abc").await;
        assert_eq!(cache.get("corridor:abc:rates").await, None);
        assert_eq!(cache.get("corridor:abc:fees").await, None);
        assert_eq!(cache.get("anchor:xyz").await, Some("v3".to_string()));
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let cache: CacheManager<String> = CacheManager::new(2);
        cache.set("k1", "v1".to_string(), DEFAULT_TTL).await;
        cache.set("k2", "v2".to_string(), DEFAULT_TTL).await;
        // Access k1 to make k2 the LRU
        cache.get("k1").await;
        // Adding k3 should evict k2 (LRU)
        cache.set("k3", "v3".to_string(), DEFAULT_TTL).await;
        assert!(cache.get("k1").await.is_some());
        assert!(cache.get("k3").await.is_some());
    }

    #[tokio::test]
    async fn test_metrics() {
        let cache: CacheManager<String> = CacheManager::new(100);
        cache.set("key1", "value1".to_string(), DEFAULT_TTL).await;
        cache.get("key1").await; // hit
        cache.get("missing").await; // miss
        let m = cache.metrics().await;
        assert_eq!(m.hits, 1);
        assert_eq!(m.misses, 1);
        assert!((m.hit_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_event_driven_invalidation() {
        let cache: CacheManager<String> = CacheManager::new(100);
        cache
            .set("corridor:abc:data", "v".to_string(), DEFAULT_TTL)
            .await;
        cache.publish_event(CacheInvalidationEvent::PaymentDetected {
            corridor_id: "abc".to_string(),
        });
        // Give the background task a moment to process
        sleep(Duration::from_millis(50)).await;
        assert_eq!(cache.get("corridor:abc:data").await, None);
    }
}
