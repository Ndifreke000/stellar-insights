use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Cache performance metrics
#[derive(Debug, Clone)]
pub struct CacheMetrics {
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    invalidations: Arc<AtomicU64>,
}

impl CacheMetrics {
    pub fn new() -> Self {
        Self {
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            invalidations: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_invalidation(&self) {
        self.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    pub fn get_misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    pub fn get_errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    pub fn get_invalidations(&self) -> u64 {
        self.invalidations.load(Ordering::Relaxed)
    }

    pub fn hit_rate(&self) -> f64 {
        let hits = self.get_hits();
        let misses = self.get_misses();
        let total = hits + misses;
        
        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }

    pub fn summary(&self) -> CacheMetricsSummary {
        CacheMetricsSummary {
            hits: self.get_hits(),
            misses: self.get_misses(),
            errors: self.get_errors(),
            invalidations: self.get_invalidations(),
            hit_rate: self.hit_rate(),
        }
    }

    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.invalidations.store(0, Ordering::Relaxed);
    }
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheMetricsSummary {
    pub hits: u64,
    pub misses: u64,
    pub errors: u64,
    pub invalidations: u64,
    pub hit_rate: f64,
}
