use serde::{Deserialize, Serialize};

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub key: String,
    pub value: T,
    pub ttl_secs: u64,
    pub created_at: i64,
}

/// Cache invalidation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInvalidation {
    pub key: String,
    pub reason: String,
    pub timestamp: i64,
}

/// Cache performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub memory_used_bytes: u64,
}

impl CacheMetrics {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}
