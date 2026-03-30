use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{ConnectOptions, SqlitePool};
use std::time::{Duration, Instant};

use crate::admin_audit_log::AdminAuditLogger;
use crate::cache::CacheManager;
use crate::db::{
    anchors::AnchorDb,
    api_keys::ApiKeyDb,
    assets::AssetDb,
    corridors::CorridorDb,
    metrics::MetricsDb,
};
use crate::models::api_key::{ApiKey, ApiKeyInfo, CreateApiKeyRequest, CreateApiKeyResponse};
use crate::models::{
    Anchor, AnchorDetailResponse, AnchorMetricsHistory, Asset, CreateAnchorRequest,
    MetricRecord, MuxedAccountAnalytics, SnapshotRecord,
};

// Re-export param types so existing callers don't need to change imports
pub use crate::db::anchors::{AnchorMetricsParams, AnchorMetricsUpdate, AnchorRpcUpdate};

// ---------------------------------------------------------------------------
// Pool configuration
// ---------------------------------------------------------------------------

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
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            max_connections: env_u32("DB_POOL_MAX_CONNECTIONS", 10),
            min_connections: env_u32("DB_POOL_MIN_CONNECTIONS", 2),
            connect_timeout_seconds: env_u64("DB_POOL_CONNECT_TIMEOUT_SECONDS", 30),
            idle_timeout_seconds: env_u64("DB_POOL_IDLE_TIMEOUT_SECONDS", 600),
            max_lifetime_seconds: env_u64("DB_POOL_MAX_LIFETIME_SECONDS", 1800),
        }
    }

    pub async fn create_pool(&self, database_url: &str) -> Result<SqlitePool> {
        let sql_log = SqlLogConfig::from_env();

        let mut opts: SqliteConnectOptions = database_url
            .parse()
            .map_err(|e: sqlx::Error| anyhow::anyhow!("Invalid DATABASE_URL: {e}"))
            .context("Failed to parse DATABASE_URL")?;

        opts = opts.journal_mode(SqliteJournalMode::Wal);

        if sql_log.level != log::LevelFilter::Off {
            if sql_log.log_all_in_dev {
                opts = opts.log_statements(sql_log.level);
            } else {
                opts = opts.log_slow_statements(
                    sql_log.level,
                    Duration::from_millis(sql_log.slow_query_threshold_ms),
                );
            }
        }

        sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .acquire_timeout(Duration::from_secs(self.connect_timeout_seconds))
            .idle_timeout(Some(Duration::from_secs(self.idle_timeout_seconds)))
            .max_lifetime(Some(Duration::from_secs(self.max_lifetime_seconds)))
            .connect_with(opts)
            .await
            .context("Failed to create SQLite connection pool")
    }
}

// ---------------------------------------------------------------------------
// SQL logging configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SqlLogConfig {
    pub level: log::LevelFilter,
    pub log_all_in_dev: bool,
    pub slow_query_threshold_ms: u64,
}

impl Default for SqlLogConfig {
    fn default() -> Self {
        Self {
            level: log::LevelFilter::Debug,
            log_all_in_dev: true,
            slow_query_threshold_ms: 100,
        }
    }
}

impl SqlLogConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let env_mode = std::env::var("RUST_ENV")
            .or_else(|_| std::env::var("ENVIRONMENT"))
            .unwrap_or_else(|_| "development".to_string());
        let is_dev =
            env_mode.eq_ignore_ascii_case("development") || env_mode.eq_ignore_ascii_case("dev");

        let level = parse_db_log_level(is_dev);
        let slow_query_threshold_ms = std::env::var("DB_SLOW_QUERY_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        Self {
            level,
            log_all_in_dev: is_dev,
            slow_query_threshold_ms,
        }
    }
}

fn parse_db_log_level(is_dev: bool) -> log::LevelFilter {
    let default = if is_dev {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    match std::env::var("DB_LOG_LEVEL")
        .unwrap_or_default()
        .to_uppercase()
        .as_str()
    {
        "TRACE" => log::LevelFilter::Trace,
        "DEBUG" => log::LevelFilter::Debug,
        "INFO" => log::LevelFilter::Info,
        "WARN" | "WARNING" => log::LevelFilter::Warn,
        "ERROR" => log::LevelFilter::Error,
        "OFF" | "NONE" => log::LevelFilter::Off,
        _ => default,
    }
}

// ---------------------------------------------------------------------------
// Pool metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct PoolMetrics {
    pub size: u32,
    pub idle: usize,
    pub active: u32,
}

impl PoolMetrics {
    #[must_use]
    pub const fn new(size: u32, idle: usize, active: u32) -> Self {
        Self { size, idle, active }
    }
}

// ---------------------------------------------------------------------------
// Database facade
// ---------------------------------------------------------------------------

