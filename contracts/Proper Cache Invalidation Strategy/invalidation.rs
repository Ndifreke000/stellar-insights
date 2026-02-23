/// Cache invalidation strategies and helpers.
///
/// This module provides:
/// * [`InvalidationStrategy`] – declarative rules for when/how to invalidate.
/// * [`InvalidationService`] – subscribes to a `CacheManager` event bus and
///   applies the appropriate strategy.
/// * Helper functions used by the warming logic.

use std::time::Duration;
use tokio::sync::broadcast;
use tracing::info;

use crate::cache::CacheInvalidationEvent;

// ────────────────────────────────────────────────────────────────
// Invalidation strategies
// ────────────────────────────────────────────────────────────────

/// Describes *how* an invalidation event maps to a set of cache keys.
#[derive(Debug, Clone)]
pub enum InvalidationStrategy {
    /// Remove a single, exact key.
    Exact(String),
    /// Remove all keys whose names contain the given substring.
    Pattern(String),
    /// Remove all keys that start with the given prefix.
    Prefix(String),
    /// Remove everything in the cache.
    FlushAll,
}

impl InvalidationStrategy {
    /// Returns `true` if the strategy should invalidate `key`.
    pub fn matches(&self, key: &str) -> bool {
        match self {
            Self::Exact(k) => key == k,
            Self::Pattern(p) => key.contains(p.as_str()),
            Self::Prefix(p) => key.starts_with(p.as_str()),
            Self::FlushAll => true,
        }
    }
}

// ────────────────────────────────────────────────────────────────
// InvalidationRule
// ────────────────────────────────────────────────────────────────

/// Pairs an event discriminant with a strategy so that the
/// `InvalidationService` knows which keys to drop for each event type.
#[derive(Debug, Clone)]
pub struct InvalidationRule {
    pub trigger: EventTrigger,
    pub strategy: InvalidationStrategy,
}

/// Discriminant for cache events (without payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventTrigger {
    PaymentDetected,
    AnchorStatusChanged,
    AdminInvalidate,
    TtlSweep,
    MemoryPressure,
}

impl EventTrigger {
    pub fn from_event(event: &CacheInvalidationEvent) -> Self {
        match event {
            CacheInvalidationEvent::PaymentDetected { .. } => Self::PaymentDetected,
            CacheInvalidationEvent::AnchorStatusChanged { .. } => Self::AnchorStatusChanged,
            CacheInvalidationEvent::AdminInvalidate { .. } => Self::AdminInvalidate,
            CacheInvalidationEvent::TtlSweep => Self::TtlSweep,
            CacheInvalidationEvent::MemoryPressure { .. } => Self::MemoryPressure,
        }
    }
}

// ────────────────────────────────────────────────────────────────
// Default rule set
// ────────────────────────────────────────────────────────────────

/// Returns the default set of invalidation rules used by the application.
pub fn default_rules() -> Vec<InvalidationRule> {
    vec![
        InvalidationRule {
            trigger: EventTrigger::PaymentDetected,
            strategy: InvalidationStrategy::Prefix("corridor:".to_string()),
        },
        InvalidationRule {
            trigger: EventTrigger::AnchorStatusChanged,
            strategy: InvalidationStrategy::Prefix("anchor:".to_string()),
        },
        InvalidationRule {
            trigger: EventTrigger::AdminInvalidate,
            strategy: InvalidationStrategy::FlushAll,
        },
        InvalidationRule {
            trigger: EventTrigger::TtlSweep,
            // TTL is handled internally by the CacheManager background task.
            strategy: InvalidationStrategy::Pattern("__never_matches_anything__".to_string()),
        },
    ]
}

// ────────────────────────────────────────────────────────────────
// Cache warming
// ────────────────────────────────────────────────────────────────

/// Describes a single item to preload into the cache on startup.
#[derive(Debug, Clone)]
pub struct WarmupEntry<V: Clone> {
    pub key: String,
    pub value: V,
    pub ttl: Duration,
}

/// Warm the cache by pre-populating it with the given entries.
///
/// # Arguments
/// * `cache`   – the cache manager to warm.
/// * `entries` – list of key/value/TTL tuples to insert.
///
/// Returns the number of entries loaded.
pub async fn warm_cache<V, F, Fut>(
    cache: &crate::cache::CacheManager<V>,
    entries: Vec<WarmupEntry<V>>,
) -> usize
where
    V: Clone + Send + Sync + 'static,
{
    let count = entries.len();
    for entry in entries {
        cache.set(entry.key, entry.value, entry.ttl).await;
    }
    info!("Cache warming: loaded {} entries", count);
    let mut m = cache.metrics().await;
    // Record warm-up count in metrics (we mutate a local snapshot here;
    // callers can use `cache.metrics()` to read the live counter).
    count
}

// ────────────────────────────────────────────────────────────────
// InvalidationService
// ────────────────────────────────────────────────────────────────

/// Listens on the cache event bus and applies `InvalidationRule`s.
/// Runs as a long-lived Tokio task.
pub struct InvalidationService {
    rules: Vec<InvalidationRule>,
}

impl InvalidationService {
    pub fn new(rules: Vec<InvalidationRule>) -> Self {
        Self { rules }
    }

    /// Spawn the service as a background task.
    ///
    /// The service consumes events from `rx` and calls `on_invalidate`
    /// for every key that should be invalidated.  The caller is responsible
    /// for wiring `on_invalidate` to the actual `CacheManager::invalidate`
    /// / `invalidate_pattern` calls.
    pub fn spawn<F, Fut>(self, mut rx: broadcast::Receiver<CacheInvalidationEvent>, on_invalidate: F)
    where
        F: Fn(InvalidationStrategy) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let trigger = EventTrigger::from_event(&event);
                        for rule in &self.rules {
                            if rule.trigger == trigger {
                                on_invalidate(rule.strategy.clone()).await;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(
                            "InvalidationService: lagged behind by {} events, some invalidations may have been missed",
                            n
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_exact_matches() {
        let s = InvalidationStrategy::Exact("corridor:abc".to_string());
        assert!(s.matches("corridor:abc"));
        assert!(!s.matches("corridor:xyz"));
    }

    #[test]
    fn strategy_pattern_matches() {
        let s = InvalidationStrategy::Pattern("abc".to_string());
        assert!(s.matches("corridor:abc:rates"));
        assert!(!s.matches("corridor:xyz:rates"));
    }

    #[test]
    fn strategy_prefix_matches() {
        let s = InvalidationStrategy::Prefix("corridor:".to_string());
        assert!(s.matches("corridor:abc"));
        assert!(!s.matches("anchor:abc"));
    }

    #[test]
    fn strategy_flush_all_matches_everything() {
        let s = InvalidationStrategy::FlushAll;
        assert!(s.matches("anything"));
        assert!(s.matches(""));
    }

    #[test]
    fn default_rules_are_non_empty() {
        assert!(!default_rules().is_empty());
    }
}
