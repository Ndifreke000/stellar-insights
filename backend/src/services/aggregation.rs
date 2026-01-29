use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Timelike, Utc};
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::database::Database;
use crate::models::corridor::CorridorMetrics;
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
            interval_hours: 1,  // Run every hour
            lookback_hours: 2,  // Process last 2 hours of data
            batch_size: 10000,  // Process 10k payments at a time
        }
    }
}

pub struct AggregationService {
    db: Arc<Database>,
    config: AggregationConfig,
}

impl AggregationService {
    pub fn new(db: Arc<Database>, config: AggregationConfig) -> Self {
        Self { db, config }
    }

    /// Start the hourly aggregation job scheduler
    pub async fn start_scheduler(self: Arc<Self>) {
        info!(
            "Starting corridor aggregation scheduler (interval: {} hours)",
            self.config.interval_hours
        );

        let mut ticker = interval(TokioDuration::from_secs(
            self.config.interval_hours * 3600,
        ));

        loop {
            ticker.tick().await;
            
            info!("Triggering hourly corridor aggregation");
            
            // Check for pending retries first
            if let Err(e) = self.process_pending_retries().await {
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
    async fn process_pending_retries(&self) -> Result<()> {
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
                info!("Aggregation completed successfully. Processed {} corridor metrics", metrics_count);
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
        _start_time: DateTime<Utc>,
    ) -> Vec<HourlyCorridorMetrics> {
        use std::collections::HashMap;

        let mut hourly_map: HashMap<(String, String), HourlyCorridorMetrics> = HashMap::new();

        for metric in metrics {
            let hour_bucket = self.truncate_to_hour(metric.date);
            let key = (metric.corridor_key.clone(), hour_bucket.to_rfc3339());

            hourly_map
                .entry(key)
                .and_modify(|existing| {
                    existing.total_transactions += metric.total_transactions;
                    existing.successful_transactions += metric.successful_transactions;
                    existing.failed_transactions += metric.failed_transactions;
                    existing.volume_usd += metric.volume_usd;
                    
                    // Update averages (weighted by transaction count)
                    if let Some(latency) = metric.avg_settlement_latency_ms {
                        let total_latency = existing.avg_settlement_latency_ms.unwrap_or(0) as i64
                            * existing.total_transactions
                            + latency as i64 * metric.total_transactions;
                        existing.avg_settlement_latency_ms = Some(
                            (total_latency / (existing.total_transactions + metric.total_transactions)) as i32
                        );
                    }
                    
                    existing.liquidity_depth_usd = 
                        (existing.liquidity_depth_usd + metric.liquidity_depth_usd) / 2.0;
                })
                .or_insert_with(|| HourlyCorridorMetrics {
                    id: Uuid::new_v4().to_string(),
                    corridor_key: metric.corridor_key.clone(),
                    asset_a_code: metric.asset_a_code.clone(),
                    asset_a_issuer: metric.asset_a_issuer.clone(),
                    asset_b_code: metric.asset_b_code.clone(),
                    asset_b_issuer: metric.asset_b_issuer.clone(),
                    hour_bucket,
                    total_transactions: metric.total_transactions,
                    successful_transactions: metric.successful_transactions,
                    failed_transactions: metric.failed_transactions,
                    success_rate: metric.success_rate,
                    volume_usd: metric.volume_usd,
                    avg_slippage_bps: 0.0, // TODO: Calculate from order book data
                    avg_settlement_latency_ms: metric.avg_settlement_latency_ms,
                    liquidity_depth_usd: metric.liquidity_depth_usd,
                });
        }

        // Recalculate success rates
        hourly_map
            .into_values()
            .map(|mut m| {
                if m.total_transactions > 0 {
                    m.success_rate = (m.successful_transactions as f64 / m.total_transactions as f64) * 100.0;
                }
                m
            })
            .collect()
    }

    /// Store hourly metrics in the database
    async fn store_hourly_metrics(&self, metrics: Vec<HourlyCorridorMetrics>) -> Result<usize> {
        let count = metrics.len();
        
        for metric in metrics {
            self.db
                .upsert_hourly_corridor_metric(&metric)
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
                let avg_volume = if !volumes.is_empty() {
                    total_volume / volumes.len() as f64
                } else {
                    0.0
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

#[derive(Debug, Clone)]
pub struct HourlyCorridorMetrics {
    pub id: String,
    pub corridor_key: String,
    pub asset_a_code: String,
    pub asset_a_issuer: String,
    pub asset_b_code: String,
    pub asset_b_issuer: String,
    pub hour_bucket: DateTime<Utc>,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub success_rate: f64,
    pub volume_usd: f64,
    pub avg_slippage_bps: f64,
    pub avg_settlement_latency_ms: Option<i32>,
    pub liquidity_depth_usd: f64,
}

#[derive(Debug, Clone)]
pub struct VolumeTrend {
    pub corridor_key: String,
    pub total_volume: f64,
    pub avg_volume: f64,
    pub trend_percentage: f64,
    pub data_points: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_to_hour() {
        let service = AggregationService::new(
            Arc::new(Database::new_mock()),
            AggregationConfig::default(),
        );

        let dt = Utc::now();
        let truncated = service.truncate_to_hour(dt);

        assert_eq!(truncated.minute(), 0);
        assert_eq!(truncated.second(), 0);
        assert_eq!(truncated.nanosecond(), 0);
    }

    #[test]
    fn test_compute_volume_trends() {
        let service = AggregationService::new(
            Arc::new(Database::new_mock()),
            AggregationConfig::default(),
        );

        let now = Utc::now();
        let metrics = vec![
            HourlyCorridorMetrics {
                id: "1".to_string(),
                corridor_key: "USDC:issuer1->EURC:issuer2".to_string(),
                asset_a_code: "USDC".to_string(),
                asset_a_issuer: "issuer1".to_string(),
                asset_b_code: "EURC".to_string(),
                asset_b_issuer: "issuer2".to_string(),
                hour_bucket: now - Duration::hours(2),
                total_transactions: 100,
                successful_transactions: 95,
                failed_transactions: 5,
                success_rate: 95.0,
                volume_usd: 1000.0,
                avg_slippage_bps: 10.0,
                avg_settlement_latency_ms: Some(500),
                liquidity_depth_usd: 50000.0,
            },
            HourlyCorridorMetrics {
                id: "2".to_string(),
                corridor_key: "USDC:issuer1->EURC:issuer2".to_string(),
                asset_a_code: "USDC".to_string(),
                asset_a_issuer: "issuer1".to_string(),
                asset_b_code: "EURC".to_string(),
                asset_b_issuer: "issuer2".to_string(),
                hour_bucket: now - Duration::hours(1),
                total_transactions: 150,
                successful_transactions: 145,
                failed_transactions: 5,
                success_rate: 96.7,
                volume_usd: 1500.0,
                avg_slippage_bps: 12.0,
                avg_settlement_latency_ms: Some(450),
                liquidity_depth_usd: 55000.0,
            },
        ];

        let trends = service.compute_volume_trends(metrics);
        assert_eq!(trends.len(), 1);
        assert_eq!(trends[0].corridor_key, "USDC:issuer1->EURC:issuer2");
        assert_eq!(trends[0].total_volume, 2500.0);
        assert_eq!(trends[0].data_points, 2);
    }
}