pub struct Database {
    pool: SqlitePool,
    pub admin_audit_logger: AdminAuditLogger,
    slow_query_threshold_ms: u64,
}

impl Database {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        let admin_audit_logger = AdminAuditLogger::new(pool.clone());
        let slow_query_threshold_ms = std::env::var("SLOW_QUERY_THRESHOLD_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(100)
            .clamp(1, 60_000);
        Self {
            pool,
            admin_audit_logger,
            slow_query_threshold_ms,
        }
    }

    async fn execute_with_timing<T, F>(&self, operation: &str, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start = Instant::now();
        let result = f.await;
        let elapsed = start.elapsed();
        let status = if result.is_ok() { "success" } else { "error" };

        if elapsed.as_millis() as u64 > self.slow_query_threshold_ms {
            log::warn!(
                "Slow query: '{operation}' took {}ms (threshold: {}ms)",
                elapsed.as_millis(),
                self.slow_query_threshold_ms,
            );
        }
        crate::observability::metrics::observe_db_query(operation, status, elapsed.as_secs_f64());
        result
    }

    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    #[must_use]
    pub fn pool_metrics(&self) -> PoolMetrics {
        let size = self.pool.size();
        let idle = self.pool.num_idle();
        PoolMetrics::new(size, idle, size.saturating_sub(idle as u32))
    }

    pub fn corridor_aggregates(&self) -> crate::db::aggregates::CorridorAggregates {
        crate::db::aggregates::CorridorAggregates::new(self.pool.clone())
    }

    pub fn aggregation_db(&self) -> crate::db::aggregation::AggregationDb {
        crate::db::aggregation::AggregationDb::new(self.pool.clone())
    }

    // -- Anchor delegates --

    fn anchors(&self) -> AnchorDb {
        AnchorDb::new(self.pool.clone())
    }

    pub async fn create_anchor(&self, req: CreateAnchorRequest) -> Result<Anchor> {
        self.execute_with_timing("create_anchor", self.anchors().create(req)).await
    }

    pub async fn get_anchor_by_id(&self, id: Uuid) -> Result<Option<Anchor>> {
        self.execute_with_timing("get_anchor_by_id", self.anchors().get_by_id(id)).await
    }

    pub async fn get_anchor_by_stellar_account(&self, account: &str) -> Result<Option<Anchor>> {
        self.execute_with_timing(
            "get_anchor_by_stellar_account",
            self.anchors().get_by_stellar_account(account),
        )
        .await
    }

    pub async fn list_anchors(&self, limit: i64, offset: i64) -> Result<Vec<Anchor>> {
        self.execute_with_timing("list_anchors", self.anchors().list(limit, offset))
            .await
    }

    pub async fn get_all_anchors(&self) -> Result<Vec<Anchor>> {
        self.execute_with_timing("get_all_anchors", self.anchors().get_all())
            .await
    }

    pub async fn update_anchor_metrics(&self, update: AnchorMetricsUpdate) -> Result<Anchor> {
        self.execute_with_timing("update_anchor_metrics", self.anchors().update_metrics(update))
            .await
    }

    pub async fn update_anchor_from_rpc(&self, params: AnchorRpcUpdate) -> Result<()> {
        self.execute_with_timing("update_anchor_from_rpc", self.anchors().update_from_rpc(params))
            .await
    }

    pub async fn record_anchor_metrics_history(
        &self,
        params: AnchorMetricsParams,
    ) -> Result<AnchorMetricsHistory> {
        self.execute_with_timing(
            "record_anchor_metrics_history",
            self.anchors().record_metrics_history(params),
        )
        .await
    }

