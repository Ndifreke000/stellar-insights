use crate::error::ApiError;
use redis::aio::MultiplexedConnection;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Redis caching layer configuration
#[derive(Debug, Clone)]
pub struct RedisCachingConfig {
    pub redis_url: String,
    pub default_ttl_secs: u64,
    pub max_retries: u32,
    pub connection_timeout_secs: u64,
}

impl Default for RedisCachingConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://127.0.0.1:6379".to_string(),
            default_ttl_secs: 300,
            max_retries: 3,
            connection_timeout_secs: 5,
        }
    }
}

impl RedisCachingConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            redis_url: std::env::var("REDIS_URL").unwrap_or(default.redis_url),
            default_ttl_secs: std::env::var("REDIS_DEFAULT_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.default_ttl_secs),
            max_retries: std::env::var("REDIS_MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.max_retries),
            connection_timeout_secs: std::env::var("REDIS_CONNECTION_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.connection_timeout_secs),
        }
    }
}

/// Redis caching layer for improved performance
pub struct RedisCachingLayer {
    config: RedisCachingConfig,
    connection: Arc<tokio::sync::RwLock<Option<MultiplexedConnection>>>,
}

impl RedisCachingLayer {
    /// Create a new Redis caching layer
    pub async fn new(config: RedisCachingConfig) -> Result<Self, ApiError> {
        let client = redis::Client::open(config.redis_url.as_str()).map_err(|e| {
            error!("Failed to create Redis client: {}", e);
            ApiError::internal("REDIS_CONNECTION_FAILED", "Redis connection failed")
        })?;

        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                error!("Failed to connect to Redis: {}", e);
                ApiError::internal("REDIS_CONNECTION_FAILED", "Redis connection failed")
            })?;

        info!("Redis caching layer initialized");
        Ok(Self {
            config,
            connection: Arc::new(tokio::sync::RwLock::new(Some(connection))),
        })
    }

    /// Set a value in cache with TTL
    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_secs: Option<u64>,
    ) -> Result<(), ApiError> {
        let conn = self.connection.read().await;
        let conn = conn.as_ref().ok_or_else(|| {
            error!("Redis connection not available");
            ApiError::internal("CACHE_UNAVAILABLE", "Cache unavailable")
        })?;

        let mut conn = conn.clone();
        let serialized = serde_json::to_string(value).map_err(|e| {
            error!("Failed to serialize value: {}", e);
            ApiError::internal("CACHE_SERIALIZATION_FAILED", "Serialization failed")
        })?;

        let ttl = ttl_secs.unwrap_or(self.config.default_ttl_secs);

        redis::cmd("SET")
            .arg(key)
            .arg(&serialized)
            .arg("EX")
            .arg(ttl)
            .query_async::<Option<String>>(&mut conn)
            .await
            .map_err(|e| {
                error!("Failed to set cache key {}: {}", key, e);
                ApiError::internal("CACHE_OPERATION_FAILED", "Cache operation failed")
            })?;

        debug!("Cache set: {} (TTL: {}s)", key, ttl);
        Ok(())
    }

    /// Get a value from cache
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, ApiError> {
        let conn = self.connection.read().await;
        let conn = conn.as_ref().ok_or_else(|| {
            error!("Redis connection not available");
            ApiError::internal("CACHE_UNAVAILABLE", "Cache unavailable")
        })?;

        let mut conn = conn.clone();
        let value: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async::<Option<String>>(&mut conn)
            .await
            .map_err(|e| {
                error!("Failed to get cache key {}: {}", key, e);
                ApiError::internal("CACHE_OPERATION_FAILED", "Cache operation failed")
            })?;

        match value {
            Some(v) => {
                let deserialized = serde_json::from_str(&v).map_err(|e| {
                    error!("Failed to deserialize cache value: {}", e);
                    ApiError::internal("CACHE_DESERIALIZATION_FAILED", "Deserialization failed")
                })?;
                debug!("Cache hit: {}", key);
                Ok(Some(deserialized))
            }
            None => {
                debug!("Cache miss: {}", key);
                Ok(None)
            }
        }
    }

    /// Delete a key from cache
    pub async fn delete(&self, key: &str) -> Result<(), ApiError> {
        let conn = self.connection.read().await;
        let conn = conn.as_ref().ok_or_else(|| {
            error!("Redis connection not available");
            ApiError::internal("CACHE_UNAVAILABLE", "Cache unavailable")
        })?;

        let mut conn = conn.clone();
        redis::cmd("DEL")
            .arg(key)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| {
                error!("Failed to delete cache key {}: {}", key, e);
                ApiError::internal("CACHE_OPERATION_FAILED", "Cache operation failed")
            })?;

        debug!("Cache deleted: {}", key);
        Ok(())
    }

    /// Clear all cache
    pub async fn clear(&self) -> Result<(), ApiError> {
        let conn = self.connection.read().await;
        let conn = conn.as_ref().ok_or_else(|| {
            error!("Redis connection not available");
            ApiError::internal("CACHE_UNAVAILABLE", "Cache unavailable")
        })?;

        let mut conn = conn.clone();
        redis::cmd("FLUSHDB")
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| {
                error!("Failed to clear cache: {}", e);
                ApiError::internal("CACHE_OPERATION_FAILED", "Cache operation failed")
            })?;

        info!("Cache cleared");
        Ok(())
    }

    /// Get cache statistics
    pub async fn stats(&self) -> Result<CacheStats, ApiError> {
        let conn = self.connection.read().await;
        let conn = conn.as_ref().ok_or_else(|| {
            error!("Redis connection not available");
            ApiError::internal("CACHE_UNAVAILABLE", "Cache unavailable")
        })?;

        let mut conn = conn.clone();
        let info: String = redis::cmd("INFO")
            .arg("stats")
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                error!("Failed to get cache stats: {}", e);
                ApiError::internal("CACHE_OPERATION_FAILED", "Cache operation failed")
            })?;

        Ok(CacheStats {
            info,
            connection_available: true,
        })
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub info: String,
    pub connection_available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_caching_config_default() {
        let config = RedisCachingConfig::default();
        assert_eq!(config.default_ttl_secs, 300);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_redis_caching_config_from_env() {
        std::env::set_var("REDIS_URL", "redis://localhost:6379");
        std::env::set_var("REDIS_DEFAULT_TTL", "600");
        let config = RedisCachingConfig::from_env();
        assert_eq!(config.redis_url, "redis://localhost:6379");
        assert_eq!(config.default_ttl_secs, 600);
    }
}
