use redis::aio::MultiplexedConnection;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub invalidations: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

/// Cache configuration with TTL settings
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub corridor_metrics_ttl: usize, // 5 minutes
    pub anchor_data_ttl: usize,      // 10 minutes
    pub dashboard_stats_ttl: usize,  // 1 minute
}

impl CacheConfig {
    pub fn get_ttl(&self, cache_type: &str) -> usize {
        match cache_type {
            "corridor" => self.corridor_metrics_ttl,
            "anchor" => self.anchor_data_ttl,
            "dashboard" => self.dashboard_stats_ttl,
            _ => 300,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            corridor_metrics_ttl: 300, // 5 minutes
            anchor_data_ttl: 600,      // 10 minutes
            dashboard_stats_ttl: 60,   // 1 minute
        }
    }
}

/// Main cache manager
pub struct CacheManager {
    redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    pub config: CacheConfig,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    invalidations: Arc<AtomicU64>,
}

impl CacheManager {
    pub async fn new(config: CacheConfig) -> anyhow::Result<Self> {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let connection = if let Ok(client) = redis::Client::open(redis_url.as_str()) {
            match client.get_multiplexed_tokio_connection().await {
                Ok(conn) => {
                    tracing::info!("Connected to Redis for caching");
                    Some(conn)
                }
                Err(e) => {
                    tracing::warn!("Failed to connect to Redis for caching: {}", e);
                    None
                }
            }
        } else {
            tracing::warn!("Invalid Redis URL for caching");
            None
        };

        Ok(Self {
            redis_connection: Arc::new(RwLock::new(connection)),
            config,
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            invalidations: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Get value from cache, returns None if not found or Redis unavailable
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> anyhow::Result<Option<T>> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match redis::cmd("GET")
                .arg(key)
                .query_async::<_, Option<String>>(&mut conn)
                .await
            {
                Ok(Some(value)) => {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    crate::observability::metrics::record_cache_lookup(true);
                    tracing::debug!("Cache hit for key: {}", key);
                    match serde_json::from_str::<T>(&value) {
                        Ok(data) => Ok(Some(data)),
                        Err(e) => {
                            tracing::warn!("Failed to deserialize cached value for {}: {}", key, e);
                            Ok(None)
                        }
                    }
                }
                Ok(None) => {
                    self.misses.fetch_add(1, Ordering::Relaxed);
                    crate::observability::metrics::record_cache_lookup(false);
                    tracing::debug!("Cache miss for key: {}", key);
                    Ok(None)
                }
                Err(e) => {
                    tracing::warn!("Redis GET error for {}: {}", key, e);
                    self.misses.fetch_add(1, Ordering::Relaxed);
                    crate::observability::metrics::record_cache_lookup(false);
                    Ok(None)
                }
            }
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            crate::observability::metrics::record_cache_lookup(false);
            Ok(None)
        }
    }

    /// Set value in cache with TTL
    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_seconds: usize,
    ) -> anyhow::Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match serde_json::to_string(value) {
                Ok(serialized) => {
                    match redis::cmd("SETEX")
                        .arg(key)
                        .arg(ttl_seconds)
                        .arg(&serialized)
                        .query_async::<_, ()>(&mut conn)
                        .await
                    {
                        Ok(_) => {
                            tracing::debug!("Cache set for key: {} (TTL: {}s)", key, ttl_seconds);
                            Ok(())
                        }
                        Err(e) => {
                            tracing::warn!("Redis SETEX error for {}: {}", key, e);
                            Ok(())
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to serialize value for cache key {}: {}", key, e);
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Delete a cache key
    pub async fn delete(&self, key: &str) -> anyhow::Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match redis::cmd("DEL")
                .arg(key)
                .query_async::<_, ()>(&mut conn)
                .await
            {
                Ok(_) => {
                    self.invalidations.fetch_add(1, Ordering::Relaxed);
                    tracing::debug!("Cache invalidated for key: {}", key);
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!("Redis DEL error for {}: {}", key, e);
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Delete multiple cache keys matching a pattern
    /// Uses SCAN instead of KEYS to avoid blocking Redis
    pub async fn delete_pattern(&self, pattern: &str) -> anyhow::Result<usize> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let mut cursor: u64 = 0;
            let mut deleted_count: usize = 0;

            loop {
                let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(pattern)
                    .arg("COUNT")
                    .arg(100)
                    .query_async(&mut conn)
                    .await?;

                if !keys.is_empty() {
                    let mut pipe = redis::pipe();
                    pipe.atomic();

                    // non-blocking delete
                    for key in &keys {
                        pipe.cmd("UNLINK").arg(key);
                    }

                    pipe.query_async::<_, ()>(&mut conn).await?;

                    self.invalidations
                        .fetch_add(keys.len() as u64, Ordering::Relaxed);

                    deleted_count += keys.len();
                }

                cursor = new_cursor;

                if cursor == 0 {
                    break;
                }

                // cooperative async scheduling
                tokio::task::yield_now().await;
            }

            tracing::info!(
                "Deleted {} keys matching pattern: {}",
                deleted_count,
                pattern
            );

            Ok(deleted_count)
        } else {
            Ok(0)
        }
    }

    /// Invalidate cache keys matching a pattern (alias for delete_pattern)
    pub async fn invalidate_pattern(&self, pattern: &str) -> anyhow::Result<usize> {
        self.delete_pattern(pattern).await
    }

    /// Clean up expired entries (Redis handles this automatically, but useful for monitoring)
    pub async fn cleanup_expired(&self) -> anyhow::Result<()> {
        tracing::debug!("Cache cleanup triggered (Redis auto-expires keys)");
        Ok(())
    }

    /// Get current cache statistics
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            invalidations: self.invalidations.load(Ordering::Relaxed),
        }
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.invalidations.store(0, Ordering::Relaxed);
    }

    /// Close Redis connection gracefully
    pub async fn close(&self) -> anyhow::Result<()> {
        let mut conn_guard = self.redis_connection.write().await;
        if let Some(mut conn) = conn_guard.take() {
            // Ensure all pending operations are flushed
            match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                Ok(_) => tracing::debug!("Redis connection verified before close"),
                Err(e) => tracing::warn!("Redis PING failed before close: {}", e),
            }
            tracing::info!("Redis connection closed");
        }
        Ok(())
    }
}

/// Cache key builders for consistency
pub mod keys {
    pub fn anchor_list(limit: i64, offset: i64) -> String {
        format!("anchor:list:{}:{}", limit, offset)
    }

