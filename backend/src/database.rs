use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::analytics::compute_anchor_metrics;
use crate::models::{
    Anchor, AnchorDetailResponse, AnchorMetricsHistory, Asset, CorridorRecord, CreateAnchorRequest,
    MetricRecord, SnapshotRecord,
};

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn corridor_aggregates(&self) -> crate::db::aggregates::CorridorAggregates {
        crate::db::aggregates::CorridorAggregates::new(self.pool.clone())
    }

    // Anchor operations
    pub async fn create_anchor(&self, req: CreateAnchorRequest) -> Result<Anchor> {
        let id = Uuid::new_v4().to_string();
        let anchor = sqlx::query_as::<_, Anchor>(
            r#"
            INSERT INTO anchors (id, name, stellar_account, home_domain)
            VALUES (?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.stellar_account)
        .bind(&req.home_domain)
        .fetch_one(&self.pool)
        .await?;

        Ok(anchor)
    }

    pub async fn get_anchor_by_id(&self, id: Uuid) -> Result<Option<Anchor>> {
        let anchor = sqlx::query_as::<_, Anchor>(
            r#"
            SELECT * FROM anchors WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(anchor)
    }

    pub async fn get_anchor_by_stellar_account(
        &self,
        stellar_account: &str,
    ) -> Result<Option<Anchor>> {
        let anchor = sqlx::query_as::<_, Anchor>(
            r#"
            SELECT * FROM anchors WHERE stellar_account = ?
            "#,
        )
        .bind(stellar_account)
        .fetch_optional(&self.pool)
        .await?;

        Ok(anchor)
    }

    pub async fn list_anchors(&self, limit: i64, offset: i64) -> Result<Vec<Anchor>> {
        let anchors = sqlx::query_as::<_, Anchor>(
            r#"
            SELECT * FROM anchors
            ORDER BY reliability_score DESC, updated_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(anchors)
    }

    pub async fn update_anchor_metrics(
        &self,
        anchor_id: Uuid,
        total_transactions: i64,
        successful_transactions: i64,
        failed_transactions: i64,
        avg_settlement_time_ms: Option<i32>,
        volume_usd: Option<f64>,
    ) -> Result<Anchor> {
        // Compute metrics
        let metrics = compute_anchor_metrics(
            total_transactions,
            successful_transactions,
            failed_transactions,
            avg_settlement_time_ms,
        );

        // Update anchor
        let anchor = sqlx::query_as::<_, Anchor>(
            r#"
            UPDATE anchors
            SET total_transactions = ?,
                successful_transactions = ?,
                failed_transactions = ?,
                avg_settlement_time_ms = ?,
                reliability_score = ?,
                status = ?,
                total_volume_usd = COALESCE(?, total_volume_usd),
                updated_at = ?
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(total_transactions)
        .bind(successful_transactions)
        .bind(failed_transactions)
        .bind(avg_settlement_time_ms.unwrap_or(0))
        .bind(metrics.reliability_score)
        .bind(metrics.status.as_str())
        .bind(volume_usd.unwrap_or(0.0))
        .bind(Utc::now())
        .bind(anchor_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        // Record metrics history
        self.record_anchor_metrics_history(
            anchor_id,
            metrics.success_rate,
            metrics.failure_rate,
            metrics.reliability_score,
            total_transactions,
            successful_transactions,
            failed_transactions,
            avg_settlement_time_ms,
            volume_usd,
        )
        .await?;

        Ok(anchor)
    }

    // Asset operations
    pub async fn create_asset(
        &self,
        anchor_id: Uuid,
        asset_code: String,
        asset_issuer: String,
    ) -> Result<Asset> {
        let id = Uuid::new_v4().to_string();
        let asset = sqlx::query_as::<_, Asset>(
            r#"
            INSERT INTO assets (id, anchor_id, asset_code, asset_issuer)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (asset_code, asset_issuer) DO UPDATE
            SET anchor_id = EXCLUDED.anchor_id,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(anchor_id.to_string())
        .bind(&asset_code)
        .bind(&asset_issuer)
        .fetch_one(&self.pool)
        .await?;

        Ok(asset)
    }

    pub async fn get_assets_by_anchor(&self, anchor_id: Uuid) -> Result<Vec<Asset>> {
        let assets = sqlx::query_as::<_, Asset>(
            r#"
            SELECT * FROM assets WHERE anchor_id = ?
            ORDER BY asset_code ASC
            "#,
        )
        .bind(anchor_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(assets)
    }

    pub async fn count_assets_by_anchor(&self, anchor_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM assets WHERE anchor_id = ?
            "#,
        )
        .bind(anchor_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    // Update anchor metrics from RPC ingestion
    pub async fn update_anchor_from_rpc(
        &self,
        stellar_account: &str,
        total_transactions: i64,
        successful_transactions: i64,
        failed_transactions: i64,
        total_volume_usd: f64,
        avg_settlement_time_ms: i32,
        reliability_score: f64,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE anchors
            SET total_transactions = ?,
                successful_transactions = ?,
                failed_transactions = ?,
                total_volume_usd = ?,
                avg_settlement_time_ms = ?,
                reliability_score = ?,
                status = ?,
                updated_at = ?
            WHERE stellar_account = ?
            "#,
        )
        .bind(total_transactions)
        .bind(successful_transactions)
        .bind(failed_transactions)
        .bind(total_volume_usd)
        .bind(avg_settlement_time_ms)
        .bind(reliability_score)
        .bind(status)
        .bind(Utc::now())
        .bind(stellar_account)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Metrics history operations
    pub async fn record_anchor_metrics_history(
        &self,
        anchor_id: Uuid,
        success_rate: f64,
        failure_rate: f64,
        reliability_score: f64,
        total_transactions: i64,
        successful_transactions: i64,
        failed_transactions: i64,
        avg_settlement_time_ms: Option<i32>,
        volume_usd: Option<f64>,
    ) -> Result<AnchorMetricsHistory> {
        let id = Uuid::new_v4().to_string();
        let history = sqlx::query_as::<_, AnchorMetricsHistory>(
            r#"
            INSERT INTO anchor_metrics_history (
                id, anchor_id, timestamp, success_rate, failure_rate, reliability_score,
                total_transactions, successful_transactions, failed_transactions,
                avg_settlement_time_ms, volume_usd
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(anchor_id.to_string())
        .bind(Utc::now())
        .bind(success_rate)
        .bind(failure_rate)
        .bind(reliability_score)
        .bind(total_transactions)
        .bind(successful_transactions)
        .bind(failed_transactions)
        .bind(avg_settlement_time_ms.unwrap_or(0))
        .bind(volume_usd.unwrap_or(0.0))
        .fetch_one(&self.pool)
        .await?;

        Ok(history)
    }

    pub async fn get_anchor_metrics_history(
        &self,
        anchor_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AnchorMetricsHistory>> {
        let history = sqlx::query_as::<_, AnchorMetricsHistory>(
            r#"
            SELECT * FROM anchor_metrics_history
            WHERE anchor_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(anchor_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(history)
    }

    pub async fn get_anchor_detail(&self, anchor_id: Uuid) -> Result<Option<AnchorDetailResponse>> {
        let anchor = match self.get_anchor_by_id(anchor_id).await? {
            Some(a) => a,
            None => return Ok(None),
        };

        let assets = self.get_assets_by_anchor(anchor_id).await?;
        let metrics_history = self.get_anchor_metrics_history(anchor_id, 30).await?;

        Ok(Some(AnchorDetailResponse {
            anchor,
            assets,
            metrics_history,
        }))
    }

    // Corridor operations
    pub async fn create_corridor(
        &self,
        req: crate::models::CreateCorridorRequest,
    ) -> Result<crate::models::corridor::Corridor> {
        let corridor = crate::models::corridor::Corridor::new(
            req.source_asset_code,
            req.source_asset_issuer,
            req.dest_asset_code,
            req.dest_asset_issuer,
        );

        // Ensure the corridor exists in the database
        sqlx::query(
            r#"
            INSERT INTO corridors (
                id, source_asset_code, source_asset_issuer,
                destination_asset_code, destination_asset_issuer
            )
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer)
            DO UPDATE SET updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&corridor.asset_a_code)
        .bind(&corridor.asset_a_issuer)
        .bind(&corridor.asset_b_code)
        .bind(&corridor.asset_b_issuer)
        .execute(&self.pool)
        .await?;

        Ok(corridor)
    }

    pub async fn list_corridors(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::corridor::Corridor>> {
        let records = sqlx::query_as::<_, CorridorRecord>(
            r#"
            SELECT * FROM corridors ORDER BY reliability_score DESC LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(records
            .into_iter()
            .map(|r| {
                crate::models::corridor::Corridor::new(
                    r.source_asset_code,
                    r.source_asset_issuer,
                    r.destination_asset_code,
                    r.destination_asset_issuer,
                )
            })
            .collect())
    }

    pub async fn get_corridor_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::corridor::Corridor>> {
        let record = sqlx::query_as::<_, CorridorRecord>(
            r#"
            SELECT * FROM corridors WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| {
            crate::models::corridor::Corridor::new(
                r.source_asset_code,
                r.source_asset_issuer,
                r.destination_asset_code,
                r.destination_asset_issuer,
            )
        }))
    }

    pub async fn update_corridor_metrics(
        &self,
        id: Uuid,
        metrics: crate::models::corridor::CorridorMetrics,
    ) -> Result<crate::models::corridor::Corridor> {
        let record = sqlx::query_as::<_, CorridorRecord>(
            r#"
            UPDATE corridors
            SET reliability_score = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(metrics.success_rate)
        .bind(id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::corridor::Corridor::new(
            record.source_asset_code,
            record.source_asset_issuer,
            record.destination_asset_code,
            record.destination_asset_issuer,
        ))
    }

    // Generic Metric operations
    pub async fn record_metric(
        &self,
        name: &str,
        value: f64,
        entity_id: Option<String>,
        entity_type: Option<String>,
    ) -> Result<MetricRecord> {
        let id = Uuid::new_v4().to_string();
        let metric = sqlx::query_as::<_, MetricRecord>(
            r#"
            INSERT INTO metrics (id, name, value, entity_id, entity_type, timestamp)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(value)
        .bind(entity_id)
        .bind(entity_type)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(metric)
    }

    // Snapshot operations
    pub async fn create_snapshot(
        &self,
        entity_id: &str,
        entity_type: &str,
        data: serde_json::Value,
        hash: Option<String>,
        epoch: Option<i64>,
    ) -> Result<SnapshotRecord> {
        let id = Uuid::new_v4().to_string();
        let snapshot = sqlx::query_as::<_, SnapshotRecord>(
            r#"
            INSERT INTO snapshots (id, entity_id, entity_type, data, hash, epoch, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(entity_id)
        .bind(entity_type)
        .bind(data.to_string())
        .bind(hash)
        .bind(epoch)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(snapshot)
    }

    pub async fn get_snapshot_by_epoch(&self, epoch: i64) -> Result<Option<SnapshotRecord>> {
        let snapshot = sqlx::query_as::<_, SnapshotRecord>(
            r#"
            SELECT * FROM snapshots WHERE epoch = ? LIMIT 1
            "#,
        )
        .bind(epoch)
        .fetch_optional(&self.pool)
        .await?;

        Ok(snapshot)
    }

    pub async fn list_snapshots(&self, limit: i64, offset: i64) -> Result<Vec<SnapshotRecord>> {
        let snapshots = sqlx::query_as::<_, SnapshotRecord>(
            r#"
            SELECT * FROM snapshots
            WHERE epoch IS NOT NULL
            ORDER BY epoch DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(snapshots)
    }

    // Ingestion methods
    pub async fn get_ingestion_cursor(&self, task_name: &str) -> Result<Option<String>> {
        let state = sqlx::query_as::<_, crate::models::IngestionState>(
            r#"
            SELECT * FROM ingestion_state WHERE task_name = ?
            "#,
        )
        .bind(task_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(state.map(|s| s.last_cursor))
    }

    pub async fn update_ingestion_cursor(&self, task_name: &str, last_cursor: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO ingestion_state (task_name, last_cursor, updated_at)
            VALUES (?, ?, ?)
            ON CONFLICT (task_name) DO UPDATE SET
                last_cursor = EXCLUDED.last_cursor,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(task_name)
        .bind(last_cursor)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn save_payments(&self, payments: Vec<crate::models::PaymentRecord>) -> Result<()> {
        for payment in payments {
            sqlx::query(
                r#"
                INSERT INTO payments (
                    id, transaction_hash, source_account, destination_account,
                    asset_type, asset_code, asset_issuer, amount, created_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (id) DO NOTHING
                "#,
            )
            .bind(&payment.id)
            .bind(&payment.transaction_hash)
            .bind(&payment.source_account)
            .bind(&payment.destination_account)
            .bind(&payment.asset_type)
            .bind(&payment.asset_code)
            .bind(&payment.asset_issuer)
            .bind(payment.amount)
            .bind(payment.created_at)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    // Aggregation methods
    pub fn aggregation_db(&self) -> crate::db::aggregation::AggregationDb {
        crate::db::aggregation::AggregationDb::new(self.pool.clone())
    }

    pub async fn fetch_payments_by_timerange(
        &self,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<crate::models::corridor::PaymentRecord>> {
        self.aggregation_db()
            .fetch_payments_by_timerange(start_time, end_time, limit)
            .await
    }

    pub async fn upsert_hourly_corridor_metric(
        &self,
        metric: &crate::services::aggregation::HourlyCorridorMetrics,
    ) -> Result<()> {
        self.aggregation_db()
            .upsert_hourly_corridor_metric(metric)
            .await
    }

    pub async fn fetch_hourly_metrics_by_timerange(
        &self,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<crate::services::aggregation::HourlyCorridorMetrics>> {
        self.aggregation_db()
            .fetch_hourly_metrics_by_timerange(start_time, end_time)
            .await
    }

    pub async fn create_aggregation_job(&self, job_id: &str, job_type: &str) -> Result<()> {
        self.aggregation_db()
            .create_aggregation_job(job_id, job_type)
            .await
    }

    pub async fn update_aggregation_job_status(
        &self,
        job_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.aggregation_db()
            .update_aggregation_job_status(job_id, status, error_message)
            .await
    }

    pub async fn update_last_processed_hour(&self, job_id: &str, last_hour: &str) -> Result<()> {
        self.aggregation_db()
            .update_last_processed_hour(job_id, last_hour)
            .await
    }

    pub async fn get_job_retry_count(&self, job_id: &str) -> Result<i32> {
        self.aggregation_db().get_job_retry_count(job_id).await
    }

    pub async fn increment_job_retry_count(&self, job_id: &str) -> Result<()> {
        self.aggregation_db()
            .increment_job_retry_count(job_id)
            .await
    }
}
