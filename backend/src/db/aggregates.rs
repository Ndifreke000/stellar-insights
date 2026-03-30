use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::models::corridor::{Corridor, CorridorAnalytics, CorridorMetrics};

pub struct CorridorAggregates {
    pool: SqlitePool,
}

impl CorridorAggregates {
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn store_daily_corridor_metrics(
        &self,
        analytics: &CorridorAnalytics,
        date: NaiveDate,
    ) -> Result<CorridorMetrics> {
        let date_datetime = start_of_day_utc(date)?;
        let corridor_key = analytics.corridor.to_string_key();

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r"
            INSERT INTO corridor_metrics (
                corridor_key, asset_a_code, asset_a_issuer, asset_b_code, asset_b_issuer,
                date, total_transactions, successful_transactions, failed_transactions,
                success_rate, volume_usd
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (corridor_key, date) DO UPDATE SET
                total_transactions = EXCLUDED.total_transactions,
                successful_transactions = EXCLUDED.successful_transactions,
                failed_transactions = EXCLUDED.failed_transactions,
                success_rate = EXCLUDED.success_rate,
                volume_usd = EXCLUDED.volume_usd,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            ",
        )
        .bind(&corridor_key)
        .bind(&analytics.corridor.source_asset_code)
        .bind(&analytics.corridor.source_asset_issuer)
        .bind(&analytics.corridor.destination_asset_code)
        .bind(&analytics.corridor.destination_asset_issuer)
        .bind(date_datetime)
        .bind(analytics.total_transactions)
        .bind(analytics.successful_transactions)
        .bind(analytics.failed_transactions)
        .bind(analytics.success_rate)
        .bind(analytics.volume_usd)
        .fetch_one(&self.pool)
        .await
        .context(format!(
            "Failed to store daily corridor metrics for corridor: {} on date: {}",
            corridor_key, date
        ))?;

        Ok(metrics)
    }

    pub async fn get_corridor_metrics(
        &self,
        corridor: &Corridor,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<CorridorMetrics>> {
        let corridor_key = corridor.to_string_key();
        let start_datetime = start_of_day_utc(start_date)?;
        let end_datetime = end_of_day_utc(end_date)?;

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r"
            SELECT * FROM corridor_metrics
            WHERE corridor_key = ? AND date >= ? AND date <= ?
            ORDER BY date DESC
            ",
        )
        .bind(&corridor_key)
        .bind(start_datetime)
        .bind(end_datetime)
        .fetch_all(&self.pool)
        .await
        .context(format!(
            "Failed to get corridor metrics for corridor: {} from {} to {}",
            corridor_key, start_date, end_date
        ))?;

        Ok(metrics)
    }

    pub async fn get_corridor_metrics_for_date(
        &self,
        date: NaiveDate,
    ) -> Result<Vec<CorridorMetrics>> {
        let date_datetime = start_of_day_utc(date)?;
        let next_day = date_datetime + chrono::Duration::days(1);

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r"
            SELECT * FROM corridor_metrics
            WHERE date >= ? AND date < ?
            ORDER BY volume_usd DESC
            ",
        )
        .bind(date_datetime)
        .bind(next_day)
        .fetch_all(&self.pool)
        .await
        .context(format!("Failed to get corridor metrics for date: {}", date))?;

