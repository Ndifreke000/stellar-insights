use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::analytics::compute_anchor_metrics;
use crate::models::{
    Anchor, AnchorDetailResponse, AnchorMetricsHistory, CreateAnchorRequest,
};

pub struct AnchorRpcUpdate {
    pub stellar_account: String,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub total_volume_usd: f64,
    pub avg_settlement_time_ms: i32,
    pub reliability_score: f64,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct AnchorMetricsUpdate {
    pub anchor_id: Uuid,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_time_ms: Option<i32>,
    pub volume_usd: Option<f64>,
}

pub struct AnchorMetricsParams {
    pub anchor_id: Uuid,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub reliability_score: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_time_ms: Option<i32>,
    pub volume_usd: Option<f64>,
}

pub struct AnchorDb {
    pool: SqlitePool,
}

impl AnchorDb {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, req: CreateAnchorRequest) -> Result<Anchor> {
        let id = Uuid::new_v4().to_string();
        sqlx::query_as::<_, Anchor>(
            "INSERT INTO anchors (id, name, stellar_account, home_domain)
             VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.stellar_account)
        .bind(&req.home_domain)
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to create anchor: {}", req.name))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Anchor>> {
        sqlx::query_as::<_, Anchor>("SELECT * FROM anchors WHERE id = $1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .with_context(|| format!("Failed to fetch anchor: {id}"))
    }

    pub async fn get_by_stellar_account(&self, stellar_account: &str) -> Result<Option<Anchor>> {
        sqlx::query_as::<_, Anchor>("SELECT * FROM anchors WHERE stellar_account = $1")
            .bind(stellar_account)
            .fetch_optional(&self.pool)
            .await
            .with_context(|| format!("Failed to fetch anchor by account: {stellar_account}"))
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Anchor>> {
        sqlx::query_as::<_, Anchor>(
            "SELECT * FROM anchors ORDER BY reliability_score DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to list anchors (limit={limit}, offset={offset})"))
    }

    pub async fn get_all(&self) -> Result<Vec<Anchor>> {
        sqlx::query_as::<_, Anchor>("SELECT * FROM anchors ORDER BY name ASC")
            .fetch_all(&self.pool)
            .await
            .context("Failed to get all anchors")
    }

    pub async fn update_metrics(&self, update: AnchorMetricsUpdate) -> Result<Anchor> {
        let metrics = compute_anchor_metrics(
            update.total_transactions,
            update.successful_transactions,
            update.failed_transactions,
            update.avg_settlement_time_ms,
        );

        let mut tx = self.pool.begin().await.with_context(|| {
            format!("Failed to begin transaction for anchor: {}", update.anchor_id)
        })?;

        let anchor = sqlx::query_as::<_, Anchor>(
            "UPDATE anchors
             SET total_transactions = $1, successful_transactions = $2, failed_transactions = $3,
                 avg_settlement_time_ms = $4, reliability_score = $5, status = $6,
                 total_volume_usd = COALESCE($7, total_volume_usd), updated_at = $8
             WHERE id = $9 RETURNING *",
        )
        .bind(update.total_transactions)
        .bind(update.successful_transactions)
        .bind(update.failed_transactions)
        .bind(update.avg_settlement_time_ms.unwrap_or(0))
        .bind(metrics.reliability_score)
        .bind(metrics.status.as_str())
        .bind(update.volume_usd)
        .bind(Utc::now())
        .bind(update.anchor_id.to_string())
        .fetch_one(&mut *tx)
        .await
        .with_context(|| format!("Failed to update anchor metrics: {}", update.anchor_id))?;

        sqlx::query(
            "INSERT INTO anchor_metrics_history (
                id, anchor_id, timestamp, success_rate, failure_rate, reliability_score,
                total_transactions, successful_transactions, failed_transactions,
                avg_settlement_time_ms, volume_usd
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(update.anchor_id.to_string())
        .bind(Utc::now())
        .bind(metrics.success_rate)
        .bind(metrics.failure_rate)
        .bind(metrics.reliability_score)
        .bind(update.total_transactions)
        .bind(update.successful_transactions)
        .bind(update.failed_transactions)
        .bind(update.avg_settlement_time_ms.unwrap_or(0))
        .bind(update.volume_usd.unwrap_or(0.0))
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!("Failed to record metrics history: {}", update.anchor_id)
        })?;

