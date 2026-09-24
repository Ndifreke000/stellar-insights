use crate::cache::{keys, CacheManager};
use std::sync::Arc;

/// Service for managing cache invalidation on data updates
pub struct CacheInvalidationService {
    cache: Arc<CacheManager>,
}

impl CacheInvalidationService {
    #[must_use]
    pub const fn new(cache: Arc<CacheManager>) -> Self {
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
        tracing::info!(
            anchor_id = crate::logging::redaction::redact_user_id(anchor_id),
            "Invalidating cache for anchor"
        );
        self.cache.delete(&keys::anchor_detail(anchor_id)).await?;
        self.cache.delete(&keys::anchor_assets(anchor_id)).await?;
        // Also invalidate the list caches since they contain this anchor
        self.cache.delete_pattern(&keys::anchor_pattern()).await?;
        Ok(())
    }

    /// Invalidate anchor by account
    pub async fn invalidate_anchor_by_account(&self, account: &str) -> anyhow::Result<()> {
        tracing::info!(
            account = crate::logging::redaction::redact_account(account),
            "Invalidating cache for anchor account"
        );
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
    use crate::cache::CacheConfig;

    fn make_service() -> CacheInvalidationService {
        let cache = Arc::new(CacheManager::new_in_memory_for_tests(CacheConfig::default()));
        CacheInvalidationService::new(cache)
    }

    #[test]
    fn test_cache_key_patterns() {
        assert_eq!(keys::anchor_pattern(), "anchor:*");
        assert_eq!(keys::corridor_pattern(), "corridor:*");
        assert_eq!(keys::dashboard_pattern(), "dashboard:*");
    }

    #[tokio::test]
    async fn invalidate_anchors_succeeds() {
        let svc = make_service();
        assert!(svc.invalidate_anchors().await.is_ok());
    }

    #[tokio::test]
    async fn invalidate_anchor_succeeds() {
        let svc = make_service();
        assert!(svc.invalidate_anchor("anchor-123").await.is_ok());
    }

    #[tokio::test]
    async fn invalidate_anchor_by_account_succeeds() {
        let svc = make_service();
        assert!(svc.invalidate_anchor_by_account("GABC1234").await.is_ok());
    }

    #[tokio::test]
    async fn invalidate_corridors_succeeds() {
        let svc = make_service();
        assert!(svc.invalidate_corridors().await.is_ok());
    }

    #[tokio::test]
    async fn invalidate_corridor_succeeds() {
        let svc = make_service();
        assert!(svc.invalidate_corridor("USD-EUR").await.is_ok());
    }

    #[tokio::test]
    async fn invalidate_dashboard_succeeds() {
        let svc = make_service();
        assert!(svc.invalidate_dashboard().await.is_ok());
    }

    #[tokio::test]
    async fn invalidate_metrics_succeeds() {
        let svc = make_service();
        assert!(svc.invalidate_metrics().await.is_ok());
    }

    #[tokio::test]
    async fn invalidate_all_succeeds() {
        let svc = make_service();
        assert!(svc.invalidate_all().await.is_ok());
    }
}
