//! Background job for running retention policy.
//!
//! This job runs periodically to delete old events using batched deletion
//! to avoid SQLite lock contention, and to roll up analytics to aggregates.

use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::distributed_lock::DistributedLock;
use crate::services::retention::RetentionService;
use crate::state::AppState;

/// Retention job configuration.
#[derive(Debug, Clone)]
pub struct RetentionJobConfig {
    /// Enable the retention job.
    pub enabled: bool,
    /// Run interval in seconds.
    pub interval_seconds: u64,
    /// Default retention in days for events.
    pub default_days: i64,
    /// Default batch size for deletions.
    pub batch_size: i64,
    /// Sleep between batches in milliseconds.
    pub sleep_ms: u64,
    /// Enable incremental VACUUM.
    pub vacuum_enabled: bool,
    /// Pages to vacuum per batch.
    pub vacuum_pages_per_batch: i32,
    /// Maximum pages to vacuum.
    pub vacuum_max_pages: i32,
}

impl Default for RetentionJobConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 86400, // Run once per day
            default_days: 30,
            batch_size: 1000,
            sleep_ms: 100,
            vacuum_enabled: true,
            vacuum_pages_per_batch: 100,
            vacuum_max_pages: 10000,
        }
    }
}

impl RetentionJobConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let enabled = std::env::var("JOB_RETENTION_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let interval_seconds = std::env::var("JOB_RETENTION_INTERVAL_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(86400);

        let default_days = std::env::var("RETENTION_DEFAULT_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let batch_size = std::env::var("RETENTION_BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        let sleep_ms = std::env::var("RETENTION_SLEEP_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        let vacuum_enabled = std::env::var("VACUUM_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let vacuum_pages_per_batch = std::env::var("VACUUM_PAGES_PER_BATCH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        let vacuum_max_pages = std::env::var("VACUUM_MAX_PAGES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10000);

        Self {
            enabled,
            interval_seconds,
            default_days,
            batch_size,
            sleep_ms,
            vacuum_enabled,
            vacuum_pages_per_batch,
            vacuum_max_pages,
        }
    }
}

/// Background job for running retention.
pub struct RetentionJob {
    config: RetentionJobConfig,
    app_state: AppState,
}

impl RetentionJob {
    /// Create a new retention job.
    pub fn new(config: RetentionJobConfig, app_state: AppState) -> Self {
        Self { config, app_state }
    }

    /// Run the retention job once.
    pub async fn run_once(&self) -> Result<()> {
        info!("Starting retention job");

        let config = self.config.clone();
        let service = RetentionService::new(self.app_state.clone(), None);

        // Configure retention with our settings
        let config = crate::models::retention::RetentionConfig {
            default_days: config.default_days,
            table_configs: {
                let mut map = std::collections::HashMap::new();
                
                map.insert(
                    "contract_events".to_string(),
                    crate::models::retention::TableRetention {
                        days: config.default_days,
                        batch_size: config.batch_size,
                        sleep_ms: config.sleep_ms,
                        preserve_aggregates: true,
                    },
                );
                
                map.insert(
                    "corridor_metrics".to_string(),
                    crate::models::retention::TableRetention {
                        days: 365, // Keep corridor metrics for 1 year
                        batch_size: config.batch_size,
                        sleep_ms: config.sleep_ms,
                        preserve_aggregates: true,
                    },
                );
                
                map
            },
        };

        // Set vacuum config
        let vacuum_config = crate::models::retention::VacuumConfig {
            enabled: config.vacuum_enabled,
            pages_per_batch: config.vacuum_pages_per_batch,
            max_pages: config.vacuum_max_pages,
        };

        // Run retention
        let results = service.run_retention().await;

        // Log results
        for result in &results {
            info!(
                "Retention completed for {}: {} rows deleted in {} batches, {}ms elapsed",
                result.table, result.deleted_rows, result.batches_processed, result.elapsed_ms
            );

            for warning in &result.warnings {
                warn!("Retention warning for {}: {}", result.table, warning);
            }
        }

        info!("Retention job completed successfully");
        Ok(())
    }

    /// Spawn the retention job as a background task.
    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let lock = DistributedLock::shared().await;
            let mut interval = tokio::time::interval(Duration::from_secs(self.config.interval_seconds));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                // Only run if we have the lock
                let lock_key = "job-lock:retention";
                let lock_ttl = Duration::from_secs(self.config.interval_seconds.saturating_sub(60).max(60));
                
                let Some(_guard) = lock.try_acquire(&lock_key, lock_ttl).await else {
                    info!("Retention job skipped — another instance holds the lock");
                    continue;
                };

                if let Err(e) = self.run_once().await {
                    error!("Retention job failed: {}", e);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RetentionJobConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.interval_seconds, 86400);
        assert_eq!(config.default_days, 30);
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.sleep_ms, 100);
        assert!(config.vacuum_enabled);
        assert_eq!(config.vacuum_pages_per_batch, 100);
        assert_eq!(config.vacuum_max_pages, 10000);
    }

    #[test]
    fn test_config_from_env() {
        // This test would require setting environment variables
        // For now, just verify the struct can be created
        let config = RetentionJobConfig::from_env();
        
        // Verify defaults
        assert!(config.enabled);
        assert!(config.interval_seconds > 0);
    }
}
