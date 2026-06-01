use anyhow::Result;
use chrono::NaiveDate;
use sqlx::SqlitePool;

use crate::models::corridor::{Corridor, CorridorAnalytics, CorridorMetrics};

pub struct CorridorAggregates {
    pool: SqlitePool,
}

impl CorridorAggregates {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn store_daily_corridor_metrics(
        &self,
        analytics: &CorridorAnalytics,
        date: NaiveDate,
    ) -> Result<CorridorMetrics> {
        let date_datetime = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let corridor_key = analytics.corridor.to_string_key();

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r#"
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
            "#,
        )
        .bind(&corridor_key)
        .bind(&analytics.corridor.asset_a_code)
        .bind(&analytics.corridor.asset_a_issuer)
        .bind(&analytics.corridor.asset_b_code)
        .bind(&analytics.corridor.asset_b_issuer)
        .bind(date_datetime)
        .bind(analytics.total_transactions)
        .bind(analytics.successful_transactions)
        .bind(analytics.failed_transactions)
        .bind(analytics.success_rate)
        .bind(analytics.volume_usd)
        .fetch_one(&self.pool)
        .await?;

        Ok(metrics)
    }

    pub async fn get_corridor_metrics(
        &self,
        corridor: &Corridor,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<CorridorMetrics>> {
        let corridor_key = corridor.to_string_key();
        let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap().and_utc();

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r#"
            SELECT * FROM corridor_metrics
            WHERE corridor_key = ? AND date >= ? AND date <= ?
            ORDER BY date DESC
            "#,
        )
        .bind(&corridor_key)
        .bind(start_datetime)
        .bind(end_datetime)
        .fetch_all(&self.pool)
        .await?;

        Ok(metrics)
    }

    pub async fn get_corridor_metrics_for_date(
        &self,
        date: NaiveDate,
    ) -> Result<Vec<CorridorMetrics>> {
        let date_datetime = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let next_day = date_datetime + chrono::Duration::days(1);

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r#"
            SELECT * FROM corridor_metrics
            WHERE date >= ? AND date < ?
            ORDER BY volume_usd DESC
            "#,
        )
        .bind(date_datetime)
        .bind(next_day)
        .fetch_all(&self.pool)
        .await?;

        Ok(metrics)
    }

    pub async fn get_aggregated_corridor_metrics(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<AggregatedCorridorMetrics>> {
        let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap().and_utc();

        let metrics = sqlx::query_as::<_, AggregatedCorridorMetrics>(
            r#"
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
            "#,
        )
        .bind(start_datetime)
        .bind(end_datetime)
        .fetch_all(&self.pool)
        .await?;

        Ok(metrics)
    }

    pub async fn get_top_corridors_by_volume(
        &self,
        date: NaiveDate,
        limit: i64,
    ) -> Result<Vec<CorridorMetrics>> {
        let date_datetime = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let next_day = date_datetime + chrono::Duration::days(1);

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r#"
            SELECT * FROM corridor_metrics
            WHERE date >= ? AND date < ?
            ORDER BY volume_usd DESC
            LIMIT ?
            "#,
        )
        .bind(date_datetime)
        .bind(next_day)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(metrics)
    }

    pub async fn get_top_corridors_by_transactions(
        &self,
        date: NaiveDate,
        limit: i64,
    ) -> Result<Vec<CorridorMetrics>> {
        let date_datetime = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let next_day = date_datetime + chrono::Duration::days(1);

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r#"
            SELECT * FROM corridor_metrics
            WHERE date >= ? AND date < ?
            ORDER BY total_transactions DESC
            LIMIT ?
            "#,
        )
        .bind(date_datetime)
        .bind(next_day)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(metrics)
    }

    pub async fn get_corridors_by_success_rate(
        &self,
        date: NaiveDate,
        min_success_rate: f64,
        min_transactions: i64,
    ) -> Result<Vec<CorridorMetrics>> {
        let date_datetime = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let next_day = date_datetime + chrono::Duration::days(1);

        let metrics = sqlx::query_as::<_, CorridorMetrics>(
            r#"
            SELECT * FROM corridor_metrics
            WHERE date >= ? AND date < ?
            AND success_rate >= ?
            AND total_transactions >= ?
            ORDER BY success_rate DESC, total_transactions DESC
            "#,
        )
        .bind(date_datetime)
        .bind(next_day)
        .bind(min_success_rate)
        .bind(min_transactions)
        .fetch_all(&self.pool)
        .await?;

        Ok(metrics)
    }

    pub async fn get_corridor_summary_stats(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<CorridorSummaryStats> {
        let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap().and_utc();

        let stats = sqlx::query_as::<_, CorridorSummaryStats>(
            r#"
            SELECT 
                COUNT(*) as total_corridors,
                SUM(total_transactions) as total_transactions,
                SUM(successful_transactions) as successful_transactions,
                SUM(failed_transactions) as failed_transactions,
                SUM(volume_usd) as total_volume_usd,
                AVG(success_rate) as avg_success_rate
            FROM corridor_metrics
            WHERE date >= ? AND date <= ?
            "#,
        )
        .bind(start_datetime)
        .bind(end_datetime)
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }

    pub async fn delete_old_metrics(&self, cutoff_date: NaiveDate) -> Result<u64> {
        let cutoff_datetime = cutoff_date.and_hms_opt(0, 0, 0).unwrap().and_utc();

        let result: sqlx::sqlite::SqliteQueryResult = sqlx::query(
            r#"
            DELETE FROM corridor_metrics
            WHERE date < ?
            "#,
        )
        .bind(cutoff_datetime)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AggregatedCorridorMetrics {
    pub corridor_key: String,
    pub asset_a_code: String,
    pub asset_a_issuer: String,
    pub asset_b_code: String,
    pub asset_b_issuer: String,
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
