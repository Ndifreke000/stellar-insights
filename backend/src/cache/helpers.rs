use crate::cache::CacheManager;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;
use std::sync::Arc;

/// Executes a query using a cache-aside strategy.
pub async fn cached_query<T, F, Fut>(
    cache: &Arc<CacheManager>,
    key: &str,
    ttl: usize,
    query_fn: F,
) -> anyhow::Result<T>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    if let Some(cached) = cache.get::<T>(key).await? {
        tracing::debug!("Cache hit for key: {}", key);
        return Ok(cached);
    }

    tracing::debug!("Cache miss for key: {}", key);

    let result = query_fn().await?;

    // Cache write is best-effort so reads are never blocked by cache backend issues.
    if let Err(error) = cache.set(key, &result, ttl).await {
        tracing::warn!("Failed to cache result for key {}: {}", key, error);
    }

    Ok(result)
}

/// Executes a query with a cache key generated from serialized params.
pub async fn cached_query_with_params<T, P, F, Fut>(
    cache: &Arc<CacheManager>,
    key_prefix: &str,
    params: &P,
    ttl: usize,
    query_fn: F,
) -> anyhow::Result<T>
where
    T: Serialize + DeserializeOwned,
    P: Serialize,
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let key = build_param_cache_key(key_prefix, params);
    cached_query(cache, &key, ttl, query_fn).await
}

/// Builds a deterministic cache key from a prefix and serializable params.
pub fn build_param_cache_key<P: Serialize>(key_prefix: &str, params: &P) -> String {
    let params_hash = calculate_hash(params);
    format!("{}:{}", key_prefix, params_hash)
}

fn calculate_hash<T: Serialize>(value: &T) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let json = serde_json::to_string(value).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct TestParams {
        limit: i64,
        offset: i64,
    }

    #[test]
    fn test_build_param_cache_key_is_stable() {
        let params = TestParams {
            limit: 10,
            offset: 0,
        };

        let key_a = build_param_cache_key("corridor:list", &params);
        let key_b = build_param_cache_key("corridor:list", &params);

        assert_eq!(key_a, key_b);
        assert!(key_a.starts_with("corridor:list:"));
    }
}
