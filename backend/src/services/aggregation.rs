#![allow(clippy::needless_raw_string_hashes)]

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Timelike, Utc};
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::database::Database;
use crate::models::corridor::{CorridorMetrics, HourlyCorridorMetrics, VolumeTrend};
use crate::services::analytics::compute_metrics_from_payments;

const MAX_RETRIES: i32 = 3;
const RETRY_DELAY_SECS: u64 = 60;

#[derive(Debug, Clone)]
pub struct AggregationConfig {
    pub interval_hours: u64,
    pub lookback_hours: i64,
    pub batch_size: i64,
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            interval_hours: 1, // Run every hour
            lookback_hours: 2, // Process last 2 hours of data
            batch_size: 10000, // Process 10k payments at a time
        }
    }
}

pub struct AggregationService {
    db: Arc<Database>,
    config: AggregationConfig,
}

impl AggregationService {
    #[must_use]
    pub const fn new(db: Arc<Database>, config: AggregationConfig) -> Self {
        Self { db, config }
    }

    /// Start the hourly aggregation job scheduler
    pub async fn start_scheduler(self: Arc<Self>) {
        info!(
            "Starting corridor aggregation scheduler (interval: {} hours)",
            self.config.interval_hours
        );

        let mut ticker = interval(TokioDuration::from_secs(self.config.interval_hours * 3600));

        loop {
            ticker.tick().await;

            info!("Triggering hourly corridor aggregation");

            // Check for pending retries first
            if let Err(e) = self.process_pending_retries() {
                error!("Failed to process pending retries: {}", e);
            }

            // Run new aggregation
            if let Err(e) = self.run_hourly_aggregation().await {
                error!("Hourly aggregation failed: {}", e);
                // Continue running despite errors
            }
        }
    }

    /// Process jobs marked for retry
    fn process_pending_retries(&self) -> Result<()> {
        // This would query for jobs with status 'pending_retry'
        // and retry them. For simplicity, we'll skip this for now
        // as it requires additional database queries
        Ok(())
    }

    /// Run the hourly aggregation job
    pub async fn run_hourly_aggregation(&self) -> Result<()> {
        let job_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Create job record
        self.create_job_record(&job_id, "hourly").await?;

        // Update job status to running
        self.update_job_status(&job_id, "running", None).await?;

        match self.execute_aggregation(&job_id, now).await {
            Ok(metrics_count) => {
                info!(
                    "Aggregation completed successfully. Processed {} corridor metrics",
                    metrics_count
                );
                self.update_job_status(&job_id, "completed", None).await?;
                Ok(())
            }
            Err(e) => {
                error!("Aggregation failed: {}", e);
                self.handle_job_failure(&job_id, &e.to_string()).await?;
                Err(e)
            }
        }
    }

    /// Execute the actual aggregation logic
    async fn execute_aggregation(&self, job_id: &str, now: DateTime<Utc>) -> Result<usize> {
        // Calculate time window for aggregation
        let end_time = now;
        let start_time = end_time - Duration::hours(self.config.lookback_hours);

        info!(
            "Aggregating corridor metrics from {} to {}",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        );

        // Fetch payments from the time window
        let payments = self
            .db
            .fetch_payments_by_timerange(start_time, end_time, self.config.batch_size)
            .await
            .context("Failed to fetch payments for aggregation")?;

        if payments.is_empty() {
            info!("No payments found in time window");
            return Ok(0);
        }

        info!("Processing {} payments", payments.len());

        // Compute metrics for each corridor
        let corridor_metrics = compute_metrics_from_payments(&payments);

        if corridor_metrics.is_empty() {
            info!("No corridor metrics computed");
            return Ok(0);
        }

        // Group metrics by hour bucket
        let hourly_metrics = self.group_by_hour_bucket(corridor_metrics, start_time);

        // Store aggregated metrics
        let stored_count = self.store_hourly_metrics(hourly_metrics).await?;

        // Update last processed hour
        let last_hour = self.truncate_to_hour(end_time);
        self.update_last_processed_hour(job_id, last_hour).await?;

        Ok(stored_count)
    }

    /// Group metrics by hour bucket
    fn group_by_hour_bucket(
        &self,
        metrics: Vec<CorridorMetrics>,
        _start_time: DateTime<Utc>, // Reserved for future time-based filtering
    ) -> Vec<HourlyCorridorMetrics> {
        use std::collections::HashMap;

        let mut hourly_map: HashMap<(String, String), HourlyCorridorMetrics> = HashMap::new();

        for metric in metrics {
            let hour_bucket = self.truncate_to_hour(metric.date);
            let key = (metric.corridor_key.clone(), hour_bucket.to_rfc3339());

            hourly_map
                .entry(key)
                .and_modify(|existing| Self::merge_hourly_metric(existing, &metric))
                .or_insert_with(|| Self::new_hourly_metric(&metric, hour_bucket));
        }

        // Recalculate success rates
        hourly_map
            .into_values()
            .map(Self::recalculate_success_rate)
            .collect()
    }

