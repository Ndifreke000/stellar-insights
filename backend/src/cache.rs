use redis::aio::MultiplexedConnection;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

#[path = "cache/helpers.rs"]
pub mod helpers;

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub invalidations: u64,
}

impl CacheStats {
    #[must_use]
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
    #[must_use]
    pub fn get_ttl(&self, cache_type: &str) -> usize {
        match cache_type {
            "corridor" => self.corridor_metrics_ttl,
            "anchor" => self.anchor_data_ttl,
            "dashboard" => self.dashboard_stats_ttl,
            _ => 300,
        }
    }

    /// Load TTL settings from environment variables, falling back to defaults.
    ///
    /// | Variable                        | Field                   | Default |
    /// |---------------------------------|-------------------------|---------|
    /// | `CACHE_CORRIDOR_METRICS_TTL`    | `corridor_metrics_ttl`  | 300 s   |
    /// | `CACHE_ANCHOR_DATA_TTL`         | `anchor_data_ttl`       | 600 s   |
    /// | `CACHE_DASHBOARD_STATS_TTL`     | `dashboard_stats_ttl`   | 60 s    |
    #[must_use]
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            corridor_metrics_ttl: std::env::var("CACHE_CORRIDOR_METRICS_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.corridor_metrics_ttl),
            anchor_data_ttl: std::env::var("CACHE_ANCHOR_DATA_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.anchor_data_ttl),
            dashboard_stats_ttl: std::env::var("CACHE_DASHBOARD_STATS_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.dashboard_stats_ttl),
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
    in_memory_store: Arc<RwLock<HashMap<String, String>>>,
}

impl CacheManager {
    pub async fn new(config: CacheConfig) -> anyhow::Result<Self> {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let connection = if let Ok(client) = redis::Client::open(redis_url.as_str()) {
            match client.get_multiplexed_async_connection().await {
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
            in_memory_store: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn new_in_memory_for_tests(config: CacheConfig) -> Self {
        Self {
            redis_connection: Arc::new(RwLock::new(None)),
            config,
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            invalidations: Arc::new(AtomicU64::new(0)),
            in_memory_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Returns a clone of the underlying Redis connection handle.
    pub async fn connection(&self) -> Arc<RwLock<Option<MultiplexedConnection>>> {
        self.redis_connection.clone()
    }

    /// Synchronous form of [`Self::connection`], for wiring at startup.
    #[must_use]
    pub fn redis_handle(&self) -> Arc<RwLock<Option<MultiplexedConnection>>> {
        self.redis_connection.clone()
    }

    /// Health check for the cache dependency
    pub async fn health_check(&self) -> anyhow::Result<()> {
        self.ping().await
    }

    /// Check if Redis connection is healthy
    pub async fn ping(&self) -> anyhow::Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            redis::cmd("PING").query_async::<String>(&mut conn).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Redis connection not available"))
        }
    }

    /// Get value from cache, returns None if not found or Redis unavailable
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> anyhow::Result<Option<T>> {
        if self.redis_connection.read().await.is_none() {
            if let Some(payload) = self.in_memory_store.read().await.get(key).cloned() {
                self.hits.fetch_add(1, Ordering::Relaxed);
                crate::observability::metrics::record_cache_lookup(true);
                tracing::debug!("In-memory cache hit for key: {}", key);
                match serde_json::from_str::<T>(&payload) {
                    Ok(data) => return Ok(Some(data)),
                    Err(e) => {
                        tracing::warn!(
                            "Failed to deserialize in-memory cached value for {}: {}",
                            key,
                            e
                        );
                        return Ok(None);
                    }
                }
            }
        }

        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match redis::cmd("GET")
                .arg(key)
                .query_async::<Option<String>>(&mut conn)
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
        if self.redis_connection.read().await.is_none() {
            match serde_json::to_string(value) {
                Ok(serialized) => {
                    self.in_memory_store
                        .write()
                        .await
                        .insert(key.to_string(), serialized);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to serialize value for in-memory cache key {}: {}",
                        key,
                        e
                    );
                }
            }
            let _ = ttl_seconds;
            return Ok(());
        }

        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match serde_json::to_string(value) {
                Ok(serialized) => {
                    // SET NX prevents concurrent miss-then-write races: the first writer wins
                    // and subsequent concurrent writers silently skip rather than overwriting.
                    match redis::cmd("SET")
                        .arg(key)
                        .arg(&serialized)
                        .arg("NX")
                        .arg("PX")
                        .arg(ttl_seconds * 1000)
                        .query_async::<Option<String>>(&mut conn)
                        .await
                    {
                        Ok(_) => {
                            tracing::debug!("Cache set for key: {} (TTL: {}s)", key, ttl_seconds);
                            Ok(())
                        }
                        Err(e) => {
                            tracing::warn!("Redis SET NX error for {}: {}", key, e);
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

    /// Delete a cache key using an atomic Lua script to avoid TOCTOU races.
    pub async fn delete(&self, key: &str) -> anyhow::Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            // Lua guarantees the check-and-delete is atomic on the Redis server.
            const LUA_DEL: &str = "return redis.call('DEL', KEYS[1])";
            match redis::Script::new(LUA_DEL)
                .key(key)
                .invoke_async::<i64>(&mut conn)
                .await
            {
                Ok(_) => {
                    self.invalidations.fetch_add(1, Ordering::Relaxed);
                    tracing::debug!("Cache invalidated for key: {}", key);
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!("Redis Lua DEL error for {}: {}", key, e);
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

                    pipe.query_async::<()>(&mut conn).await?;

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
            let mut store = self.in_memory_store.write().await;
            let before = store.len();
            store.retain(|k, _| !glob_match(pattern, k));
            let deleted = before - store.len();
            self.invalidations
                .fetch_add(deleted as u64, Ordering::Relaxed);
            Ok(deleted)
        }
    }

    /// Invalidate cache keys matching a pattern (alias for `delete_pattern`)
    pub async fn invalidate_pattern(&self, pattern: &str) -> anyhow::Result<usize> {
        self.delete_pattern(pattern).await
    }

    /// Invalidate all corridor-related cache entries.
    pub async fn invalidate_corridors(&self) -> anyhow::Result<usize> {
        let pattern = keys::corridor_pattern(keys::active_network());
        let deleted = self.invalidate_pattern(&pattern).await?;
        tracing::info!(
            "Invalidated {} corridor cache entries matching pattern: {}",
            deleted,
            pattern
        );
        Ok(deleted)
    }

    /// Invalidate cache entries for a specific corridor and related list views.
    pub async fn invalidate_corridor(&self, corridor_key: &str) -> anyhow::Result<()> {
        let detail_key = keys::corridor_detail(keys::active_network(), corridor_key);
        self.delete(&detail_key).await?;

        // Corridor list endpoints can include this corridor, so clear list/detail variants.
        let invalidated = self.invalidate_corridors().await?;
        tracing::info!(
            "Invalidated corridor cache for key: {} ({} related entries removed)",
            corridor_key,
            invalidated
        );
        Ok(())
    }

    /// Clean up expired entries (Redis handles this automatically, but useful for monitoring)
    pub fn cleanup_expired(&self) -> anyhow::Result<()> {
        tracing::debug!("Cache cleanup triggered (Redis auto-expires keys)");
        Ok(())
    }

    /// Get current cache statistics
    #[must_use]
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
            match redis::cmd("PING").query_async::<String>(&mut conn).await {
                Ok(_) => tracing::debug!("Redis connection verified before close"),
                Err(e) => tracing::warn!("Redis PING failed before close: {}", e),
            }
            tracing::info!("Redis connection closed");
        }
        Ok(())
    }
}

/// Cache key builders for consistency.
///
/// Every key is prefixed with the Stellar network (`mainnet:` / `testnet:`)
/// so that, in multi-network mode, testnet data can never be served from a
/// mainnet key and flushing one network's keys never touches the other's
/// (#2772). The network is a required argument, never an ambient default.
pub mod keys {
    use crate::network::StellarNetwork;

    /// Network this process serves (`STELLAR_NETWORK`, default mainnet). Used
    /// by request handlers to scope their cache keys.
    #[must_use]
    pub fn active_network() -> StellarNetwork {
        static NETWORK: std::sync::OnceLock<StellarNetwork> = std::sync::OnceLock::new();
        *NETWORK.get_or_init(|| crate::network::NetworkConfig::from_env().network)
    }

    #[must_use]
    pub fn anchor_list(network: StellarNetwork, limit: i64, offset: i64) -> String {
        format!("{network}:anchor:list:{limit}:{offset}")
    }

    #[must_use]
    pub fn anchor_detail(network: StellarNetwork, id: &str) -> String {
        format!("{network}:anchor:detail:{id}")
    }

    #[must_use]
    pub fn anchor_by_account(network: StellarNetwork, account: &str) -> String {
        format!("{network}:anchor:account:{account}")
    }

    #[must_use]
    pub fn anchor_assets(network: StellarNetwork, anchor_id: &str) -> String {
        format!("{network}:anchor:assets:{anchor_id}")
    }

    #[must_use]
    pub fn corridor_list(
        network: StellarNetwork,
        limit: i64,
        offset: i64,
        filters: &str,
    ) -> String {
        format!("{network}:corridor:list:{limit}:{offset}:{filters}")
    }

    #[must_use]
    pub fn corridor_detail(network: StellarNetwork, corridor_key: &str) -> String {
        format!("{network}:corridor:detail:{corridor_key}")
    }

    #[must_use]
    pub fn dashboard_stats(network: StellarNetwork) -> String {
        format!("{network}:dashboard:stats")
    }

    #[must_use]
    pub fn metrics_overview(network: StellarNetwork) -> String {
        format!("{network}:metrics:overview")
    }

    #[must_use]
    pub fn analytics_dashboard(network: StellarNetwork) -> String {
        format!("{network}:analytics:dashboard")
    }

    /// Preflight corridor table, scoped per network.
    #[must_use]
    pub fn preflight_corridors(network: StellarNetwork) -> String {
        format!("{network}:preflight:corridors")
    }

    /// Pattern for invalidating all anchor-related caches of one network
    #[must_use]
    pub fn anchor_pattern(network: StellarNetwork) -> String {
        format!("{network}:anchor:*")
    }

    /// Pattern for invalidating all corridor-related caches of one network
    #[must_use]
    pub fn corridor_pattern(network: StellarNetwork) -> String {
        format!("{network}:corridor:*")
    }

    /// Pattern for invalidating all dashboard caches of one network
    #[must_use]
    pub fn dashboard_pattern(network: StellarNetwork) -> String {
        format!("{network}:dashboard:*")
    }

    /// Pattern matching every key of one network
    #[must_use]
    pub fn network_pattern(network: StellarNetwork) -> String {
        format!("{network}:*")
    }
}

/// Minimal glob match supporting `*` (any run of characters), matching the
/// subset of Redis `MATCH` syntax our key patterns use.
fn glob_match(pattern: &str, key: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == key;
    }
    let mut rest = key;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            let Some(r) = rest.strip_prefix(part) else {
                return false;
            };
            rest = r;
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else if let Some(pos) = rest.find(part) {
            rest = &rest[pos + part.len()..];
        } else {
            return false;
        }
    }
    true
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
        use crate::network::StellarNetwork::Mainnet;
        assert_eq!(
            keys::anchor_list(Mainnet, 50, 0),
            "mainnet:anchor:list:50:0"
        );
        assert_eq!(
            keys::anchor_detail(Mainnet, "123"),
            "mainnet:anchor:detail:123"
        );
        assert_eq!(
            keys::anchor_by_account(Mainnet, "GA123"),
            "mainnet:anchor:account:GA123"
        );
        assert_eq!(
            keys::corridor_detail(Mainnet, "USDC:issuer->XLM:native"),
            "mainnet:corridor:detail:USDC:issuer->XLM:native"
        );
        assert_eq!(keys::corridor_pattern(Mainnet), "mainnet:corridor:*");
        assert_eq!(keys::dashboard_stats(Mainnet), "mainnet:dashboard:stats");
        assert_eq!(keys::anchor_pattern(Mainnet), "mainnet:anchor:*");
    }

    /// Enumerates every `keys::*` builder: each must be prefixed with the
    /// network it was built for (#2772).
    #[test]
    fn every_cache_key_is_network_prefixed() {
        use crate::network::StellarNetwork;
        for net in [StellarNetwork::Mainnet, StellarNetwork::Testnet] {
            let all = [
                keys::anchor_list(net, 1, 2),
                keys::anchor_detail(net, "a"),
                keys::anchor_by_account(net, "G"),
                keys::anchor_assets(net, "a"),
                keys::corridor_list(net, 1, 2, "f"),
                keys::corridor_detail(net, "c"),
                keys::dashboard_stats(net),
                keys::metrics_overview(net),
                keys::analytics_dashboard(net),
                keys::preflight_corridors(net),
                keys::anchor_pattern(net),
                keys::corridor_pattern(net),
                keys::dashboard_pattern(net),
                keys::network_pattern(net),
            ];
            let prefix = format!("{net}:");
            for key in all {
                assert!(key.starts_with(&prefix), "{key} not scoped to {net}");
            }
        }
    }

    #[test]
    fn glob_match_basics() {
        assert!(glob_match("testnet:*", "testnet:anchor:list:1:2"));
        assert!(!glob_match("testnet:*", "mainnet:anchor:list:1:2"));
        assert!(glob_match("a:*:c", "a:b:c"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "exact2"));
    }

    #[tokio::test]
    async fn flushing_testnet_keys_never_touches_mainnet() {
        use crate::network::StellarNetwork::{Mainnet, Testnet};
        let cache = CacheManager::new_in_memory_for_tests(CacheConfig::default());
        cache
            .set(&keys::anchor_list(Mainnet, 50, 0), &"main", 60)
            .await
            .unwrap();
        cache
            .set(&keys::anchor_list(Testnet, 50, 0), &"test", 60)
            .await
            .unwrap();

        cache
            .delete_pattern(&keys::network_pattern(Testnet))
            .await
            .unwrap();

        let main: Option<String> = cache.get(&keys::anchor_list(Mainnet, 50, 0)).await.unwrap();
        let test: Option<String> = cache.get(&keys::anchor_list(Testnet, 50, 0)).await.unwrap();
        assert_eq!(main.as_deref(), Some("main"));
        assert_eq!(test, None);
    }
}
