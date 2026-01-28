use anyhow::{Context, Result};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn, error};

use super::metrics::CacheMetrics;

/// Redis cache wrapper with fallback to memory cache
pub struct RedisCache {
    redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    metrics: CacheMetrics,
    memory_cache: Arc<RwLock<std::collections::HashMap<String, CachedValue>>>,
}

#[derive(Clone, Debug)]
struct CachedValue {
    data: String,
    expires_at: std::time::Instant,
}

impl RedisCache {
    pub async fn new() -> Result<Self> {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let connection = match redis::Client::open(redis_url.as_str()) {
            Ok(client) => match client.get_multiplexed_tokio_connection().await {
                Ok(conn) => {
                    tracing::info!("Connected to Redis for caching");
                    Some(conn)
                }
                Err(e) => {
                    tracing::warn!("Failed to connect to Redis ({}), using memory cache fallback", e);
                    None
                }
            },
            Err(e) => {
                tracing::warn!("Invalid Redis URL ({}), using memory cache fallback", e);
                None
            }
        };

        Ok(Self {
            redis_connection: Arc::new(RwLock::new(connection)),
            metrics: CacheMetrics::new(),
            memory_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Get value from cache
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        // Try Redis first
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match conn.get::<_, String>(key).await {
                Ok(value) => {
                    debug!("Cache hit (Redis): {}", key);
                    self.metrics.record_hit();
                    return serde_json::from_str(&value)
                        .map(Some)
                        .context("Failed to deserialize cached value");
                }
                Err(redis::RedisError { .. }) => {
                    // Key not found in Redis, continue to memory cache
                }
            }
        }

        // Try memory cache
        let memory = self.memory_cache.read().await;
        if let Some(cached) = memory.get(key) {
            if cached.expires_at > std::time::Instant::now() {
                debug!("Cache hit (Memory): {}", key);
                self.metrics.record_hit();
                return serde_json::from_str(&cached.data)
                    .map(Some)
                    .context("Failed to deserialize cached value");
            }
        }
        drop(memory);

        // Clean up expired entry from memory cache
        self.memory_cache.write().await.remove(key);

        debug!("Cache miss: {}", key);
        self.metrics.record_miss();
        Ok(None)
    }

    /// Set value in cache with TTL in seconds
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: usize) -> Result<()> {
        let serialized = serde_json::to_string(value)
            .context("Failed to serialize value for cache")?;

        // Try Redis first
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match conn.set_ex::<_, _, ()>(key, &serialized, ttl_secs as u64).await {
                Ok(_) => {
                    debug!("Cache set (Redis): {} (TTL: {}s)", key, ttl_secs);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to set Redis cache for {}: {}", key, e);
                    self.metrics.record_error();
                    // Fall through to memory cache
                }
            }
        }

        // Fall back to memory cache
        let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(ttl_secs as u64);
        self.memory_cache.write().await.insert(
            key.to_string(),
            CachedValue {
                data: serialized,
                expires_at,
            },
        );
        debug!("Cache set (Memory): {} (TTL: {}s)", key, ttl_secs);

