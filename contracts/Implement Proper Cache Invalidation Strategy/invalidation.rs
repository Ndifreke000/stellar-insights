/// cache/invalidation.rs
/// 
/// Event-driven cache invalidation system for the corridor/anchor cache layer.
/// Provides:
///   - Typed invalidation events published via a tokio broadcast channel
///   - Pattern-based key matching for selective invalidation
///   - An `InvalidationWorker` that subscribes to events and drives cache cleanup
///   - Admin-facing `InvalidationController` for manual triggers
///   - Aggregated `CacheMetrics` with Prometheus-style counters

use std::sync::Arc;
use std::time::{Duration, Instant};

use regex::Regex;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::cache::CacheManager;

// ---------------------------------------------------------------------------
// Invalidation events
// ---------------------------------------------------------------------------

/// All reasons a cache entry (or group of entries) can be invalidated.
#[derive(Debug, Clone)]
pub enum InvalidationEvent {
    /// A new payment was detected; the corridor key is provided.
    PaymentDetected { corridor_key: String },

    /// An anchor's status changed; the anchor key is provided.
    AnchorStatusChanged { anchor_key: String },

    /// An administrator explicitly requested invalidation of a key pattern.
    AdminTrigger { pattern: String },

    /// A time-based sweep should remove entries older than `max_age`.
    TimeBased { max_age: Duration },

    /// The cache is under memory pressure; evict the least-recently-used `count` entries.
    MemoryPressure { count: usize },
}

// ---------------------------------------------------------------------------
// Cache metrics
// ---------------------------------------------------------------------------

/// Thread-safe counters for cache observability.
#[derive(Debug, Default)]
pub struct CacheMetrics {
    pub hits: std::sync::atomic::AtomicU64,
    pub misses: std::sync::atomic::AtomicU64,
    pub invalidations: std::sync::atomic::AtomicU64,
    pub warm_loads: std::sync::atomic::AtomicU64,
    pub evictions: std::sync::atomic::AtomicU64,
}