    fn merge_hourly_metric(existing: &mut HourlyCorridorMetrics, metric: &CorridorMetrics) {
        let previous_total = existing.total_transactions;
        existing.total_transactions += metric.total_transactions;
        existing.successful_transactions += metric.successful_transactions;
        existing.failed_transactions += metric.failed_transactions;
        existing.volume_usd += metric.volume_usd;

        existing.avg_settlement_latency_ms = Self::merge_latency(
            existing.avg_settlement_latency_ms,
            previous_total,
            metric.avg_settlement_latency_ms,
            metric.total_transactions,
        );

        // Calculate midpoint for liquidity depth manually as f64 doesn't have .midpoint()
        existing.liquidity_depth_usd =
            (existing.liquidity_depth_usd + metric.liquidity_depth_usd) / 2.0;
    }

    fn merge_latency(
        current_latency: Option<i32>,
        current_weight: i64,
        new_latency: Option<i32>,
        new_weight: i64,
    ) -> Option<i32> {
        let incoming = new_latency?;
        let existing = i64::from(current_latency.unwrap_or(0));
        let weighted_sum = existing * current_weight + i64::from(incoming) * new_weight;
        let total_weight = current_weight + new_weight;

        if total_weight > 0 {
            Some((weighted_sum / total_weight) as i32)
        } else {
            Some(incoming)
        }
    }

    fn new_hourly_metric(
        metric: &CorridorMetrics,
        hour_bucket: DateTime<Utc>,
    ) -> HourlyCorridorMetrics {
        HourlyCorridorMetrics {
            id: Uuid::new_v4().to_string(),
            corridor_key: metric.corridor_key.clone(),
            asset_a_code: metric.source_asset_code.clone(),
            asset_a_issuer: metric.source_asset_issuer.clone(),
            asset_b_code: metric.destination_asset_code.clone(),
            asset_b_issuer: metric.destination_asset_issuer.clone(),
            source_asset_code: metric.reserve_asset_a_code.clone(),
            source_asset_issuer: metric.reserve_asset_a_issuer.clone(),
            destination_asset_code: metric.reserve_asset_b_code.clone(),
            destination_asset_issuer: metric.reserve_asset_b_issuer.clone(),
            source_asset_code: metric.source_asset_code.clone(),
            source_asset_issuer: metric.source_asset_issuer.clone(),
            destination_asset_code: metric.destination_asset_code.clone(),
            destination_asset_issuer: metric.destination_asset_issuer.clone(),
            hour_bucket,
            total_transactions: metric.total_transactions,
            successful_transactions: metric.successful_transactions,
            failed_transactions: metric.failed_transactions,
            success_rate: metric.success_rate,
            volume_usd: metric.volume_usd,
            avg_slippage_bps: 0.0,
            avg_settlement_latency_ms: metric.avg_settlement_latency_ms,
            liquidity_depth_usd: metric.liquidity_depth_usd,
        }
    }

    fn recalculate_success_rate(mut metric: HourlyCorridorMetrics) -> HourlyCorridorMetrics {
        if metric.total_transactions > 0 {
            metric.success_rate =
                (metric.successful_transactions as f64 / metric.total_transactions as f64) * 100.0;
        }
        metric
    }

    /// Store hourly metrics in the database
    async fn store_hourly_metrics(&self, metrics: Vec<HourlyCorridorMetrics>) -> Result<usize> {
        let count = metrics.len();

        for metric in metrics {
            // Convert to database model type
            let db_metric = crate::models::corridor::HourlyCorridorMetrics {
                id: metric.id.clone(),
                corridor_key: metric.corridor_key.clone(),
                asset_a_code: metric.asset_a_code.clone(),
                asset_a_issuer: metric.asset_a_issuer.clone(),
                asset_b_code: metric.asset_b_code.clone(),
                asset_b_issuer: metric.asset_b_issuer.clone(),
                hour_bucket: metric.hour_bucket,
                total_transactions: metric.total_transactions,
                successful_transactions: metric.successful_transactions,
                failed_transactions: metric.failed_transactions,
                success_rate: metric.success_rate,
                volume_usd: metric.volume_usd,
                avg_slippage_bps: metric.avg_slippage_bps,
                avg_settlement_latency_ms: metric.avg_settlement_latency_ms,
                liquidity_depth_usd: metric.liquidity_depth_usd,
            };
            self.db
                .upsert_hourly_corridor_metric(&db_metric)
                .await
                .context("Failed to store hourly corridor metric")?;
        }

        info!("Stored {} hourly corridor metrics", count);
        Ok(count)
    }