    pub async fn get_anchor_metrics_history(
        &self,
        anchor_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AnchorMetricsHistory>> {
        self.execute_with_timing(
            "get_anchor_metrics_history",
            self.anchors().get_metrics_history(anchor_id, limit),
        )
        .await
    }

    pub async fn get_anchor_detail(&self, anchor_id: Uuid) -> Result<Option<AnchorDetailResponse>> {
        self.execute_with_timing("get_anchor_detail", self.anchors().get_detail(anchor_id))
            .await
    }

    pub async fn get_recent_anchor_performance(
        &self,
        anchor_id: &str,
        minutes: i64,
    ) -> Result<crate::models::AnchorMetrics> {
        self.execute_with_timing(
            "get_recent_anchor_performance",
            self.anchors().get_recent_performance(anchor_id, minutes),
        )
        .await
    }

    // -- Asset delegates --

    fn assets(&self) -> AssetDb {
        AssetDb::new(self.pool.clone())
    }

    pub async fn create_asset(
        &self,
        anchor_id: Uuid,
        asset_code: String,
        asset_issuer: String,
    ) -> Result<Asset> {
        self.execute_with_timing(
            "create_asset",
            self.assets().create(anchor_id, asset_code, asset_issuer),
        )
        .await
    }

    pub async fn get_assets_by_anchor(&self, anchor_id: Uuid) -> Result<Vec<Asset>> {
        self.execute_with_timing(
            "get_assets_by_anchor",
            self.assets().get_by_anchor(anchor_id),
        )
        .await
    }

    pub async fn get_assets_by_anchors(
        &self,
        anchor_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<String, Vec<Asset>>> {
        self.execute_with_timing(
            "get_assets_by_anchors",
            self.assets().get_by_anchors(anchor_ids),
        )
        .await
    }

    pub async fn count_assets_by_anchor(&self, anchor_id: Uuid) -> Result<i64> {
        self.execute_with_timing(
            "count_assets_by_anchor",
            self.assets().count_by_anchor(anchor_id),
        )
        .await
    }

    // -- Corridor delegates --

    fn corridors(&self) -> CorridorDb {
        CorridorDb::new(self.pool.clone())
    }

    pub async fn create_corridor(
        &self,
        req: crate::models::CreateCorridorRequest,
    ) -> Result<crate::models::corridor::Corridor> {
        self.execute_with_timing("create_corridor", self.corridors().create(req))
            .await
    }

    pub async fn list_corridors(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::corridor::Corridor>> {
        self.execute_with_timing("list_corridors", self.corridors().list(limit, offset))
            .await
    }

    pub async fn get_corridor_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::corridor::Corridor>> {
        self.execute_with_timing("get_corridor_by_id", self.corridors().get_by_id(id))
            .await
    }

    pub async fn update_corridor_metrics(
        &self,
        id: Uuid,
        metrics: crate::models::corridor::CorridorMetrics,
        cache: &CacheManager,
    ) -> Result<crate::models::corridor::Corridor> {
        self.execute_with_timing(
            "update_corridor_metrics",
            self.corridors().update_metrics(id, metrics, cache),
        )
        .await
    }

    // -- Metrics/snapshot/ingestion/payment/muxed delegates --

    fn metrics(&self) -> MetricsDb {
        MetricsDb::new(self.pool.clone())
    }

    pub async fn record_metric(
        &self,
        name: &str,
        value: f64,
        entity_id: Option<String>,
        entity_type: Option<String>,
    ) -> Result<MetricRecord> {
        self.execute_with_timing(
            "record_metric",
            self.metrics().record_metric(name, value, entity_id, entity_type),
        )
        .await
    }

    pub async fn create_snapshot(
        &self,
        entity_id: &str,
        entity_type: &str,
        data: serde_json::Value,
        hash: Option<String>,
        epoch: Option<i64>,
    ) -> Result<SnapshotRecord> {
        self.execute_with_timing(
            "create_snapshot",
            self.metrics()
                .create_snapshot(entity_id, entity_type, data, hash, epoch),
        )
        .await
    }

    pub async fn get_snapshot_by_epoch(&self, epoch: i64) -> Result<Option<SnapshotRecord>> {
        self.execute_with_timing(
            "get_snapshot_by_epoch",
            self.metrics().get_snapshot_by_epoch(epoch),
        )
        .await
    }

    pub async fn list_snapshots(&self, limit: i64, offset: i64) -> Result<Vec<SnapshotRecord>> {
        self.execute_with_timing(
            "list_snapshots",
            self.metrics().list_snapshots(limit, offset),
        )
        .await
    }

    pub async fn get_ingestion_cursor(&self, task_name: &str) -> Result<Option<String>> {
        self.execute_with_timing(
            "get_ingestion_cursor",
            self.metrics().get_ingestion_cursor(task_name),
        )
        .await
    }

    pub async fn update_ingestion_cursor(&self, task_name: &str, last_cursor: &str) -> Result<()> {
        self.execute_with_timing(
            "update_ingestion_cursor",
            self.metrics().update_ingestion_cursor(task_name, last_cursor),
        )
        .await
    }

    pub async fn save_payments(&self, payments: Vec<crate::models::PaymentRecord>) -> Result<()> {
        self.execute_with_timing("save_payments", self.metrics().save_payments(payments))
            .await
    }

    pub async fn create_pending_transaction(
        &self,
        source_account: &str,
        xdr: &str,
        required_signatures: i32,
    ) -> Result<crate::models::PendingTransaction> {
        self.execute_with_timing(
            "create_pending_transaction",
            self.metrics()
                .create_pending_transaction(source_account, xdr, required_signatures),
        )
        .await
    }

    pub async fn get_pending_transaction(
        &self,
        id: &str,
    ) -> Result<Option<crate::models::PendingTransactionWithSignatures>> {
        self.execute_with_timing(
            "get_pending_transaction",
            self.metrics().get_pending_transaction(id),
        )
        .await
    }

    pub async fn add_transaction_signature(
        &self,
        transaction_id: &str,
        signer: &str,
        signature: &str,
    ) -> Result<()> {
        self.execute_with_timing(
            "add_transaction_signature",
            self.metrics()
                .add_transaction_signature(transaction_id, signer, signature),
        )
        .await
    }

    pub async fn update_transaction_status(&self, id: &str, status: &str) -> Result<()> {
        self.execute_with_timing(
            "update_transaction_status",
            self.metrics().update_transaction_status(id, status),
        )
        .await
    }

    pub async fn get_muxed_analytics(&self, top_limit: i64) -> Result<MuxedAccountAnalytics> {
        self.execute_with_timing(
            "get_muxed_analytics",
            self.metrics().get_muxed_analytics(top_limit),
        )
        .await
    }

    // -- API key delegates --

    fn api_keys(&self) -> ApiKeyDb {
        ApiKeyDb::new(self.pool.clone())
    }

    pub async fn create_api_key(
        &self,
        wallet_address: &str,
        req: CreateApiKeyRequest,
    ) -> Result<CreateApiKeyResponse> {
        self.execute_with_timing("create_api_key", self.api_keys().create(wallet_address, req))
            .await
    }

    pub async fn list_api_keys(&self, wallet_address: &str) -> Result<Vec<ApiKeyInfo>> {
        self.execute_with_timing("list_api_keys", self.api_keys().list(wallet_address))
            .await
    }

    pub async fn get_api_key_by_id(
        &self,
        id: &str,
        wallet_address: &str,
    ) -> Result<Option<ApiKeyInfo>> {
        self.execute_with_timing(
            "get_api_key_by_id",
            self.api_keys().get_by_id(id, wallet_address),
        )
        .await
    }

    pub async fn validate_api_key(&self, plain_key: &str) -> Result<Option<ApiKey>> {
        self.execute_with_timing("validate_api_key", self.api_keys().validate(plain_key))
            .await
    }

    pub async fn revoke_api_key(&self, id: &str, wallet_address: &str) -> Result<bool> {
        self.execute_with_timing("revoke_api_key", self.api_keys().revoke(id, wallet_address))
            .await
    }

    pub async fn rotate_api_key(
        &self,
        id: &str,
        wallet_address: &str,
    ) -> Result<Option<CreateApiKeyResponse>> {
        self.execute_with_timing(
            "rotate_api_key",
            self.api_keys().rotate(id, wallet_address),
        )
        .await
    }

    // -- Aggregation delegates (pass-through) --

    pub async fn fetch_payments_by_timerange(
        &self,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<crate::models::corridor::PaymentRecord>> {
        self.execute_with_timing(
            "fetch_payments_by_timerange",
            self.aggregation_db()
                .fetch_payments_by_timerange(start_time, end_time, limit),
        )
        .await
    }

    pub async fn upsert_hourly_corridor_metric(
        &self,
        metric: &crate::models::corridor::HourlyCorridorMetrics,
    ) -> Result<()> {
        self.execute_with_timing(
            "upsert_hourly_corridor_metric",
            self.aggregation_db().upsert_hourly_corridor_metric(metric),
        )
        .await
    }

    pub async fn fetch_hourly_metrics_by_timerange(
        &self,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<crate::models::corridor::HourlyCorridorMetrics>> {
        self.execute_with_timing(
            "fetch_hourly_metrics_by_timerange",
            self.aggregation_db()
                .fetch_hourly_metrics_by_timerange(start_time, end_time),
        )
        .await
    }

    pub async fn create_aggregation_job(&self, job_id: &str, job_type: &str) -> Result<()> {
        self.execute_with_timing(
            "create_aggregation_job",
            self.aggregation_db().create_aggregation_job(job_id, job_type),
        )
        .await
    }

    pub async fn update_aggregation_job_status(
        &self,
        job_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.execute_with_timing(
            "update_aggregation_job_status",
            self.aggregation_db()
                .update_aggregation_job_status(job_id, status, error_message),
        )
        .await
    }

    pub async fn update_last_processed_hour(&self, job_id: &str, last_hour: &str) -> Result<()> {
        self.execute_with_timing(
            "update_last_processed_hour",
            self.aggregation_db()
                .update_last_processed_hour(job_id, last_hour),
        )
        .await
    }

    pub async fn get_job_retry_count(&self, job_id: &str) -> Result<i32> {
        self.execute_with_timing(
            "get_job_retry_count",
            self.aggregation_db().get_job_retry_count(job_id),
        )
        .await
    }

    pub async fn increment_job_retry_count(&self, job_id: &str) -> Result<()> {
        self.execute_with_timing(
            "increment_job_retry_count",
            self.aggregation_db().increment_job_retry_count(job_id),
        )
        .await
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
