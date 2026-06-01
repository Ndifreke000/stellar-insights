use crate::cache::CacheManager;
use std::sync::Arc;

/// Helper trait for cache-aware operations
pub trait CacheAware {
    fn get_or_fetch<T, F>(
        cache: &Arc<CacheManager>,
        key: &str,
        ttl: usize,
        fetch_fn: F,
    ) -> impl std::future::Future<Output = anyhow::Result<T>>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
        F: std::future::Future<Output = anyhow::Result<T>>;
}

/// Implement for unit type to provide static methods
impl CacheAware for () {
    fn get_or_fetch<T, F>(
        cache: &Arc<CacheManager>,
        key: &str,
        ttl: usize,
        fetch_fn: F,
    ) -> impl std::future::Future<Output = anyhow::Result<T>>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
        F: std::future::Future<Output = anyhow::Result<T>>,
    {
        async move {
            // Try to get from cache first
            if let Ok(Some(cached)) = cache.get::<T>(key).await {
                return Ok(cached);
            }

            // Cache miss or error, fetch from source
            let data = fetch_fn.await?;

            // Store in cache (ignore errors, cache is optional)
            let _ = cache.set(key, &data, ttl).await;

            Ok(data)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct TestData {
        value: String,
    }

    #[tokio::test]
    async fn test_cache_aware_get_or_fetch() {
        let cache = Arc::new(
            CacheManager::new(Default::default())
                .await
                .expect("Failed to create cache"),
        );

        let test_data = TestData {
            value: "test".to_string(),
        };

        let result =
            <()>::get_or_fetch(&cache, "test:key", 60, async { Ok(test_data.clone()) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_data);
    }
}
