use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use std::time::Duration;
use uuid::Uuid;

use crate::analytics::compute_anchor_metrics;
use crate::models::{
    Anchor, AnchorDetailResponse, AnchorMetricsHistory, Asset, CorridorRecord, CreateAnchorRequest,
    MetricRecord, MuxedAccountAnalytics, MuxedAccountUsage, SnapshotRecord,
};

/// Configuration for database connection pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 2,
            connect_timeout_seconds: 30,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 1800,
        }
    }
}

impl PoolConfig {
    /// Load pool configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            max_connections: std::env::var("DB_POOL_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            min_connections: std::env::var("DB_POOL_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2),
            connect_timeout_seconds: std::env::var("DB_POOL_CONNECT_TIMEOUT_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            idle_timeout_seconds: std::env::var("DB_POOL_IDLE_TIMEOUT_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(600),
            max_lifetime_seconds: std::env::var("DB_POOL_MAX_LIFETIME_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1800),
        }
    }

    /// Create a configured SQLite pool with these settings
    pub async fn create_pool(&self, database_url: &str) -> Result<SqlitePool> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .acquire_timeout(Duration::from_secs(self.connect_timeout_seconds))
            .idle_timeout(Some(Duration::from_secs(self.idle_timeout_seconds)))
            .max_lifetime(Some(Duration::from_secs(self.max_lifetime_seconds)))
            .connect(database_url)
            .await?;

        Ok(pool)
    }
}

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

