use anyhow::Result;
use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{error, info, warn};

use crate::models::asset_verification::VerifiedAsset;
use crate::services::asset_verifier::AssetVerifier;

/// Configuration for asset revalidation job
#[derive(Debug, Clone)]
pub struct RevalidationConfig {
    /// Whether the job is enabled
    pub enabled: bool,
    /// Interval between job runs in hours
    pub interval_hours: u64,
    /// Number of assets to revalidate per batch
    pub batch_size: usize,
    /// Maximum age in days before an asset needs revalidation
    pub max_age_days: i64,
}

impl Default for RevalidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_hours: 24,
            batch_size: 100,
            max_age_days: 7,
        }
    }
}

/// Asset revalidation job
pub struct AssetRevalidationJob {
    pool: SqlitePool,
    config: RevalidationConfig,
}

impl AssetRevalidationJob {
    /// Create a new asset revalidation job
    pub fn new(pool: SqlitePool, config: RevalidationConfig) -> Self {
        Self { pool, config }
    }

    /// Start the revalidation job
    pub async fn start(self: Arc<Self>) {
        if !self.config.enabled {
            info!("Asset revalidation job is disabled");
            return;
        }

        info!(
            "Starting asset revalidation job (interval: {}h, batch_size: {}, max_age: {}d)",
            self.config.interval_hours, self.config.batch_size, self.config.max_age_days
        );

        let mut ticker = interval(TokioDuration::from_secs(self.config.interval_hours * 3600));

        loop {
            ticker.tick().await;

            if let Err(e) = self.run_revalidation().await {
                error!("Asset revalidation job failed: {}", e);
            }
        }
    }

    /// Run a single revalidation cycle
    async fn run_revalidation(&self) -> Result<()> {
        info!("Starting asset revalidation cycle");

        let cutoff_date = Utc::now() - Duration::days(self.config.max_age_days);

        // Get assets that need revalidation (oldest first)
        let assets = sqlx::query_as::<_, VerifiedAsset>(
            r#"
            SELECT * FROM verified_assets
            WHERE last_verified_at IS NULL OR last_verified_at < ?
            ORDER BY last_verified_at ASC NULLS FIRST
            LIMIT ?
            "#,
        )
        .bind(cutoff_date)
        .bind(self.config.batch_size as i64)
        .fetch_all(&self.pool)
        .await?;

        if assets.is_empty() {
            info!("No assets need revalidation");
            return Ok(());
        }

        info!("Revalidating {} assets", assets.len());

        let verifier = AssetVerifier::new(self.pool.clone())?;
        let mut success_count = 0;
        let mut failure_count = 0;

        for asset in assets {
            match verifier
                .verify_asset(&asset.asset_code, &asset.asset_issuer)
                .await
            {
                Ok(_) => {
                    success_count += 1;
                    info!(
                        "Revalidated asset: {}-{}",
                        asset.asset_code, asset.asset_issuer
                    );
                }
                Err(e) => {
                    failure_count += 1;
                    warn!(
                        "Failed to revalidate asset {}-{}: {}",
                        asset.asset_code, asset.asset_issuer, e
                    );
                }
            }

            // Small delay to avoid overwhelming external APIs
            tokio::time::sleep(TokioDuration::from_millis(100)).await;
        }

        info!(
            "Revalidation cycle complete: {} succeeded, {} failed",
            success_count, failure_count
        );

        Ok(())
    }

    /// Manually trigger revalidation for a specific asset
    pub async fn revalidate_asset(&self, asset_code: &str, asset_issuer: &str) -> Result<()> {
        info!(
            "Manually revalidating asset: {}-{}",
            asset_code, asset_issuer
        );

        let verifier = AssetVerifier::new(self.pool.clone())?;
        verifier.verify_asset(asset_code, asset_issuer).await?;

        info!(
            "Successfully revalidated asset: {}-{}",
            asset_code, asset_issuer
        );

        Ok(())
    }

    /// Get revalidation statistics
    pub async fn get_stats(&self) -> Result<RevalidationStats> {
        let cutoff_date = Utc::now() - Duration::days(self.config.max_age_days);

        let row = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_assets,
                SUM(CASE WHEN last_verified_at IS NULL OR last_verified_at < ? THEN 1 ELSE 0 END) as needs_revalidation,
                SUM(CASE WHEN verification_status = 'verified' THEN 1 ELSE 0 END) as verified_count,
                SUM(CASE WHEN verification_status = 'unverified' THEN 1 ELSE 0 END) as unverified_count,
                SUM(CASE WHEN verification_status = 'suspicious' THEN 1 ELSE 0 END) as suspicious_count
            FROM verified_assets
            "#,
            cutoff_date
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(RevalidationStats {
            total_assets: row.total_assets.unwrap_or(0) as i64,
            needs_revalidation: row.needs_revalidation.unwrap_or(0) as i64,
            verified_count: row.verified_count.unwrap_or(0) as i64,
            unverified_count: row.unverified_count.unwrap_or(0) as i64,
            suspicious_count: row.suspicious_count.unwrap_or(0) as i64,
        })
    }
}

/// Statistics about asset revalidation
#[derive(Debug, Clone)]
pub struct RevalidationStats {
    pub total_assets: i64,
    pub needs_revalidation: i64,
    pub verified_count: i64,
    pub unverified_count: i64,
    pub suspicious_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RevalidationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval_hours, 24);
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.max_age_days, 7);
    }

    #[test]
    fn test_custom_config() {
        let config = RevalidationConfig {
            enabled: false,
            interval_hours: 12,
            batch_size: 50,
            max_age_days: 3,
        };
        assert!(!config.enabled);
        assert_eq!(config.interval_hours, 12);
        assert_eq!(config.batch_size, 50);
        assert_eq!(config.max_age_days, 3);
    }
}