        Ok(())
    }

    /// Delete a specific key
    pub async fn delete(&self, key: &str) -> Result<()> {
        // Delete from Redis
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match conn.del::<_, ()>(key).await {
                Ok(_) => {
                    debug!("Cache deleted (Redis): {}", key);
                    self.metrics.record_invalidation();
                }
                Err(e) => {
                    warn!("Failed to delete Redis cache for {}: {}", key, e);
                }
            }
        }

        // Delete from memory cache
        self.memory_cache.write().await.remove(key);
        debug!("Cache deleted (Memory): {}", key);

        Ok(())
    }

    /// Delete all keys matching a pattern
    pub async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        let mut deleted_count = 0;

        // Delete from Redis
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match conn.keys::<_, Vec<String>>(pattern).await {
                Ok(keys) => {
                    for key in keys {
                        if let Err(e) = conn.del::<_, ()>(&key).await {
                            warn!("Failed to delete Redis key {}: {}", key, e);
                        } else {
                            deleted_count += 1;
                        }
                    }
                    debug!("Cache pattern deleted (Redis): {} ({} keys)", pattern, deleted_count);
                    self.metrics.record_invalidation();
                }
                Err(e) => {
                    warn!("Failed to scan Redis keys for pattern {}: {}", pattern, e);
                }
            }
        }

        // Delete from memory cache
        let mut memory = self.memory_cache.write().await;
        let keys_to_delete: Vec<String> = memory
            .keys()
            .filter(|k| {
                // Simple pattern matching: * matches anything
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    k.starts_with(prefix)
                } else {
                    k.as_str() == pattern
                }
            })
            .cloned()
            .collect();

        for key in keys_to_delete {
            memory.remove(&key);
            deleted_count += 1;
        }
        drop(memory);

        debug!("Cache pattern deleted (Memory): {} ({} keys)", pattern, deleted_count);

        Ok(())
    }

    /// Clear all cache
    pub async fn clear_all(&self) -> Result<()> {
        // Clear Redis
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match redis::cmd("FLUSHDB").query_async::<_, ()>(&mut conn).await {
                Ok(_) => {
                    debug!("Cache cleared (Redis)");
                    self.metrics.record_invalidation();
                }
                Err(e) => {
                    warn!("Failed to clear Redis cache: {}", e);
                }
            }
        }

        // Clear memory cache
        self.memory_cache.write().await.clear();
        debug!("Cache cleared (Memory)");

        Ok(())
    }

    /// Get cache metrics
    pub fn metrics(&self) -> super::metrics::CacheMetricsSummary {
        self.metrics.summary()
    }

    /// Check if Redis is connected
    pub async fn is_redis_connected(&self) -> bool {
        self.redis_connection.read().await.is_some()
    }

    /// Reconnect to Redis (useful after connection loss)
    pub async fn reconnect(&self) -> Result<()> {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        match redis::Client::open(redis_url.as_str()) {
            Ok(client) => match client.get_multiplexed_tokio_connection().await {
                Ok(conn) => {
                    *self.redis_connection.write().await = Some(conn);
                    tracing::info!("Reconnected to Redis");
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to reconnect to Redis: {}", e);
                    Err(e.into())
                }
            },
            Err(e) => {
                error!("Invalid Redis URL: {}", e);
                Err(e.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_cache_set_get() {
        let cache = RedisCache::new().await.unwrap();
        
        let key = "test:key";
        let value = serde_json::json!({"test": "value"});
        
        cache.set(key, &value, 60).await.unwrap();
        let retrieved: Option<serde_json::Value> = cache.get(key).await.unwrap();
        
        assert_eq!(retrieved, Some(value));
    }

    #[tokio::test]
    async fn test_cache_delete() {
        let cache = RedisCache::new().await.unwrap();
        
        let key = "test:delete";
        let value = serde_json::json!({"test": "value"});
        
        cache.set(key, &value, 60).await.unwrap();
        cache.delete(key).await.unwrap();
        
        let retrieved: Option<serde_json::Value> = cache.get(key).await.unwrap();
        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    async fn test_cache_metrics() {
        let cache = RedisCache::new().await.unwrap();
        
        let key = "test:metrics";
        let value = serde_json::json!({"test": "value"});
        
        cache.set(key, &value, 60).await.unwrap();
        let _: Option<serde_json::Value> = cache.get(key).await.unwrap();
        let _: Option<serde_json::Value> = cache.get(key).await.unwrap();
        let _: Option<serde_json::Value> = cache.get("nonexistent").await.unwrap();
        
        let metrics = cache.metrics();
        assert_eq!(metrics.hits, 2);
        assert_eq!(metrics.misses, 1);
    }
}
