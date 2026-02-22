use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info};

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
    pub fn from_env(name: &str, default_interval: u64) -> Self {
        let env_prefix = format!("JOB_{}", name.to_uppercase().replace('-', "_"));
        let enabled = std::env::var(format!("{}_ENABLED", env_prefix))
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);
        let interval_seconds = std::env::var(format!("{}_INTERVAL_SECONDS", env_prefix))
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

impl JobScheduler {
    pub fn new() -> Self {
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

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.interval_seconds));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;
                info!("Running job '{}'", config.name);
                match job_fn().await {
                    Ok(_) => info!("Job '{}' completed successfully", config.name),
                    Err(e) => error!("Job '{}' failed: {}", config.name, e),
                }
            }
        });

        self.handles.push(handle);
    }

    pub async fn start(
        db: Arc<Database>,
        cache: Arc<CacheManager>,
        rpc: Arc<StellarRpcClient>,
        ingestion: Arc<DataIngestionService>,
        price_feed: Arc<PriceFeedClient>,
    ) -> Self {
        let mut scheduler = Self::new();

        // Corridor refresh job
        let config = JobConfig::from_env("corridor-refresh", 300);
        let db_clone = Arc::clone(&db);
        let cache_clone = Arc::clone(&cache);
        let rpc_clone = Arc::clone(&rpc);
        let ingestion_clone = Arc::clone(&ingestion);
        scheduler.add_job(config, move || {
            let db = Arc::clone(&db_clone);
            let cache = Arc::clone(&cache_clone);
            let rpc = Arc::clone(&rpc_clone);
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
                cache.cleanup_expired().await?;
                Ok(())
            })
        });

        scheduler
    }

    pub async fn shutdown(self) {
        info!("Shutting down job scheduler");
        for handle in self.handles {
            handle.abort();
        }
    }
}
