/// Cache invalidation service
/// Handles cache invalidation on data mutations and ingestion events

use std::sync::Arc;
use tracing::{debug, warn};

use crate::cache::{RedisCache, CacheKey};

pub struct CacheInvalidationService {
    cache: Arc<RedisCache>,
}

impl CacheInvalidationService {
    pub fn new(cache: Arc<RedisCache>) -> Self {
        Self { cache }
    }

    /// Invalidate all corridor-related caches
    pub async fn invalidate_corridors(&self) {
        debug!("Invalidating corridor caches");
        if let Err(e) = self.cache.delete_pattern(&CacheKey::corridor_pattern()).await {
            warn!("Failed to invalidate corridor cache: {}", e);
        }
    }

    /// Invalidate all anchor-related caches
    pub async fn invalidate_anchors(&self) {
        debug!("Invalidating anchor caches");
        if let Err(e) = self.cache.delete_pattern(&CacheKey::anchor_pattern()).await {
            warn!("Failed to invalidate anchor cache: {}", e);
        }
    }

    /// Invalidate dashboard caches
    pub async fn invalidate_dashboard(&self) {
        debug!("Invalidating dashboard caches");
        if let Err(e) = self.cache.delete_pattern(&CacheKey::dashboard_pattern()).await {
            warn!("Failed to invalidate dashboard cache: {}", e);
        }
    }

    /// Invalidate specific corridor metrics
    pub async fn invalidate_corridor_metrics(&self, corridor_key: &str) {
        debug!("Invalidating metrics for corridor: {}", corridor_key);
        let key = CacheKey::corridor_metrics(corridor_key);
        if let Err(e) = self.cache.delete(&key).await {
            warn!("Failed to invalidate corridor metrics cache: {}", e);
        }
    }

    /// Invalidate specific anchor data
    pub async fn invalidate_anchor(&self, anchor_id: &str) {
        debug!("Invalidating data for anchor: {}", anchor_id);
        
        if let Err(e) = self.cache.delete(&CacheKey::anchor_data(anchor_id)).await {
            warn!("Failed to invalidate anchor data cache: {}", e);
        }
        if let Err(e) = self.cache.delete(&CacheKey::anchor_detail(anchor_id)).await {
            warn!("Failed to invalidate anchor detail cache: {}", e);
        }
        if let Err(e) = self.cache.delete(&CacheKey::anchor_assets(anchor_id)).await {
            warn!("Failed to invalidate anchor assets cache: {}", e);
        }
    }

    /// Invalidate all caches (full refresh)
    pub async fn invalidate_all(&self) {
        debug!("Invalidating all caches");
        if let Err(e) = self.cache.clear_all().await {
            warn!("Failed to clear all caches: {}", e);
        }
    }

    /// Called after metrics ingestion completes
    pub async fn on_metrics_ingestion_complete(&self) {
        debug!("Metrics ingestion complete, invalidating related caches");
        self.invalidate_corridors().await;
        self.invalidate_dashboard().await;
    }

    /// Called after anchor metrics update
    pub async fn on_anchor_metrics_updated(&self, anchor_id: &str) {
        debug!("Anchor metrics updated: {}", anchor_id);
        self.invalidate_anchor(anchor_id).await;
        self.invalidate_dashboard().await;
    }

    /// Called after corridor metrics update
    pub async fn on_corridor_metrics_updated(&self, corridor_key: &str) {
        debug!("Corridor metrics updated: {}", corridor_key);
        self.invalidate_corridor_metrics(corridor_key).await;
        self.invalidate_dashboard().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invalidation_service_creation() {
        let cache = Arc::new(RedisCache::new().await.unwrap());
        let service = CacheInvalidationService::new(cache);
        
        // Should not panic
        service.invalidate_all().await;
    }
}
