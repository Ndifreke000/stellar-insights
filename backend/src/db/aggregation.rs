use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::services::aggregation::HourlyCorridorMetrics;

pub struct AggregationDb {
    pool: SqlitePool,
}

impl AggregationDb {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Fetch payments within a time range
    pub async fn fetch_payments_by_timerange(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<crate::models::corridor::PaymentRecord>> {
        let records = sqlx::query_as::<_, PaymentRecordRow>(
            r#"
            SELECT 
                id,
                transaction_hash,
                source_account,
                destination_account,
                asset_type,
                asset_code,
                asset_issuer,
                amount,
                created_at
            FROM payments
            WHERE created_at >= ? AND created_at <= ?
            ORDER BY created_at ASC
            LIMIT ?
            "#,
        )
        .bind(start_time.to_rfc3339())
        .bind(end_time.to_rfc3339())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch payments by timerange")?;

        // Convert to PaymentRecord with corridor information
        let payment_records: Vec<crate::models::corridor::PaymentRecord> = records
            .into_iter()
            .filter_map(|row| {
                // Parse the created_at timestamp
                let timestamp = DateTime::parse_from_rfc3339(&row.created_at)
                    .ok()?
                    .with_timezone(&Utc);

                // For now, assume all payments are successful
                // In a real system, you'd have a status field
                let successful = true;

                Some(crate::models::corridor::PaymentRecord {
                    id: uuid::Uuid::parse_str(&row.id).ok()?,
                    source_asset_code: row.asset_code.clone().unwrap_or_else(|| "XLM".to_string()),
                    source_asset_issuer: row.asset_issuer.clone().unwrap_or_else(|| "native".to_string()),
                    destination_asset_code: row.asset_code.unwrap_or_else(|| "XLM".to_string()),
                    destination_asset_issuer: row.asset_issuer.unwrap_or_else(|| "native".to_string()),
                    amount: row.amount,
                    successful,
                    timestamp,
                })
            })
            .collect();

        Ok(payment_records)
    }

    /// Upsert hourly corridor metric
    pub async fn upsert_hourly_corridor_metric(&self, metric: &HourlyCorridorMetrics) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"
            INSERT INTO corridor_metrics_hourly (
                id,
                corridor_key,
                asset_a_code,
                asset_a_issuer,
                asset_b_code,
                asset_b_issuer,
                hour_bucket,
                total_transactions,
                successful_transactions,
                failed_transactions,
                success_rate,
                volume_usd,
                avg_slippage_bps,
                avg_settlement_latency_ms,
                liquidity_depth_usd,
                created_at,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(corridor_key, hour_bucket) DO UPDATE SET
                total_transactions = total_transactions + excluded.total_transactions,
                successful_transactions = successful_transactions + excluded.successful_transactions,
                failed_transactions = failed_transactions + excluded.failed_transactions,
                success_rate = (successful_transactions * 100.0) / NULLIF(total_transactions, 0),
                volume_usd = volume_usd + excluded.volume_usd,
                avg_slippage_bps = (avg_slippage_bps + excluded.avg_slippage_bps) / 2.0,
                avg_settlement_latency_ms = COALESCE(
                    (avg_settlement_latency_ms + excluded.avg_settlement_latency_ms) / 2,
                    avg_settlement_latency_ms,
                    excluded.avg_settlement_latency_ms
                ),
                liquidity_depth_usd = (liquidity_depth_usd + excluded.liquidity_depth_usd) / 2.0,
                updated_at = ?
            "#,
        )
        .bind(&metric.id)
        .bind(&metric.corridor_key)
        .bind(&metric.asset_a_code)
        .bind(&metric.asset_a_issuer)
        .bind(&metric.asset_b_code)
        .bind(&metric.asset_b_issuer)
        .bind(metric.hour_bucket.to_rfc3339())
        .bind(metric.total_transactions)
        .bind(metric.successful_transactions)
        .bind(metric.failed_transactions)
        .bind(metric.success_rate)
        .bind(metric.volume_usd)
        .bind(metric.avg_slippage_bps)
        .bind(metric.avg_settlement_latency_ms)
        .bind(metric.liquidity_depth_usd)
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .context("Failed to upsert hourly corridor metric")?;

        Ok(())
    }

