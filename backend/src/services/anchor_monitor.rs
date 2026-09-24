use crate::alerts::{AlertManager, AlertType};
use crate::cache::CacheManager;
use crate::database::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::models::{AnchorMetrics, AnchorStatus};

/// TTL for cached anchor performance metrics (1 minute as per issue #1114).
const ANCHOR_METRICS_CACHE_TTL_SECS: usize = 60;

/// Wrapper to make AnchorMetrics serializable for the cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedAnchorMetrics {
    success_rate: f64,
    failure_rate: f64,
    reliability_score: f64,
    total_transactions: i64,
    successful_transactions: i64,
    failed_transactions: i64,
    avg_settlement_time_ms: Option<i32>,
    status: String,
}

impl From<&AnchorMetrics> for CachedAnchorMetrics {
    fn from(m: &AnchorMetrics) -> Self {
        Self {
            success_rate: m.success_rate,
            failure_rate: m.failure_rate,
            reliability_score: m.reliability_score,
            total_transactions: m.total_transactions,
            successful_transactions: m.successful_transactions,
            failed_transactions: m.failed_transactions,
            avg_settlement_time_ms: m.avg_settlement_time_ms,
            status: format!("{:?}", m.status),
        }
    }
}

impl From<CachedAnchorMetrics> for AnchorMetrics {
    fn from(c: CachedAnchorMetrics) -> Self {
        let status = match c.status.to_lowercase().as_str() {
            "yellow" => AnchorStatus::Yellow,
            "red" => AnchorStatus::Red,
            _ => AnchorStatus::Green,
        };
        Self {
            success_rate: c.success_rate,
            failure_rate: c.failure_rate,
            reliability_score: c.reliability_score,
            total_transactions: c.total_transactions,
            successful_transactions: c.successful_transactions,
            failed_transactions: c.failed_transactions,
            avg_settlement_time_ms: c.avg_settlement_time_ms,
            status,
        }
    }
}

pub struct AnchorMonitor {
    db: Arc<Database>,
    alert_manager: Arc<AlertManager>,
    cache: Arc<CacheManager>,
    last_metrics: Arc<tokio::sync::RwLock<HashMap<String, AnchorMetrics>>>,
}

impl AnchorMonitor {
    #[must_use]
    pub fn new(
        db: Arc<Database>,
        alert_manager: Arc<AlertManager>,
        cache: Arc<CacheManager>,
    ) -> Self {
        Self {
            db,
            alert_manager,
            cache,
            last_metrics: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(self) {
        let mut check_interval = interval(Duration::from_secs(300)); // Check every 5 minutes
        tracing::info!("Anchor monitor started");

        loop {
            check_interval.tick().await;
            if let Err(e) = self.check_anchors().await {
                tracing::error!("Anchor monitoring failed: {}", e);
            }
        }
    }

    async fn check_anchors(&self) -> Result<()> {
        let anchors = self.db.list_anchors(1000, 0).await?;

        for anchor in anchors {
            let cache_key = format!("anchor_metrics:{}", anchor.id);

            // Try the cache first (1-minute TTL reduces repeated expensive DB/RPC calls)
            let current_metrics = match self.cache.get::<CachedAnchorMetrics>(&cache_key).await {
                Ok(Some(cached)) => {
                    tracing::debug!(
                        "Cache hit for anchor metrics: {} ({})",
                        anchor.id,
                        anchor.name
                    );
                    AnchorMetrics::from(cached)
                }
                _ => {
                    // Cache miss — fetch from database
                    let metrics = match self
                        .db
                        .get_recent_anchor_performance(&anchor.stellar_account, 60)
                        .await
                    {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to get performance for anchor {} (using neutral metrics): {}",
                                anchor.id,
                                e
                            );
                            AnchorMetrics {
                                success_rate: 100.0,
                                failure_rate: 0.0,
                                reliability_score: 100.0,
                                total_transactions: 0,
                                successful_transactions: 0,
                                failed_transactions: 0,
                                avg_settlement_time_ms: None,
                                status: AnchorStatus::Green,
                            }
                        }
                    };

                    // Store in cache with 1-minute TTL
                    let cached = CachedAnchorMetrics::from(&metrics);
                    if let Err(e) = self
                        .cache
                        .set(&cache_key, &cached, ANCHOR_METRICS_CACHE_TTL_SECS)
                        .await
                    {
                        tracing::warn!("Failed to cache anchor metrics for {}: {}", anchor.id, e);
                    }

                    metrics
                }
            };

            let mut last_metrics = self.last_metrics.write().await;

            if let Some(prev_metrics) = last_metrics.get(&anchor.id) {
                // Alert on significant success rate drop (>10%)
                if current_metrics.success_rate < prev_metrics.success_rate - 10.0 {
                    self.alert_manager.send_anchor_alert(
                        AlertType::AnchorMetricChange,
                        &anchor.id,
                        format!(
                            "Anchor '{}' success rate dropped from {:.1}% to {:.1}%",
                            anchor.name, prev_metrics.success_rate, current_metrics.success_rate
                        ),
                        prev_metrics.success_rate,
                        current_metrics.success_rate,
                    );
                }

                // Alert on significant latency increase (>50%)
                let current_latency = current_metrics.avg_settlement_time_ms.unwrap_or(0) as f64;
                let prev_latency = prev_metrics.avg_settlement_time_ms.unwrap_or(0) as f64;

                if current_latency > prev_latency * 1.5 && prev_latency > 0.0 {
                    self.alert_manager.send_anchor_alert(
                        AlertType::AnchorMetricChange,
                        &anchor.id,
                        format!(
                            "Anchor '{}' latency increased from {:.0}ms to {:.0}ms",
                            anchor.name, prev_latency, current_latency
                        ),
                        prev_latency,
                        current_latency,
                    );
                }
            }

            last_metrics.insert(anchor.id.clone(), current_metrics);
        }

        // Log cache statistics periodically for observability
        let stats = self.cache.get_stats();
        tracing::info!(
            "Anchor monitor cache stats — hits: {}, misses: {}, hit rate: {:.1}%",
            stats.hits,
            stats.misses,
            stats.hit_rate()
        );

        Ok(())
    }
}
