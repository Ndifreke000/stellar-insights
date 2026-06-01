use crate::cache::{keys, CacheManager};
use std::sync::Arc;

/// Service for managing cache invalidation on data updates
pub struct CacheInvalidationService {
    cache: Arc<CacheManager>,
}

impl CacheInvalidationService {
    pub fn new(cache: Arc<CacheManager>) -> Self {
        Self { cache }
    }

    /// Invalidate all anchor-related caches
    pub async fn invalidate_anchors(&self) -> anyhow::Result<()> {
        tracing::info!("Invalidating anchor caches");
        self.cache.delete_pattern(&keys::anchor_pattern()).await?;
        Ok(())
    }

    /// Invalidate specific anchor caches
    pub async fn invalidate_anchor(&self, anchor_id: &str) -> anyhow::Result<()> {
        tracing::info!("Invalidating cache for anchor: {}", anchor_id);
        self.cache.delete(&keys::anchor_detail(anchor_id)).await?;
        self.cache.delete(&keys::anchor_assets(anchor_id)).await?;
        // Also invalidate the list caches since they contain this anchor
        self.cache.delete_pattern(&keys::anchor_pattern()).await?;
        Ok(())
    }

    /// Invalidate anchor by account
    pub async fn invalidate_anchor_by_account(&self, account: &str) -> anyhow::Result<()> {
        tracing::info!("Invalidating cache for anchor account: {}", account);
        self.cache.delete(&keys::anchor_by_account(account)).await?;
        // Also invalidate list caches
        self.cache.delete_pattern(&keys::anchor_pattern()).await?;
        Ok(())
    }

    /// Invalidate all corridor-related caches
    pub async fn invalidate_corridors(&self) -> anyhow::Result<()> {
        tracing::info!("Invalidating corridor caches");
        self.cache.delete_pattern(&keys::corridor_pattern()).await?;
        Ok(())
    }

    /// Invalidate specific corridor cache
    pub async fn invalidate_corridor(&self, corridor_key: &str) -> anyhow::Result<()> {
        tracing::info!("Invalidating cache for corridor: {}", corridor_key);
        self.cache
            .delete(&keys::corridor_detail(corridor_key))
            .await?;
        // Also invalidate the list caches since they contain this corridor
        self.cache.delete_pattern(&keys::corridor_pattern()).await?;
        Ok(())
    }

    /// Invalidate dashboard caches
    pub async fn invalidate_dashboard(&self) -> anyhow::Result<()> {
        tracing::info!("Invalidating dashboard caches");
        self.cache
            .delete_pattern(&keys::dashboard_pattern())
            .await?;
        Ok(())
    }

    /// Invalidate metrics caches
    pub async fn invalidate_metrics(&self) -> anyhow::Result<()> {
        tracing::info!("Invalidating metrics caches");
        self.cache.delete(&keys::metrics_overview()).await
    }

    /// Full cache invalidation (use sparingly)
    pub async fn invalidate_all(&self) -> anyhow::Result<()> {
        tracing::warn!("Performing full cache invalidation");
        self.invalidate_anchors().await?;
        self.invalidate_corridors().await?;
        self.invalidate_dashboard().await?;
        self.invalidate_metrics().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_patterns() {
        assert_eq!(keys::anchor_pattern(), "anchor:*");
        assert_eq!(keys::corridor_pattern(), "corridor:*");
        assert_eq!(keys::dashboard_pattern(), "dashboard:*");
    }
}