    pub fn anchor_detail(id: &str) -> String {
        format!("anchor:detail:{}", id)
    }

    pub fn anchor_by_account(account: &str) -> String {
        format!("anchor:account:{}", account)
    }

    pub fn anchor_assets(anchor_id: &str) -> String {
        format!("anchor:assets:{}", anchor_id)
    }

    pub fn corridor_list(limit: i64, offset: i64, filters: &str) -> String {
        format!("corridor:list:{}:{}:{}", limit, offset, filters)
    }

    pub fn corridor_detail(corridor_key: &str) -> String {
        format!("corridor:detail:{}", corridor_key)
    }

    pub fn dashboard_stats() -> String {
        "dashboard:stats".to_string()
    }

    pub fn metrics_overview() -> String {
        "metrics:overview".to_string()
    }

    /// Pattern for invalidating all anchor-related caches
    pub fn anchor_pattern() -> String {
        "anchor:*".to_string()
    }

    /// Pattern for invalidating all corridor-related caches
    pub fn corridor_pattern() -> String {
        "corridor:*".to_string()
    }

    /// Pattern for invalidating all dashboard caches
    pub fn dashboard_pattern() -> String {
        "dashboard:*".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats {
            hits: 80,
            misses: 20,
            invalidations: 5,
        };
        assert_eq!(stats.hit_rate(), 80.0);
    }

    #[test]
    fn test_cache_stats_hit_rate_zero() {
        let stats = CacheStats {
            hits: 0,
            misses: 0,
            invalidations: 0,
        };
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_key_builders() {
        assert_eq!(keys::anchor_list(50, 0), "anchor:list:50:0");
        assert_eq!(keys::anchor_detail("123"), "anchor:detail:123");
        assert_eq!(keys::anchor_by_account("GA123"), "anchor:account:GA123");
        assert_eq!(keys::dashboard_stats(), "dashboard:stats");
        assert_eq!(keys::anchor_pattern(), "anchor:*");
    }
}