impl CacheMetrics {
    pub fn record_hit(&self) {
        self.hits
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.misses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_invalidation(&self) {
        self.invalidations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_warm_load(&self) {
        self.warm_loads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_eviction(&self) {
        self.evictions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Return a snapshot as a plain struct for serialisation.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            hits: self.hits.load(std::sync::atomic::Ordering::Relaxed),
            misses: self.misses.load(std::sync::atomic::Ordering::Relaxed),
            invalidations: self
                .invalidations
                .load(std::sync::atomic::Ordering::Relaxed),
            warm_loads: self.warm_loads.load(std::sync::atomic::Ordering::Relaxed),
            evictions: self.evictions.load(std::sync::atomic::Ordering::Relaxed),
            hit_rate: {
                let h = self.hits.load(std::sync::atomic::Ordering::Relaxed) as f64;
                let m = self.misses.load(std::sync::atomic::Ordering::Relaxed) as f64;
                if h + m == 0.0 { 0.0 } else { h / (h + m) }
            },
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct MetricsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub invalidations: u64,
    pub warm_loads: u64,
    pub evictions: u64,
    pub hit_rate: f64,
}

// ---------------------------------------------------------------------------
// Pattern matcher
// ---------------------------------------------------------------------------

/// Decides whether a cache key matches an invalidation pattern.
///
/// Patterns may use glob-style wildcards (`*` matches any sequence) or be
/// treated as regex when they start/end with `/`.
pub struct KeyPattern {
    inner: PatternKind,
}

enum PatternKind {
    Exact(String),
    Glob(String),
    Regex(Regex),
}

impl KeyPattern {
    /// Parse a pattern string.
    ///
    /// * `/regex/` → compiled as a regular expression  
    /// * `*` anywhere → treated as a glob  
    /// * otherwise → exact match
    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        if raw.starts_with('/') && raw.ends_with('/') && raw.len() > 2 {
            let inner_pattern = &raw[1..raw.len() - 1];
            let re = Regex::new(inner_pattern)?;
            return Ok(Self {
                inner: PatternKind::Regex(re),
            });
        }
        if raw.contains('*') {
            return Ok(Self {
                inner: PatternKind::Glob(raw.to_owned()),
            });
        }
        Ok(Self {
            inner: PatternKind::Exact(raw.to_owned()),
        })
    }

    /// Returns `true` when `key` satisfies this pattern.
    pub fn matches(&self, key: &str) -> bool {
        match &self.inner {
            PatternKind::Exact(p) => key == p.as_str(),
            PatternKind::Glob(g) => glob_match(g, key),
            PatternKind::Regex(re) => re.is_match(key),
        }
    }
}

/// Minimal glob matcher supporting `*` wildcards.
fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.is_empty() {
        return true;
    }
    let mut remaining = text;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            if !remaining.starts_with(*part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        } else if i == parts.len() - 1 {
            return remaining.ends_with(*part);
        } else {
            match remaining.find(*part) {
                Some(pos) => remaining = &remaining[pos + part.len()..],
                None => return false,
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Invalidation worker
// ---------------------------------------------------------------------------

/// Background task that listens for `InvalidationEvent`s and acts on the cache.
pub struct InvalidationWorker {
    cache: Arc<CacheManager>,
    metrics: Arc<CacheMetrics>,
    rx: broadcast::Receiver<InvalidationEvent>,
}

impl InvalidationWorker {
    pub fn new(
        cache: Arc<CacheManager>,
        metrics: Arc<CacheMetrics>,
        rx: broadcast::Receiver<InvalidationEvent>,
    ) -> Self {
        Self { cache, metrics, rx }
    }

    /// Spawn the worker on the current tokio runtime and return the join handle.
    pub fn spawn(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("InvalidationWorker started");
            loop {
                match self.rx.recv().await {
                    Ok(event) => self.handle(event).await,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("InvalidationWorker lagged, missed {} events", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("InvalidationWorker channel closed – shutting down");
                        break;
                    }
                }
            }
        })
    }

    async fn handle(&self, event: InvalidationEvent) {
        match event {
            InvalidationEvent::PaymentDetected { corridor_key } => {
                info!("Payment detected → invalidating corridor '{}'", corridor_key);
                self.invalidate_key(&corridor_key).await;
            }

            InvalidationEvent::AnchorStatusChanged { anchor_key } => {
                info!("Anchor changed → invalidating anchor '{}'", anchor_key);
                self.invalidate_key(&anchor_key).await;
            }

            InvalidationEvent::AdminTrigger { pattern } => {
                info!("Admin trigger → invalidating pattern '{}'", pattern);
                self.invalidate_pattern(&pattern).await;
            }

            InvalidationEvent::TimeBased { max_age } => {
                info!("Time-based sweep → max_age={:?}", max_age);
                match self.cache.evict_older_than(max_age).await {
                    Ok(n) => {
                        for _ in 0..n {
                            self.metrics.record_eviction();
                            self.metrics.record_invalidation();
                        }
                        info!("Time-based sweep removed {} entries", n);
                    }
                    Err(e) => error!("Time-based eviction failed: {}", e),
                }
            }

            InvalidationEvent::MemoryPressure { count } => {
                info!("Memory pressure → evicting {} LRU entries", count);
                match self.cache.evict_lru(count).await {
                    Ok(n) => {
                        for _ in 0..n {
                            self.metrics.record_eviction();
                        }
                        info!("LRU eviction removed {} entries", n);
                    }
                    Err(e) => error!("LRU eviction failed: {}", e),
                }
            }
        }
    }

    async fn invalidate_key(&self, key: &str) {
        match self.cache.invalidate(key).await {
            Ok(true) => {
                self.metrics.record_invalidation();
                info!("Invalidated key '{}'", key);
            }
            Ok(false) => {}
            Err(e) => error!("Failed to invalidate '{}': {}", key, e),
        }
    }

    async fn invalidate_pattern(&self, pattern: &str) {
        match KeyPattern::parse(pattern) {
            Ok(p) => match self.cache.invalidate_matching(|k| p.matches(k)).await {
                Ok(n) => {
                    for _ in 0..n {
                        self.metrics.record_invalidation();
                    }
                    info!("Pattern '{}' invalidated {} entries", pattern, n);
                }
                Err(e) => error!("Pattern invalidation failed: {}", e),
            },
            Err(e) => error!("Bad pattern '{}': {}", pattern, e),
        }
    }
}

// ---------------------------------------------------------------------------
// Admin controller
// ---------------------------------------------------------------------------

/// High-level handle that API handlers can use to fire invalidation events.
#[derive(Clone)]
pub struct InvalidationController {
    tx: broadcast::Sender<InvalidationEvent>,
    metrics: Arc<CacheMetrics>,
}

impl InvalidationController {
    pub fn new(tx: broadcast::Sender<InvalidationEvent>, metrics: Arc<CacheMetrics>) -> Self {
        Self { tx, metrics }
    }

    /// Notify that a payment was detected for `corridor_key`.
    pub fn payment_detected(&self, corridor_key: impl Into<String>) {
        let _ = self.tx.send(InvalidationEvent::PaymentDetected {
            corridor_key: corridor_key.into(),
        });
    }

    /// Notify that an anchor's status changed.
    pub fn anchor_status_changed(&self, anchor_key: impl Into<String>) {
        let _ = self.tx.send(InvalidationEvent::AnchorStatusChanged {
            anchor_key: anchor_key.into(),
        });
    }

    /// Trigger invalidation for all keys matching a pattern (admin endpoint).
    pub fn admin_invalidate(&self, pattern: impl Into<String>) {
        let _ = self.tx.send(InvalidationEvent::AdminTrigger {
            pattern: pattern.into(),
        });
    }

    /// Trigger a time-based sweep.
    pub fn time_sweep(&self, max_age: Duration) {
        let _ = self.tx.send(InvalidationEvent::TimeBased { max_age });
    }

    /// Signal that memory pressure requires LRU eviction.
    pub fn memory_pressure(&self, count: usize) {
        let _ = self.tx.send(InvalidationEvent::MemoryPressure { count });
    }

    /// Return a snapshot of current cache metrics.
    pub fn metrics(&self) -> MetricsSnapshot {
        self.metrics.snapshot()
    }
}

// ---------------------------------------------------------------------------
// Builder / factory
// ---------------------------------------------------------------------------

/// Construct the invalidation subsystem: channel + worker + controller.
///
/// Returns `(controller, worker)`.  Callers must call `worker.spawn()` to
/// start background processing.
pub fn build_invalidation_system(
    cache: Arc<CacheManager>,
    metrics: Arc<CacheMetrics>,
    channel_capacity: usize,
) -> (InvalidationController, InvalidationWorker) {
    let (tx, rx) = broadcast::channel(channel_capacity);
    let controller = InvalidationController::new(tx, Arc::clone(&metrics));
    let worker = InvalidationWorker::new(cache, metrics, rx);
    (controller, worker)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_exact() {
        assert!(glob_match("hello", "hello"));
        assert!(!glob_match("hello", "world"));
    }

    #[test]
    fn glob_wildcard_prefix() {
        assert!(glob_match("corridor:*", "corridor:usd-eur"));
        assert!(!glob_match("corridor:*", "anchor:foo"));
    }

    #[test]
    fn glob_wildcard_suffix() {
        assert!(glob_match("*:v2", "corridor:v2"));
        assert!(!glob_match("*:v2", "corridor:v3"));
    }

    #[test]
    fn glob_wildcard_middle() {
        assert!(glob_match("corridor:*:rates", "corridor:usd-eur:rates"));
        assert!(!glob_match("corridor:*:rates", "corridor:usd-eur:fees"));
    }

    #[test]
    fn key_pattern_exact() {
        let p = KeyPattern::parse("anchor:foo").unwrap();
        assert!(p.matches("anchor:foo"));
        assert!(!p.matches("anchor:bar"));
    }

    #[test]
    fn key_pattern_glob() {
        let p = KeyPattern::parse("anchor:*").unwrap();
        assert!(p.matches("anchor:foo"));
        assert!(p.matches("anchor:bar"));
        assert!(!p.matches("corridor:x"));
    }

    #[test]
    fn key_pattern_regex() {
        let p = KeyPattern::parse("/^corridor:\\d+$/").unwrap();
        assert!(p.matches("corridor:42"));
        assert!(!p.matches("corridor:usd"));
    }

    #[test]
    fn metrics_snapshot() {
        let m = CacheMetrics::default();
        m.record_hit();
        m.record_hit();
        m.record_miss();
        let s = m.snapshot();
        assert_eq!(s.hits, 2);
        assert_eq!(s.misses, 1);
        assert!((s.hit_rate - 2.0 / 3.0).abs() < 1e-9);
    }
}
