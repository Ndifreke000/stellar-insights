use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::distributed_lock::DistributedLock;
use crate::observability::job_metrics::instrument_job;

use crate::cache::CacheManager;
use crate::database::Database;
use crate::ingestion::DataIngestionService;
use crate::rpc::StellarRpcClient;
use crate::services::price_feed::PriceFeedClient;

#[derive(Clone)]
pub struct JobConfig {
    pub name: String,
    pub interval_seconds: u64,
    pub enabled: bool,
}

impl JobConfig {
    #[must_use]
    pub fn from_env(name: &str, default_interval: u64) -> Self {
        let env_prefix = format!("JOB_{}", name.to_uppercase().replace('-', "_"));
        let enabled = std::env::var(format!("{env_prefix}_ENABLED"))
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);
        let interval_seconds = std::env::var(format!("{env_prefix}_INTERVAL_SECONDS"))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(default_interval);

        Self {
            name: name.to_string(),
            interval_seconds,
            enabled,
        }
    }
}

pub struct JobScheduler {
    handles: Vec<JoinHandle<()>>,
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl JobScheduler {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    pub fn add_job<F>(&mut self, config: JobConfig, job_fn: F)
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
            + Send
            + 'static,
    {
        if !config.enabled {
            info!("Job '{}' is disabled, skipping", config.name);
            return;
        }

        info!(
            "Scheduling job '{}' to run every {} seconds",
            config.name, config.interval_seconds
        );

        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.interval_seconds));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;
                let lock_key = format!("job-lock:{}", config.name);
                // TTL is slightly shorter than the interval so the lock expires before
                // the next tick, allowing any instance to acquire it next round.
                let lock_ttl = config.interval_seconds.saturating_sub(5).max(1);
                if !DistributedLock::try_acquire(&redis_url, &lock_key, lock_ttl).await {
                    info!("Job '{}' skipped — another instance holds the lock", config.name);
                    continue;
                }
                let job_name = config.name.clone();
                instrument_job!(job_name.as_str(), { job_fn().await });
            }
        });

        self.handles.push(handle);
    }

    pub fn start(
        _db: Arc<Database>,
        cache: Arc<CacheManager>,
        _rpc: Arc<StellarRpcClient>,
        ingestion: Arc<DataIngestionService>,
        price_feed: Arc<PriceFeedClient>,
    ) -> Self {
        let mut scheduler = Self::new();

        // Corridor refresh job
        let config = JobConfig::from_env("corridor-refresh", 300);
        let cache_clone = Arc::clone(&cache);
        let ingestion_clone = Arc::clone(&ingestion);
        scheduler.add_job(config, move || {
            let cache = Arc::clone(&cache_clone);
            let ingestion = Arc::clone(&ingestion_clone);
            Box::pin(async move {
                ingestion.sync_all_metrics().await?;
                cache.invalidate_pattern("corridor:*").await?;
                Ok(())
            })
        });

        // Anchor refresh job
        let config = JobConfig::from_env("anchor-refresh", 600);
        let cache_clone = Arc::clone(&cache);
        scheduler.add_job(config, move || {
            let cache = Arc::clone(&cache_clone);
            Box::pin(async move {
                cache.invalidate_pattern("anchor:*").await?;
                Ok(())
            })
        });

        // Price feed update job
        let config = JobConfig::from_env("price-feed-update", 900);
        let price_feed_clone = Arc::clone(&price_feed);
        scheduler.add_job(config, move || {
            let price_feed = Arc::clone(&price_feed_clone);
            Box::pin(async move {
                price_feed.warm_cache().await?;
                Ok(())
            })
        });

        // Cache cleanup job
        let config = JobConfig::from_env("cache-cleanup", 3600);
        let cache_clone = Arc::clone(&cache);
        scheduler.add_job(config, move || {
            let cache = Arc::clone(&cache_clone);
            Box::pin(async move {
                cache.cleanup_expired()?;
                Ok(())
            })
        });

        scheduler
    }

    pub fn shutdown(self) {
        info!("Shutting down job scheduler");
        for handle in self.handles {
            handle.abort();
        }
    }
}