/// Connection pool metrics
#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolMetrics {
    pub size: u32,
    pub idle: usize,
}

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

    /// Get connection pool metrics
    pub fn pool_metrics(&self) -> PoolMetrics {
        PoolMetrics {
            size: self.pool.size(),
            idle: self.pool.num_idle(),
        }
    }

    // Anchor operations
    pub async fn create_anchor(&self, req: CreateAnchorRequest) -> Result<Anchor> {
        let id = Uuid::new_v4().to_string();
        let anchor = sqlx::query_as::<_, Anchor>(
            r#"
            INSERT INTO anchors (id, name, stellar_account, home_domain)
            VALUES ($1, $2, $3, $4)
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
            SELECT * FROM anchors WHERE id = $1
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
            SELECT * FROM anchors WHERE stellar_account = $1
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
            LIMIT $1 OFFSET $2
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
        self.record_anchor_metrics_history(AnchorMetricsParams {
            anchor_id,
            success_rate: metrics.success_rate,
            failure_rate: metrics.failure_rate,
            reliability_score: metrics.reliability_score,
            total_transactions,
            successful_transactions,
            failed_transactions,
            avg_settlement_time_ms,
            volume_usd,
        })
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
            VALUES ($1, $2, $3, $4)
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
            SELECT * FROM assets WHERE anchor_id = $1
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
            SELECT COUNT(*) FROM assets WHERE anchor_id = $1
            "#,
        )
        .bind(anchor_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    // Update anchor metrics from RPC ingestion
    pub async fn update_anchor_from_rpc(&self, params: AnchorRpcUpdate) -> Result<()> {
        sqlx::query(
            r#"
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
            "#,
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
        .await?;

        Ok(())
    }

    // Metrics history operations
    pub async fn record_anchor_metrics_history(
        &self,
        params: AnchorMetricsParams,
    ) -> Result<AnchorMetricsHistory> {
        let id = Uuid::new_v4().to_string();
        let history = sqlx::query_as::<_, AnchorMetricsHistory>(
            r#"
            INSERT INTO anchor_metrics_history (
                id, anchor_id, timestamp, success_rate, failure_rate, reliability_score,
                total_transactions, successful_transactions, failed_transactions,
                avg_settlement_time_ms, volume_usd
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
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
            WHERE anchor_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
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
            VALUES ($1, $2, $3, $4, $5)
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
            SELECT * FROM corridors ORDER BY reliability_score DESC LIMIT $1 OFFSET $2
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
            SELECT * FROM corridors WHERE id = $1
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
            SET reliability_score = $1,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
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
            VALUES ($1, $2, $3, $4, $5, $6)
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
            VALUES ($1, $2, $3, $4, $5, $6, $7)
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
            SELECT * FROM snapshots WHERE epoch = $1 LIMIT 1
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
            LIMIT $1 OFFSET $2
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
            SELECT * FROM ingestion_state WHERE task_name = $1
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
            VALUES ($1, $2, $3)
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
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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

    /// Muxed account analytics: counts and top addresses from payments table.
    /// Uses M-address detection (starts with 'M', length 69).
    pub async fn get_muxed_analytics(&self, top_limit: i64) -> Result<MuxedAccountAnalytics> {
        use crate::muxed;
        const MUXED_LEN: i64 = 69;

        let total_muxed_payments = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM payments
            WHERE (source_account LIKE 'M%' AND LENGTH(source_account) = ?1)
               OR (destination_account LIKE 'M%' AND LENGTH(destination_account) = ?1)
            "#,
        )
        .bind(MUXED_LEN)
        .fetch_one(&self.pool)
        .await?;

        #[derive(sqlx::FromRow)]
        struct AddrCount {
            addr: String,
            cnt: i64,
        }

        let source_counts: Vec<AddrCount> = sqlx::query_as(
            r#"
            SELECT source_account AS addr, COUNT(*) AS cnt FROM payments
            WHERE source_account LIKE 'M%' AND LENGTH(source_account) = ?1
            GROUP BY source_account
            ORDER BY cnt DESC
            LIMIT ?2
            "#,
        )
        .bind(MUXED_LEN)
        .bind(top_limit)
        .fetch_all(&self.pool)
        .await?;

        let dest_counts: Vec<AddrCount> = sqlx::query_as(
            r#"
            SELECT destination_account AS addr, COUNT(*) AS cnt FROM payments
            WHERE destination_account LIKE 'M%' AND LENGTH(destination_account) = ?1
            GROUP BY destination_account
            ORDER BY cnt DESC
            LIMIT ?2
            "#,
        )
        .bind(MUXED_LEN)
        .bind(top_limit)
        .fetch_all(&self.pool)
        .await?;

        let mut by_addr: std::collections::HashMap<String, (i64, i64)> =
            std::collections::HashMap::new();
        for row in source_counts {
            by_addr.entry(row.addr).or_insert((0, 0)).0 = row.cnt;
        }
        for row in dest_counts {
            by_addr.entry(row.addr).or_insert((0, 0)).1 = row.cnt;
        }

        let mut top_muxed_by_activity: Vec<MuxedAccountUsage> = by_addr
            .into_iter()
            .map(|(account_address, (src, dest))| {
                let total = src + dest;
                let info = muxed::parse_muxed_address(&account_address);
                MuxedAccountUsage {
                    account_address,
                    base_account: info.as_ref().and_then(|i| i.base_account.clone()),
                    muxed_id: info.and_then(|i| i.muxed_id),
                    payment_count_as_source: src,
                    payment_count_as_destination: dest,
                    total_payments: total,
                }
            })
            .collect();
        top_muxed_by_activity.sort_by(|a, b| b.total_payments.cmp(&a.total_payments));
        top_muxed_by_activity.truncate(top_limit as usize);

        let unique_muxed_addresses = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(DISTINCT addr) FROM (
                SELECT source_account AS addr FROM payments WHERE source_account LIKE 'M%' AND LENGTH(source_account) = ?1
                UNION
                SELECT destination_account AS addr FROM payments WHERE destination_account LIKE 'M%' AND LENGTH(destination_account) = ?1
            )
            "#,
        )
        .bind(MUXED_LEN)
        .fetch_one(&self.pool)
        .await?;

        let base_accounts_with_muxed: Vec<String> = top_muxed_by_activity
            .iter()
            .filter_map(|u| u.base_account.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();

        Ok(MuxedAccountAnalytics {
            total_muxed_payments,
            unique_muxed_addresses,
            top_muxed_by_activity,
            base_accounts_with_muxed,
        })
    }

    // =========================
    // Transaction Builder Methods
    // =========================

    pub async fn create_pending_transaction(
        &self,
        source_account: &str,
        xdr: &str,
        required_signatures: i32,
    ) -> Result<crate::models::PendingTransaction> {
        let id = Uuid::new_v4().to_string();
        let status = "pending";

        let tx = sqlx::query_as::<_, crate::models::PendingTransaction>(
            r#"
            INSERT INTO pending_transactions (id, source_account, xdr, required_signatures, status)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(source_account)
        .bind(xdr)
        .bind(required_signatures)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        Ok(tx)
    }

    pub async fn get_pending_transaction(
        &self,
        id: &str,
    ) -> Result<Option<crate::models::PendingTransactionWithSignatures>> {
        let tx = sqlx::query_as::<_, crate::models::PendingTransaction>(
            r#"
            SELECT * FROM pending_transactions WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(transaction) = tx {
            let signatures = sqlx::query_as::<_, crate::models::Signature>(
                r#"
                SELECT * FROM transaction_signatures WHERE transaction_id = $1
                "#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await?;

            Ok(Some(crate::models::PendingTransactionWithSignatures {
                transaction,
                collected_signatures: signatures,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn add_transaction_signature(
        &self,
        transaction_id: &str,
        signer: &str,
        signature: &str,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO transaction_signatures (id, transaction_id, signer, signature)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(id)
        .bind(transaction_id)
        .bind(signer)
        .bind(signature)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_transaction_status(&self, id: &str, status: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE pending_transactions
            SET status = $1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            "#,
        )
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
