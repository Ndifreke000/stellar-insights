use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::distributed_lock::DistributedLock;
use crate::observability::job_metrics::JobMetricsCollector;

use crate::cache::CacheManager;
use crate::database::Database;
use crate::ingestion::DataIngestionService;
use crate::rpc::StellarRpcClient;
use crate::services::price_feed::PriceFeedClient;

pub use crate::jobs::config::{
    JobConfigResponse, ScheduleRunRequest, ScheduleRunResponse, UpdateJobConfigRequest,
};
pub use crate::jobs::persistence::{CatchUpPolicy, JobPersistence};

/// Maximum tolerated drift between the local wall clock and the ledger close
/// time before we log a warning. Snapshot epochs are anchored to ledger close
/// times, so a skewed server clock must not change which epoch a ledger lands
/// in. This constant only drives observability, not epoch assignment.
pub const CLOCK_SKEW_TOLERANCE_SECS: i64 = 300;

#[derive(Clone)]
pub struct JobConfig {
    pub name: String,
    pub interval_seconds: u64,
    pub enabled: bool,
    pub catch_up_policy: CatchUpPolicy,
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
            catch_up_policy: CatchUpPolicy::LatestOnly,
        }
    }
}

pub struct JobScheduler {
    handles: Vec<JoinHandle<()>>,
    pool: Option<Arc<Database>>,
    persistence: Option<Arc<JobPersistence>>,
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
            pool: None,
            persistence: None,
        }
    }

    /// Create a new scheduler with database pool
    pub fn with_pool(db: Arc<Database>, persistence: Arc<JobPersistence>) -> Self {
        Self {
            handles: Vec::new(),
            pool: Some(db),
            persistence: Some(persistence),
        }
    }

    pub fn add_job<F>(&mut self, config: JobConfig, job_fn: F)
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        if !config.enabled {
            info!("Job '{}' is disabled, skipping", config.name);
            return;
        }

        info!(
            "Scheduling job '{}' to run every {} seconds with catch-up policy: {:?}",
            config.name, config.interval_seconds, config.catch_up_policy
        );

        let persistence = self.persistence.clone();
        let handle = tokio::spawn(async move {
            let lock = DistributedLock::shared().await;
            let interval = Duration::from_secs(config.interval_seconds);
            let job_name = config.name.clone();
            let job_fn = Arc::new(job_fn);

            loop {
                // Check for missed runs based on catch-up policy. Missed-run
                // detection is anchored to the ledger close time rather than
                // the local wall clock so that a skewed server clock does not
                // cause spurious catch-up runs (and thus divergent epochs).
                if let Some(ref persistence) = persistence {
                    let now = ledger_anchored_now().await;
                    match persistence.find_missed_runs(&job_name, now).await {
                        Ok(missed) => {
                            for scheduled_for in missed {
                                match persistence.get_or_create_run(&job_name, scheduled_for).await {
                                    Ok(mut run) => {
                                        run.skip();
                                        if let Err(e) = persistence.fail_run(run.id, "Skipped by catch-up policy").await {
                                            warn!("Failed to record skipped run: {}", e);
                                        }
                                    }
                                    Err(e) => warn!("Failed to get or create run: {}", e),
                                }
                            }
                        }
                        Err(e) => warn!("Failed to find missed runs: {}", e),
                    }
                }

                // Wait for next interval
                tokio::time::sleep(interval).await;

                let lock_key = format!("job-lock:{}", config.name);
                let lock_ttl = Duration::from_secs(config.interval_seconds.saturating_sub(5).max(1));
                let Some(guard) = lock.try_acquire(&lock_key, lock_ttl).await else {
                    info!(
                        "Job '{}' skipped — another instance holds the lock (or Redis is unavailable)",
                        config.name
                    );
                    continue;
                };

                // Execute job with metrics tracking
                let _metrics = JobMetricsCollector::new(&job_name);
                let started = std::time::Instant::now();
                let job = job_fn();
                tokio::pin!(job);
                let mut renew = tokio::time::interval((lock_ttl / 3).max(Duration::from_secs(1)));
                renew.tick().await;
                let result = loop {
                    tokio::select! {
                        result = &mut job => break result,
                        _ = renew.tick() => {
                            if !guard.extend(lock_ttl).await {
                                tracing::warn!("Job '{}' lost its lock while running", config.name);
                            }
                        }
                    }
                };
                match lock_ttl.checked_sub(started.elapsed()) {
                    Some(remaining) if !remaining.is_zero() => {
                        guard.extend(remaining).await;
                    }
                    _ => guard.release().await,
                }

                match result {
                    Ok(_) => {
                        _metrics.complete_success();
                    }
                    Err(e) => {
                        _metrics.complete_failure(&e.to_string());
                    }
                }
            }
        });

        self.handles.push(handle);
    }

    pub fn start(
        db: Arc<Database>,
        cache: Arc<CacheManager>,
        rpc: Arc<StellarRpcClient>,
        ingestion: Arc<DataIngestionService>,
        price_feed: Arc<PriceFeedClient>,
    ) -> Self {
        let persistence = Arc::new(JobPersistence::new(db.pool().clone()));

        // Initialize job configs asynchronously on the active runtime.
        let init_persistence = Arc::clone(&persistence);
        tokio::spawn(async move {
            if let Err(e) = init_persistence.initialize_job_configs().await {
                warn!("Failed to initialize job configs: {}", e);
            }
        });

        let mut scheduler = Self::with_pool(db, persistence);

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
                price_feed.update_prices().await?;
                Ok(())
            })
        });

        scheduler
    }
}

/// Returns a wall-clock timestamp anchored to the latest ledger close time
/// when available, falling back to the local clock otherwise.
///
/// Snapshot epochs are defined by ledger sequence/close time, so scheduling
/// decisions that feed into epoch assignment must not depend on the raw local
/// clock. When the local clock drifts beyond [`CLOCK_SKEW_TOLERANCE_SECS`] we
/// log a warning but still return the ledger-anchored time so that all
/// instances agree on which epoch a ledger belongs to.
async fn ledger_anchored_now() -> chrono::DateTime<chrono::Utc> {
    let local = chrono::Utc::now();
    match crate::rpc::latest_ledger_close_time().await {
        Some(close_time) => {
            let skew = (local - close_time).num_seconds();
            if skew.abs() > CLOCK_SKEW_TOLERANCE_SECS {
                warn!(
                    "Local clock skew of {}s exceeds tolerance ({}s); using ledger close time for epoch anchoring",
                    skew, CLOCK_SKEW_TOLERANCE_SECS
                );
            }
            close_time
        }
        None => local,
    }
}