    /// Fetch hourly metrics by time range
    pub async fn fetch_hourly_metrics_by_timerange(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<HourlyCorridorMetrics>> {
        let rows = sqlx::query_as::<_, HourlyCorridorMetricsRow>(
            r#"
            SELECT 
                id,
                corridor_key,
                asset_a_code,
                asset_a_issuer,
                asset_b_code,
                asset_b_issuer,
                hour_bucket,
                total_transactions,
                successful_transactions,
                failed_transactions,
                success_rate,
                volume_usd,
                avg_slippage_bps,
                avg_settlement_latency_ms,
                liquidity_depth_usd
            FROM corridor_metrics_hourly
            WHERE hour_bucket >= ? AND hour_bucket <= ?
            ORDER BY hour_bucket ASC
            "#,
        )
        .bind(start_time.to_rfc3339())
        .bind(end_time.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch hourly metrics by timerange")?;

        let metrics: Vec<HourlyCorridorMetrics> = rows
            .into_iter()
            .filter_map(|row| {
                let hour_bucket = DateTime::parse_from_rfc3339(&row.hour_bucket)
                    .ok()?
                    .with_timezone(&Utc);

                Some(HourlyCorridorMetrics {
                    id: row.id,
                    corridor_key: row.corridor_key,
                    asset_a_code: row.asset_a_code,
                    asset_a_issuer: row.asset_a_issuer,
                    asset_b_code: row.asset_b_code,
                    asset_b_issuer: row.asset_b_issuer,
                    hour_bucket,
                    total_transactions: row.total_transactions,
                    successful_transactions: row.successful_transactions,
                    failed_transactions: row.failed_transactions,
                    success_rate: row.success_rate,
                    volume_usd: row.volume_usd,
                    avg_slippage_bps: row.avg_slippage_bps,
                    avg_settlement_latency_ms: row.avg_settlement_latency_ms,
                    liquidity_depth_usd: row.liquidity_depth_usd,
                })
            })
            .collect();

        Ok(metrics)
    }

    /// Create aggregation job record
    pub async fn create_aggregation_job(&self, job_id: &str, job_type: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"
            INSERT INTO aggregation_jobs (id, job_type, status, created_at, updated_at)
            VALUES (?, ?, 'pending', ?, ?)
            "#,
        )
        .bind(job_id)
        .bind(job_type)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .context("Failed to create aggregation job")?;

        Ok(())
    }

    /// Update aggregation job status
    pub async fn update_aggregation_job_status(
        &self,
        job_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        
        let time_field = match status {
            "running" => "start_time",
            "completed" | "failed" => "end_time",
            _ => "updated_at",
        };

        let query_str = format!(
            r#"
            UPDATE aggregation_jobs
            SET status = ?, error_message = ?, {} = ?, updated_at = ?
            WHERE id = ?
            "#,
            time_field
        );

        sqlx::query(&query_str)
            .bind(status)
            .bind(error_message)
            .bind(&now)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .context("Failed to update aggregation job status")?;

        Ok(())
    }

    /// Update last processed hour for a job
    pub async fn update_last_processed_hour(&self, job_id: &str, last_hour: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"
            UPDATE aggregation_jobs
            SET last_processed_hour = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(last_hour)
        .bind(&now)
        .bind(job_id)
        .execute(&self.pool)
        .await
        .context("Failed to update last processed hour")?;

        Ok(())
    }

    /// Get job retry count
    pub async fn get_job_retry_count(&self, job_id: &str) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            r#"
            SELECT retry_count FROM aggregation_jobs WHERE id = ?
            "#,
        )
        .bind(job_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to get job retry count")?;

        Ok(row.0)
    }

    /// Increment job retry count
    pub async fn increment_job_retry_count(&self, job_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"
            UPDATE aggregation_jobs
            SET retry_count = retry_count + 1, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(job_id)
        .execute(&self.pool)
        .await
        .context("Failed to increment job retry count")?;

        Ok(())
    }
}

// Database row structures
#[derive(sqlx::FromRow)]
struct PaymentRecordRow {
    id: String,
    transaction_hash: String,
    source_account: String,
    destination_account: String,
    asset_type: String,
    asset_code: Option<String>,
    asset_issuer: Option<String>,
    amount: f64,
    created_at: String,
}

#[derive(sqlx::FromRow)]
struct HourlyCorridorMetricsRow {
    id: String,
    corridor_key: String,
    asset_a_code: String,
    asset_a_issuer: String,
    asset_b_code: String,
    asset_b_issuer: String,
    hour_bucket: String,
    total_transactions: i64,
    successful_transactions: i64,
    failed_transactions: i64,
    success_rate: f64,
    volume_usd: f64,
    avg_slippage_bps: f64,
    avg_settlement_latency_ms: Option<i32>,
    liquidity_depth_usd: f64,
}