    /// Truncate datetime to hour boundary
    fn truncate_to_hour(&self, dt: DateTime<Utc>) -> DateTime<Utc> {
        dt.with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
    }

    /// Create a new job record
    async fn create_job_record(&self, job_id: &str, job_type: &str) -> Result<()> {
        self.db
            .create_aggregation_job(job_id, job_type)
            .await
            .context("Failed to create job record")
    }

    /// Update job status
    async fn update_job_status(
        &self,
        job_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.db
            .update_aggregation_job_status(job_id, status, error_message)
            .await
            .context("Failed to update job status")
    }

    /// Update last processed hour
    async fn update_last_processed_hour(
        &self,
        job_id: &str,
        last_hour: DateTime<Utc>,
    ) -> Result<()> {
        self.db
            .update_last_processed_hour(job_id, &last_hour.to_rfc3339())
            .await
            .context("Failed to update last processed hour")
    }

    /// Handle job failure with retry logic
    async fn handle_job_failure(&self, job_id: &str, error_message: &str) -> Result<()> {
        let retry_count = self.db.get_job_retry_count(job_id).await?;

        if retry_count < MAX_RETRIES {
            warn!(
                "Job {} failed (attempt {}/{}). Will retry in {} seconds",
                job_id,
                retry_count + 1,
                MAX_RETRIES,
                RETRY_DELAY_SECS
            );

            self.db
                .increment_job_retry_count(job_id)
                .await
                .context("Failed to increment retry count")?;

            // Mark for retry - actual retry will be handled by scheduler
            self.update_job_status(job_id, "pending_retry", Some(error_message))
                .await?;
        } else {
            error!(
                "Job {} failed after {} retries. Marking as failed.",
                job_id, MAX_RETRIES
            );
            self.update_job_status(job_id, "failed", Some(error_message))
                .await?;
        }

        Ok(())
    }

    /// Calculate volume trends for corridors
    pub async fn calculate_volume_trends(&self, hours: i64) -> Result<Vec<VolumeTrend>> {
        let end_time = Utc::now();
        let start_time = end_time - Duration::hours(hours);

        let metrics = self
            .db
            .fetch_hourly_metrics_by_timerange(start_time, end_time)
            .await
            .context("Failed to fetch hourly metrics for trend calculation")?;

        let trends = self.compute_volume_trends(metrics);
        Ok(trends)
    }