        tx.commit().await.with_context(|| {
            format!("Failed to commit anchor update: {}", update.anchor_id)
        })?;

        Ok(anchor)
    }

    pub async fn update_from_rpc(&self, params: AnchorRpcUpdate) -> Result<()> {
        sqlx::query(
            "UPDATE anchors
             SET total_transactions = $1, successful_transactions = $2, failed_transactions = $3,
                 total_volume_usd = $4, avg_settlement_time_ms = $5, reliability_score = $6,
                 status = $7, updated_at = $8
             WHERE stellar_account = $9",
        )
        .bind(params.total_transactions)
        .bind(params.successful_transactions)
        .bind(params.failed_transactions)
        .bind(params.total_volume_usd)
        .bind(params.avg_settlement_time_ms)
        .bind(params.reliability_score)
        .bind(&params.status)
        .bind(Utc::now())
        .bind(&params.stellar_account)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to update anchor from RPC: {}", params.stellar_account))?;
        Ok(())
    }

    pub async fn record_metrics_history(
        &self,
        params: AnchorMetricsParams,
    ) -> Result<AnchorMetricsHistory> {
        sqlx::query_as::<_, AnchorMetricsHistory>(
            "INSERT INTO anchor_metrics_history (
                id, anchor_id, timestamp, success_rate, failure_rate, reliability_score,
                total_transactions, successful_transactions, failed_transactions,
                avg_settlement_time_ms, volume_usd
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING *",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(params.anchor_id.to_string())
        .bind(Utc::now())
        .bind(params.success_rate)
        .bind(params.failure_rate)
        .bind(params.reliability_score)
        .bind(params.total_transactions)
        .bind(params.successful_transactions)
        .bind(params.failed_transactions)
        .bind(params.avg_settlement_time_ms.unwrap_or(0))
        .bind(params.volume_usd.unwrap_or(0.0))
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to record metrics history: {}", params.anchor_id))
    }

    pub async fn get_metrics_history(
        &self,
        anchor_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AnchorMetricsHistory>> {
        sqlx::query_as::<_, AnchorMetricsHistory>(
            "SELECT * FROM anchor_metrics_history WHERE anchor_id = $1
             ORDER BY timestamp DESC LIMIT $2",
        )
        .bind(anchor_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to get metrics history: {anchor_id}"))
    }

    pub async fn get_detail(&self, anchor_id: Uuid) -> Result<Option<AnchorDetailResponse>> {
        let anchor = match self.get_by_id(anchor_id).await? {
            Some(a) => a,
            None => return Ok(None),
        };

        let assets_db = crate::db::assets::AssetDb::new(self.pool.clone());
        let assets = assets_db.get_by_anchor(anchor_id).await?;
        let metrics_history = self.get_metrics_history(anchor_id, 30).await?;

        Ok(Some(AnchorDetailResponse {
            anchor,
            assets,
            metrics_history,
        }))
    }

    pub async fn get_recent_performance(
        &self,
        anchor_id: &str,
        minutes: i64,
    ) -> Result<crate::models::AnchorMetrics> {
        let start_time = Utc::now() - chrono::Duration::minutes(minutes);

        let row: (i64, i64, Option<f64>) = sqlx::query_as(
            "SELECT COUNT(*) as total, COUNT(*) as successful, AVG(amount) as avg_latency
             FROM payments
             WHERE (source_account = $1 OR destination_account = $2) AND created_at >= $3",
        )
        .bind(anchor_id)
        .bind(anchor_id)
        .bind(start_time.to_rfc3339())
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to get recent performance: {anchor_id}"))?;

        let total = row.0;
        let successful = row.1;
        let failed = total - successful;
        let success_rate = if total > 0 {
            (successful as f64 / total as f64) * 100.0
        } else {
            100.0
        };
        let failure_rate = 100.0 - success_rate;
        let status = crate::models::AnchorStatus::from_metrics(success_rate, failure_rate);

        Ok(crate::models::AnchorMetrics {
            success_rate,
            failure_rate,
            reliability_score: success_rate,
            total_transactions: total,
            successful_transactions: successful,
            failed_transactions: failed,
            avg_settlement_time_ms: row.2.map(|l| l as i32),
            status,
        })
    }
}
