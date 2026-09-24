use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{
    Anchor, AnchorDetailResponse, AnchorMetricsHistory, Asset, CreateAnchorRequest,
};

/// Parameters for updating anchor from RPC data
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

/// Parameters for updating anchor metrics
#[derive(Debug, Clone)]
pub struct AnchorMetricsUpdate {
    pub anchor_id: Uuid,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_time_ms: Option<i32>,
    pub volume_usd: Option<f64>,
    /// Pre-computed reliability score (0–100). Callers must compute this via
    /// `analytics::compute_anchor_metrics` before calling `update_anchor_metrics`.
    pub reliability_score: f64,
    /// Pre-computed success rate (0–100).
    pub success_rate: f64,
    /// Pre-computed failure rate (0–100).
    pub failure_rate: f64,
    /// Pre-computed anchor status string ("green" | "yellow" | "red").
    pub status: String,
}

/// Parameters for recording anchor metrics history
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
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates a new anchor in the database.
    #[tracing::instrument(skip(self, req), fields(anchor_name = %req.name, stellar_account = %req.stellar_account))]
    pub async fn create_anchor(&self, req: CreateAnchorRequest) -> Result<Anchor> {
        let id = Uuid::new_v4().to_string();
        let anchor = sqlx::query_as::<_, Anchor>(
            r"
            INSERT INTO anchors (id, name, stellar_account, home_domain)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.stellar_account)
        .bind(&req.home_domain)
        .fetch_one(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to create anchor: name={}, stellar_account={}",
                req.name, req.stellar_account
            )
        })?;
        Ok(anchor)
    }

    /// Retrieves an anchor by its unique identifier.
    #[tracing::instrument(skip(self), fields(anchor_id = %id))]
    pub async fn get_anchor_by_id(&self, id: Uuid) -> Result<Option<Anchor>> {
        let anchor = sqlx::query_as::<_, Anchor>(
            r"
            SELECT * FROM anchors WHERE id = $1
            ",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch anchor with id: {}", id))?;
        Ok(anchor)
    }

    /// Retrieves an anchor by its Stellar account address.
    #[tracing::instrument(skip(self), fields(stellar_account = %stellar_account))]
    pub async fn get_anchor_by_stellar_account(
        &self,
        stellar_account: &str,
    ) -> Result<Option<Anchor>> {
        let anchor = sqlx::query_as::<_, Anchor>(
            r"
            SELECT * FROM anchors WHERE stellar_account = $1
            ",
        )
        .bind(stellar_account)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to fetch anchor by stellar_account: {}",
                stellar_account
            )
        })?;
        Ok(anchor)
    }

    /// Lists all anchors with pagination, sorted by reliability score.
    #[tracing::instrument(skip(self), fields(limit = limit, offset = offset))]
    pub async fn list_anchors(&self, limit: i64, offset: i64) -> Result<Vec<Anchor>> {
        let anchors = sqlx::query_as::<_, Anchor>(
            r"
            SELECT * FROM anchors
            ORDER BY reliability_score DESC
            LIMIT $1 OFFSET $2
            ",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to list anchors (limit={}, offset={})",
                limit, offset
            )
        })?;
        Ok(anchors)
    }

    /// Retrieves all anchors from the database.
    #[tracing::instrument(skip(self))]
    pub async fn get_all_anchors(&self) -> Result<Vec<Anchor>> {
        let anchors = sqlx::query_as::<_, Anchor>("SELECT * FROM anchors ORDER BY name ASC")
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch all anchors from database")?;
        Ok(anchors)
    }

    /// Updates anchor metrics and records history in a single transaction.
    #[tracing::instrument(skip(self, update), fields(anchor_id = %update.anchor_id))]
    pub async fn update_anchor_metrics(&self, update: AnchorMetricsUpdate) -> Result<Anchor> {
        let mut tx = self.pool.begin().await.with_context(|| {
            format!(
                "Failed to begin database transaction for anchor: {}",
                update.anchor_id
            )
        })?;

        let anchor = sqlx::query_as::<_, Anchor>(
            r"
                UPDATE anchors
                SET total_transactions = $1,
                    successful_transactions = $2,
                    failed_transactions = $3,
                    avg_settlement_time_ms = $4,
                    reliability_score = $5,
                    status = $6,
                    total_volume_usd = COALESCE($7, total_volume_usd),
                    updated_at = $8
                WHERE id = $9
                RETURNING *
                ",
        )
        .bind(update.total_transactions)
        .bind(update.successful_transactions)
        .bind(update.failed_transactions)
        .bind(update.avg_settlement_time_ms.unwrap_or(0))
        .bind(update.reliability_score)
        .bind(&update.status)
        .bind(update.volume_usd)
        .bind(Utc::now())
        .bind(update.anchor_id.to_string())
        .fetch_one(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "Failed to update anchor metrics for ID: {}",
                update.anchor_id
            )
        })?;

        let history_id = Uuid::new_v4().to_string();
        sqlx::query(
            r"
                INSERT INTO anchor_metrics_history (
                    id, anchor_id, timestamp, success_rate, failure_rate, reliability_score,
                    total_transactions, successful_transactions, failed_transactions,
                    avg_settlement_time_ms, volume_usd
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                ",
        )
        .bind(history_id)
        .bind(update.anchor_id.to_string())
        .bind(Utc::now())
        .bind(update.success_rate)
        .bind(update.failure_rate)
        .bind(update.reliability_score)
        .bind(update.total_transactions)
        .bind(update.successful_transactions)
        .bind(update.failed_transactions)
        .bind(update.avg_settlement_time_ms.unwrap_or(0))
        .bind(update.volume_usd.unwrap_or(0.0))
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "Failed to record anchor metrics history for ID: {}",
                update.anchor_id
            )
        })?;

        tx.commit().await.with_context(|| {
            format!(
                "Failed to commit anchor update transaction for ID: {}",
                update.anchor_id
            )
        })?;

        Ok(anchor)
    }

    /// Update anchor metrics from RPC ingestion.
    #[tracing::instrument(skip(self, params), fields(stellar_account = %params.stellar_account))]
    pub async fn update_anchor_from_rpc(&self, params: AnchorRpcUpdate) -> Result<()> {
        sqlx::query(
            r"
            UPDATE anchors
            SET total_transactions = $1,
                successful_transactions = $2,
                failed_transactions = $3,
                total_volume_usd = $4,
                avg_settlement_time_ms = $5,
                reliability_score = $6,
                status = $7,
                updated_at = $8
            WHERE stellar_account = $9
            ",
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
        .with_context(|| {
            format!(
                "Failed to update anchor from RPC for stellar_account: {}",
                params.stellar_account
            )
        })?;
        Ok(())
    }

    /// Records a new metric point for an anchor's history.
    #[tracing::instrument(skip(self, params), fields(anchor_id = %params.anchor_id))]
    pub async fn record_anchor_metrics_history(
        &self,
        params: AnchorMetricsParams,
    ) -> Result<AnchorMetricsHistory> {
        let id = Uuid::new_v4().to_string();
        let history = sqlx::query_as::<_, AnchorMetricsHistory>(
            r"
            INSERT INTO anchor_metrics_history (
                id, anchor_id, timestamp, success_rate, failure_rate, reliability_score,
                total_transactions, successful_transactions, failed_transactions,
                avg_settlement_time_ms, volume_usd
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            ",
        )
        .bind(id)
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
        .with_context(|| {
            format!(
                "Failed to record metrics history for anchor_id: {}",
                params.anchor_id
            )
        })?;
        Ok(history)
    }

    /// Retrieves the most recent metrics history entries for an anchor.
    #[tracing::instrument(skip(self), fields(anchor_id = %anchor_id, limit = limit))]
    pub async fn get_anchor_metrics_history(
        &self,
        anchor_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AnchorMetricsHistory>> {
        let history = sqlx::query_as::<_, AnchorMetricsHistory>(
            r"
            SELECT * FROM anchor_metrics_history
            WHERE anchor_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            ",
        )
        .bind(anchor_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to get metrics history for anchor_id: {} (limit={})",
                anchor_id, limit
            )
        })?;
        Ok(history)
    }

    /// Retrieves detailed information about an anchor, including its assets and metrics history.
    #[tracing::instrument(skip(self), fields(anchor_id = %anchor_id))]
    pub async fn get_anchor_detail(
        &self,
        anchor_id: Uuid,
        assets: Vec<Asset>,
        metrics_history: Vec<AnchorMetricsHistory>,
    ) -> Result<Option<AnchorDetailResponse>> {
        let anchor = match self
            .get_anchor_by_id(anchor_id)
            .await
            .with_context(|| format!("Failed to fetch anchor for detail view: {}", anchor_id))?
        {
            Some(a) => a,
            None => return Ok(None),
        };

        Ok(Some(AnchorDetailResponse {
            anchor,
            assets,
            metrics_history,
        }))
    }

    /// Retrieves the recent performance metrics for a specific anchor.
    #[tracing::instrument(skip(self), fields(anchor_id = %anchor_id, minutes = minutes))]
    pub async fn get_recent_anchor_performance(
        &self,
        anchor_id: &str,
        minutes: i64,
    ) -> Result<crate::models::AnchorMetrics> {
        let start_time = Utc::now() - chrono::Duration::minutes(minutes);

        let row: (i64, i64, Option<f64>) = sqlx::query_as(
            r"
                SELECT 
                    COUNT(*) as total,
                    COUNT(*) as successful,
                    AVG(amount) as avg_latency
                FROM payments
                WHERE (source_account = $1 OR destination_account = $2)
                AND created_at >= $3
                ",
        )
        .bind(anchor_id)
        .bind(anchor_id)
        .bind(start_time.to_rfc3339())
        .fetch_one(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to get recent anchor performance for anchor_id: {}, minutes: {}",
                anchor_id, minutes
            )
        })?;

        let total_transactions = row.0;
        let successful_transactions = row.1;
        let failed_transactions = total_transactions - successful_transactions;
        let success_rate = if total_transactions > 0 {
            (successful_transactions as f64 / total_transactions as f64) * 100.0
        } else {
            100.0
        };
        let failure_rate = 100.0 - success_rate;
        let avg_settlement_time_ms = row.2.map(|l| l as i32);

        let status = crate::models::AnchorStatus::from_metrics(success_rate, failure_rate);

        Ok(crate::models::AnchorMetrics {
            success_rate,
            failure_rate,
            reliability_score: success_rate,
            total_transactions,
            successful_transactions,
            failed_transactions,
            avg_settlement_time_ms,
            status,
        })
    }
}