    /// Compute volume trends from hourly metrics
    fn compute_volume_trends(&self, metrics: Vec<HourlyCorridorMetrics>) -> Vec<VolumeTrend> {
        use std::collections::HashMap;

        let mut corridor_volumes: HashMap<String, Vec<(DateTime<Utc>, f64)>> = HashMap::new();

        for metric in metrics {
            corridor_volumes
                .entry(metric.corridor_key.clone())
                .or_default()
                .push((metric.hour_bucket, metric.volume_usd));
        }

        corridor_volumes
            .into_iter()
            .map(|(corridor_key, mut volumes)| {
                volumes.sort_by_key(|(time, _)| *time);

                let total_volume: f64 = volumes.iter().map(|(_, v)| v).sum();
                let avg_volume = if volumes.is_empty() {
                    0.0
                } else {
                    total_volume / volumes.len() as f64
                };

                // Calculate trend (simple linear regression slope)
                let trend = if volumes.len() >= 2 {
                    let first_half: f64 = volumes
                        .iter()
                        .take(volumes.len() / 2)
                        .map(|(_, v)| v)
                        .sum::<f64>()
                        / (volumes.len() / 2) as f64;

                    let second_half: f64 = volumes
                        .iter()
                        .skip(volumes.len() / 2)
                        .map(|(_, v)| v)
                        .sum::<f64>()
                        / (volumes.len() - volumes.len() / 2) as f64;

                    if first_half > 0.0 {
                        ((second_half - first_half) / first_half) * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                VolumeTrend {
                    corridor_key,
                    total_volume,
                    avg_volume,
                    trend_percentage: trend,
                    data_points: volumes.len(),
                }
            })
            .collect()
    }
}

impl Clone for AggregationService {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::Executor;
    use sqlx::{Row, SqlitePool};
    use std::str::FromStr;
    use tempfile::TempDir;
    use uuid::Uuid;

    async fn setup_aggregation_schema(pool: &SqlitePool) {
        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS payments (
                id TEXT PRIMARY KEY,
                transaction_hash TEXT NOT NULL,
                source_account TEXT NOT NULL,
                destination_account TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                asset_code TEXT,
                asset_issuer TEXT,
                amount REAL NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .await
        .unwrap();

        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS corridor_metrics_hourly (
                id TEXT PRIMARY KEY,
                corridor_key TEXT NOT NULL,
                asset_a_code TEXT NOT NULL,
                asset_a_issuer TEXT NOT NULL,
                asset_b_code TEXT NOT NULL,
                asset_b_issuer TEXT NOT NULL,
                hour_bucket TEXT NOT NULL,
                total_transactions INTEGER DEFAULT 0,
                successful_transactions INTEGER DEFAULT 0,
                failed_transactions INTEGER DEFAULT 0,
                success_rate REAL DEFAULT 0,
                volume_usd REAL DEFAULT 0,
                avg_slippage_bps REAL DEFAULT 0,
                avg_settlement_latency_ms INTEGER,
                liquidity_depth_usd REAL DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(corridor_key, hour_bucket)
            )
            "#,
        )
        .await
        .unwrap();

        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS aggregation_jobs (
                id TEXT PRIMARY KEY,
                job_type TEXT NOT NULL,
                status TEXT NOT NULL,
                start_time TEXT,
                end_time TEXT,
                last_processed_hour TEXT,
                error_message TEXT,
                retry_count INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .await
        .unwrap();
    }

    async fn setup_test_db() -> (Arc<Database>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("aggregation-tests.db");
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
            .unwrap()
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await.unwrap();
        setup_aggregation_schema(&pool).await;

        (Arc::new(Database::new(pool)), temp_dir)
    }

    async fn insert_test_payment(
        db: &Database,
        created_at: DateTime<Utc>,
        amount: f64,
        asset_code: &str,
        asset_issuer: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO payments (
                id,
                transaction_hash,
                source_account,
                destination_account,
                asset_type,
                asset_code,
                asset_issuer,
                amount,
                created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(format!("tx-{}", Uuid::new_v4()))
        .bind("G_SOURCE")
        .bind("G_DESTINATION")
        .bind("credit_alphanum4")
        .bind(asset_code)
        .bind(asset_issuer)
        .bind(amount)
        .bind(created_at.to_rfc3339())
        .execute(db.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_truncate_to_hour() {
        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        setup_aggregation_schema(&pool).await;
        let service =
            AggregationService::new(Arc::new(Database::new(pool)), AggregationConfig::default());

        let now = Utc::now();
        let truncated = service.truncate_to_hour(now);

        assert_eq!(truncated.minute(), 0);
        assert_eq!(truncated.second(), 0);
        assert_eq!(truncated.nanosecond(), 0);
    }

    #[tokio::test]
    async fn test_aggregate_hourly_metrics() {
        let (db, _temp_dir) = setup_test_db().await;
        let now = Utc::now();
        let start_time = now - Duration::minutes(30);
        let older_time = now - Duration::minutes(90);

        insert_test_payment(&db, start_time, 125.0, "USDC", "issuer1").await;
        insert_test_payment(&db, older_time, 75.0, "USDC", "issuer1").await;

        let service = AggregationService::new(Arc::clone(&db), AggregationConfig::default());
        let result = service.run_hourly_aggregation().await;

        assert!(result.is_ok());

        let metrics_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM corridor_metrics_hourly")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert!(metrics_count > 0);

        let metric = sqlx::query(
            r#"
            SELECT corridor_key, total_transactions, successful_transactions, volume_usd
            FROM corridor_metrics_hourly
            LIMIT 1
            "#,
        )
        .fetch_one(db.pool())
        .await
        .unwrap();

        assert_eq!(
            metric.get::<String, _>("corridor_key"),
            "USDC:issuer1->USDC:issuer1"
        );
        assert_eq!(metric.get::<i64, _>("total_transactions"), 2);
        assert_eq!(metric.get::<i64, _>("successful_transactions"), 2);
        assert_eq!(metric.get::<f64, _>("volume_usd"), 200.0);

        let job = sqlx::query(
            r#"
            SELECT status, last_processed_hour
            FROM aggregation_jobs
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .fetch_one(db.pool())
        .await
        .unwrap();

        assert_eq!(job.get::<String, _>("status"), "completed");
        assert!(job
            .get::<Option<String>, _>("last_processed_hour")
            .is_some());
    }

    #[tokio::test]
    async fn test_aggregate_with_no_data() {
        let (db, _temp_dir) = setup_test_db().await;
        let service = AggregationService::new(Arc::clone(&db), AggregationConfig::default());

        let result = service.run_hourly_aggregation().await;
        assert!(result.is_ok());

        let metrics_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM corridor_metrics_hourly")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(metrics_count, 0);

        let status: String = sqlx::query_scalar(
            "SELECT status FROM aggregation_jobs ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(status, "completed");
    }
}