        Ok(metrics)
    }

    pub async fn get_aggregated_corridor_metrics(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<AggregatedCorridorMetrics>> {
        let start_datetime = start_of_day_utc(start_date)?;
        let end_datetime = end_of_day_utc(end_date)?;

        let metrics = sqlx::query_as::<_, AggregatedCorridorMetrics>(
            r"
            SELECT
                corridor_key,
                asset_a_code,
                asset_a_issuer,
                asset_b_code,
                asset_b_issuer,
                SUM(total_transactions) as total_transactions,
                SUM(successful_transactions) as successful_transactions,
                SUM(failed_transactions) as failed_transactions,
                AVG(success_rate) as avg_success_rate,
                SUM(volume_usd) as total_volume_usd,
                MAX(date) as latest_date
            FROM corridor_metrics
            WHERE date >= ? AND date <= ?
            GROUP BY corridor_key, asset_a_code, asset_a_issuer, asset_b_code, asset_b_issuer
            ORDER BY total_volume_usd DESC
            ",
        )
        .bind(start_datetime)
        .bind(end_datetime)
        .fetch_all(&self.pool)
        .await
        .context(format!(
            "Failed to get aggregated corridor metrics from {} to {}",
            start_date, end_date
        ))?;

        Ok(metrics)
    }

    pub async fn get_top_corridors_by_volume(
        &self,
        date: NaiveDate,
        limit: i64,
    ) -> Result<Vec<CorridorMetrics>> {
        let date_datetime = start_of_day_utc(date)?;
        let next_day = date_datetime + chrono::Duration::days(1);

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r"
            SELECT * FROM corridor_metrics
            WHERE date >= ? AND date < ?
            ORDER BY volume_usd DESC
            LIMIT ?
            ",
        )
        .bind(date_datetime)
        .bind(next_day)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context(format!(
            "Failed to get top corridors by volume for date: {} (limit={})",
            date, limit
        ))?;

        Ok(metrics)
    }

    pub async fn get_top_corridors_by_transactions(
        &self,
        date: NaiveDate,
        limit: i64,
    ) -> Result<Vec<CorridorMetrics>> {
        let date_datetime = start_of_day_utc(date)?;
        let next_day = date_datetime + chrono::Duration::days(1);

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r"
            SELECT * FROM corridor_metrics
            WHERE date >= ? AND date < ?
            ORDER BY total_transactions DESC
            LIMIT ?
            ",
        )
        .bind(date_datetime)
        .bind(next_day)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context(format!(
            "Failed to get top corridors by transactions for date: {} (limit={})",
            date, limit
        ))?;

        Ok(metrics)
    }

    pub async fn get_corridors_by_success_rate(
        &self,
        date: NaiveDate,
        min_success_rate: f64,
        min_transactions: i64,
    ) -> Result<Vec<CorridorMetrics>> {
        let date_datetime = start_of_day_utc(date)?;
        let next_day = date_datetime + chrono::Duration::days(1);

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r"
            SELECT * FROM corridor_metrics
            WHERE date >= ? AND date < ?
            AND success_rate >= ?
            AND total_transactions >= ?
            ORDER BY success_rate DESC, total_transactions DESC
            ",
        )
        .bind(date_datetime)
        .bind(next_day)
        .bind(min_success_rate)
        .bind(min_transactions)
        .fetch_all(&self.pool)
        .await
        .context(format!(
            "Failed to get corridors by success rate for date: {} (min_rate={}, min_txs={})",
            date, min_success_rate, min_transactions
        ))?;

        Ok(metrics)
    }

    pub async fn get_corridor_summary_stats(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<CorridorSummaryStats> {
        let start_datetime = start_of_day_utc(start_date)?;
        let end_datetime = end_of_day_utc(end_date)?;

        let stats = sqlx::query_as::<_, CorridorSummaryStats>(
            r"
            SELECT
                COUNT(*) as total_corridors,
                SUM(total_transactions) as total_transactions,
                SUM(successful_transactions) as successful_transactions,
                SUM(failed_transactions) as failed_transactions,
                SUM(volume_usd) as total_volume_usd,
                AVG(success_rate) as avg_success_rate
            FROM corridor_metrics
            WHERE date >= ? AND date <= ?
            ",
        )
        .bind(start_datetime)
        .bind(end_datetime)
        .fetch_one(&self.pool)
        .await
        .context(format!(
            "Failed to get corridor summary stats from {} to {}",
            start_date, end_date
        ))?;

        Ok(stats)
    }

    pub async fn delete_old_metrics(&self, cutoff_date: NaiveDate) -> Result<u64> {
        let cutoff_datetime = start_of_day_utc(cutoff_date)?;

        let result: sqlx::sqlite::SqliteQueryResult = sqlx::query(
            r"
            DELETE FROM corridor_metrics
            WHERE date < ?
            ",
        )
        .bind(cutoff_datetime)
        .execute(&self.pool)
        .await
        .context(format!(
            "Failed to delete metrics older than: {}",
            cutoff_date
        ))?;

        Ok(result.rows_affected())
    }
}

fn start_of_day_utc(date: NaiveDate) -> Result<chrono::DateTime<chrono::Utc>> {
    date.and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("Invalid date at start of day: {date}"))
        .map(|dt| dt.and_utc())
}

fn end_of_day_utc(date: NaiveDate) -> Result<chrono::DateTime<chrono::Utc>> {
    date.and_hms_opt(23, 59, 59)
        .ok_or_else(|| anyhow!("Invalid date at end of day: {date}"))
        .map(|dt| dt.and_utc())
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AggregatedCorridorMetrics {
    pub corridor_key: String,
    #[sqlx(rename = "asset_a_code")]
    pub source_asset_code: String,
    #[sqlx(rename = "asset_a_issuer")]
    pub source_asset_issuer: String,
    #[sqlx(rename = "asset_b_code")]
    pub destination_asset_code: String,
    #[sqlx(rename = "asset_b_issuer")]
    pub destination_asset_issuer: String,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_success_rate: f64,
    pub total_volume_usd: f64,
    pub latest_date: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CorridorSummaryStats {
    pub total_corridors: i64,
    pub total_transactions: Option<i64>,
    pub successful_transactions: Option<i64>,
    pub failed_transactions: Option<i64>,
    pub total_volume_usd: Option<f64>,
    pub avg_success_rate: Option<f64>,
}

#[cfg(test)]
mod tests {}
