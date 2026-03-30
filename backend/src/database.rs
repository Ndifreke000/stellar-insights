use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{ConnectOptions, SqlitePool};
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::admin_audit_log::AdminAuditLogger;
use crate::analytics::compute_anchor_metrics;
use crate::cache::CacheManager;
use crate::models::api_key::{
    generate_api_key, hash_api_key, ApiKey, ApiKeyInfo, CreateApiKeyRequest, CreateApiKeyResponse,
};
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

/// SQL query logging configuration
#[derive(Debug, Clone)]
pub struct SqlLogConfig {
    /// Log level for statements (trace, debug, info, warn, error, off)
    pub level: log::LevelFilter,
    /// In development: log all queries. In production: log only slow queries.
    pub log_all_in_dev: bool,
    /// Slow query threshold (ms); only used when `log_all_in_dev` is false.
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
    /// Load from environment:
    /// - `RUST_ENV` or ENVIRONMENT: "development" => log all queries, else log only slow
    /// - `DB_LOG_LEVEL`: trace | debug | info | warn | error | off (default: debug in dev, info in prod)
    /// - `DB_SLOW_QUERY_MS`: threshold in ms for slow query logging in production (default: 100)
    #[must_use]
    pub fn from_env() -> Self {
        let env_mode = std::env::var("RUST_ENV")
            .or_else(|_| std::env::var("ENVIRONMENT"))
            .unwrap_or_else(|_| "development".to_string());
        let is_dev = env_mode.to_lowercase() == "development" || env_mode.to_lowercase() == "dev";

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
    let s = std::env::var("DB_LOG_LEVEL").unwrap_or_else(|_| String::new());
    match s.to_uppercase().as_str() {
        "TRACE" => log::LevelFilter::Trace,
        "DEBUG" => log::LevelFilter::Debug,
        "INFO" => log::LevelFilter::Info,
        "WARN" | "WARNING" => log::LevelFilter::Warn,
        "ERROR" => log::LevelFilter::Error,
        "OFF" | "NONE" => log::LevelFilter::Off,
        _ => default,
    }
}

impl PoolConfig {
    /// Load pool configuration from environment variables
    #[must_use]
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

    /// Create a configured `SQLite` pool with these settings.
    /// Uses WAL journal mode and configurable SQL query logging (all in dev, slow-only in prod).
    pub async fn create_pool(&self, database_url: &str) -> Result<SqlitePool> {
        let sql_log = SqlLogConfig::from_env();

        let mut opts: SqliteConnectOptions = database_url
            .parse()
            .map_err(|e: sqlx::Error| anyhow::anyhow!("Invalid DATABASE_URL: {e}"))
            .context("Failed to parse DATABASE_URL for SQLite connection")?;

        opts = opts.journal_mode(SqliteJournalMode::Wal);

        if sql_log.level != log::LevelFilter::Off {
            if sql_log.log_all_in_dev {
                opts = opts.log_statements(sql_log.level);
                tracing::info!(
                    "SQL query logging: all queries at level {:?} (development)",
                    sql_log.level
                );
            } else {
                let threshold = Duration::from_millis(sql_log.slow_query_threshold_ms);
                opts = opts.log_slow_statements(sql_log.level, threshold);
                tracing::info!(
                    "SQL query logging: slow queries only (> {} ms) at level {:?} (production)",
                    sql_log.slow_query_threshold_ms,
                    sql_log.level
                );
            }
        }

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .acquire_timeout(Duration::from_secs(self.connect_timeout_seconds))
            .idle_timeout(Some(Duration::from_secs(self.idle_timeout_seconds)))
            .max_lifetime(Some(Duration::from_secs(self.max_lifetime_seconds)))
            .connect_with(opts)
            .await
            .context("Failed to create SQLite connection pool")?;

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

/// Parameters for updating anchor metrics
#[derive(Debug, Clone)]
pub struct AnchorMetricsUpdate {
    pub anchor_id: Uuid,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_time_ms: Option<i32>,
    pub volume_usd: Option<f64>,
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
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[derive(Debug, Clone, Copy, Serialize)]
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

pub struct Database {
    pool: SqlitePool,
    pub admin_audit_logger: AdminAuditLogger,
    /// Threshold in milliseconds above which a query is logged as slow at WARN level.
    /// Loaded from `SLOW_QUERY_THRESHOLD_MS` (default: 100).
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
            .clamp(1, 60_000); // 1ms minimum, 60s maximum
        Self {
            pool,
            admin_audit_logger,
            slow_query_threshold_ms,
        }
    }

    /// Executes `f`, records its duration via `observe_db_query`, and emits a WARN log
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
                "Slow query detected: '{}' took {}ms (threshold: {}ms)",
                operation,
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

    /// Performs a basic connectivity check against the database.
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("Database health check failed")?;
        Ok(())
    }

    #[must_use]
    pub fn pool_metrics(&self) -> PoolMetrics {
        let size = self.pool.size();
        let idle = self.pool.num_idle();
        let active = size.saturating_sub(idle as u32);

        PoolMetrics::new(size, idle, active)
    }

    pub fn corridor_aggregates(&self) -> crate::db::aggregates::CorridorAggregates {
        crate::db::aggregates::CorridorAggregates::new(self.pool.clone())
    }

    // Anchor operations

    /// Creates a new anchor in the database.
    ///
    /// # Arguments
    ///
    /// * `req` - Anchor creation request containing name, `stellar_account`, and `home_domain`
    ///
    /// # Returns
    ///
    /// * `Ok(Anchor)` - Newly created anchor with generated UUID
    /// * `Err(_)` - Database insertion failed
    ///
    /// # Examples
    ///
    /// ```rust
    /// let req = CreateAnchorRequest {
    ///     name: "Example Anchor".to_string(),
    ///     stellar_account: "GBRPYHIL...".to_string(),
    ///     home_domain: Some("example.com".to_string()),
    /// };
    /// let anchor = db.create_anchor(req).await?;
    /// ```
    #[tracing::instrument(skip(self, req), fields(anchor_name = %req.name, stellar_account = %req.stellar_account))]
    pub async fn create_anchor(&self, req: CreateAnchorRequest) -> Result<Anchor> {
        self.execute_with_timing("create_anchor", async {
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
        })
        .await
    }

    /// Retrieves an anchor by its unique identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID of the anchor to retrieve
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Anchor))` - Anchor found and returned
    /// * `Ok(None)` - No anchor exists with the given ID
    /// * `Err(_)` - Database query failed
    ///
    /// # Examples
    ///
    /// ```rust
    /// let anchor_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
    /// let anchor = db.get_anchor_by_id(anchor_id).await?;
    ///
    /// match anchor {
    ///     Some(a) => println!("Found anchor: {}", a.name),
    ///     None => println!("Anchor not found"),
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// Indexed query on primary key, typically <1ms.
    #[tracing::instrument(skip(self), fields(anchor_id = %id))]
    pub async fn get_anchor_by_id(&self, id: Uuid) -> Result<Option<Anchor>> {
        self.execute_with_timing("get_anchor_by_id", async {
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
        })
        .await
    }

    /// Retrieves an anchor by its Stellar account address.
    ///
    /// # Arguments
    ///
    /// * `stellar_account` - The Stellar public key (G-address) of the anchor
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Anchor))` - Anchor found with this account
    /// * `Ok(None)` - No anchor exists with this account
    /// * `Err(_)` - Database query failed
    ///
    /// # Examples
    ///
    /// ```rust
    /// let account = "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H";
    /// let anchor = db.get_anchor_by_stellar_account(account).await?;
    /// ```
    #[tracing::instrument(skip(self), fields(stellar_account = %stellar_account))]
    pub async fn get_anchor_by_stellar_account(
        &self,
        stellar_account: &str,
    ) -> Result<Option<Anchor>> {
        self.execute_with_timing("get_anchor_by_stellar_account", async {
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
        })
        .await
    }

    /// Lists all anchors with pagination, sorted by reliability score.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of anchors to return
    /// * `offset` - Number of anchors to skip (for pagination)
    ///
    #[tracing::instrument(skip(self), fields(limit = limit, offset = offset))]
    pub async fn list_anchors(&self, limit: i64, offset: i64) -> Result<Vec<Anchor>> {
        self.execute_with_timing("list_anchors", async {
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
        })
        .await
    }

    /// Updates anchor metrics and records history.
    #[tracing::instrument(skip(self, update), fields(anchor_id = %update.anchor_id))]
    pub async fn update_anchor_metrics(&self, update: AnchorMetricsUpdate) -> Result<Anchor> {
        self.execute_with_timing("update_anchor_metrics", async {
            // 1. Compute derived metrics (reliability, status, etc.)
            let metrics = compute_anchor_metrics(
                update.total_transactions,
                update.successful_transactions,
                update.failed_transactions,
                update.avg_settlement_time_ms,
            );

            // 2. Start a transaction to ensure atomic updates
            let mut tx = self.pool.begin().await.with_context(|| {
                format!(
                    "Failed to begin database transaction for anchor: {}",
                    update.anchor_id
                )
            })?;

            // 3. Update the main anchor record
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
            .bind(metrics.reliability_score)
            .bind(metrics.status.as_str())
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

            // 4. Record the entry in history table
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
                format!(
                    "Failed to record anchor metrics history for ID: {}",
                    update.anchor_id
                )
            })?;

            // 5. Commit the transaction
            tx.commit().await.with_context(|| {
                format!(
                    "Failed to commit anchor update transaction for ID: {}",
                    update.anchor_id
                )
            })?;

            Ok(anchor)
        })
        .await
    }
    // Asset operations

    /// Creates a new asset or updates existing asset's anchor association.
    ///
    /// Uses UPSERT logic: if an asset with the same code and issuer exists,
    /// updates its `anchor_id` and timestamp. Otherwise, creates a new asset.
    ///
    /// # Arguments
    ///
    /// * `anchor_id` - UUID of the anchor issuing this asset
    /// * `asset_code` - Asset code (e.g., "USDC", "XLM")
    /// * `asset_issuer` - Stellar public key of the asset issuer
    ///
    /// # Returns
    ///
    /// * `Ok(Asset)` - Created or updated asset
    /// * `Err(_)` - Database operation failed
    ///
    /// # Examples
    ///
    /// ```rust
    /// let asset = db.create_asset(
    ///     anchor_id,
    ///     "USDC".to_string(),
    ///     "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
    /// ).await?;
    /// ```
    ///
    /// # Side Effects
    ///
    /// - Updates `updated_at` timestamp on conflict
    /// - May reassign asset to different anchor if already exists
    pub async fn create_asset(
        &self,
        anchor_id: Uuid,
        asset_code: String,
        asset_issuer: String,
    ) -> Result<Asset> {
        self.execute_with_timing("create_asset", async {
            let id = Uuid::new_v4().to_string();
            let asset = sqlx::query_as::<_, Asset>(
                r"
            INSERT INTO assets (id, anchor_id, asset_code, asset_issuer)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (asset_code, asset_issuer) DO UPDATE
            SET anchor_id = EXCLUDED.anchor_id,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            ",
            )
            .bind(id)
            .bind(anchor_id.to_string())
            .bind(&asset_code)
            .bind(&asset_issuer)
            .fetch_one(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to create asset: code={}, issuer={}, anchor_id={}",
                    asset_code, asset_issuer, anchor_id
                )
            })?;
            Ok(asset)
        })
        .await
    }

    /// Retrieves all assets issued by a specific anchor.
    ///
    /// # Arguments
    ///
    /// * `anchor_id` - UUID of the anchor
    ///
    /// # Returns
    ///
    /// Vector of assets sorted alphabetically by `asset_code`.
    /// Returns empty vector if anchor has no assets.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let assets = db.get_assets_by_anchor(anchor_id).await?;
    /// for asset in assets {
    ///     println!("{}: {}", asset.asset_code, asset.asset_issuer);
    /// }
    /// ```
    pub async fn get_assets_by_anchor(&self, anchor_id: Uuid) -> Result<Vec<Asset>> {
        self.execute_with_timing("get_assets_by_anchor", async {
            let assets = sqlx::query_as::<_, Asset>(
                r"
            SELECT * FROM assets WHERE anchor_id = $1
            ORDER BY asset_code ASC
            ",
            )
            .bind(anchor_id.to_string())
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!("Failed to get assets for anchor_id: {}", anchor_id))?;
            Ok(assets)
        })
        .await
    }

    /// Retrieves assets for multiple anchors in a single query.
    ///
    /// Returns a `HashMap` mapping `anchor_id` to their list of assets.
    /// More efficient than calling `get_assets_by_anchor` multiple times.
    ///
    /// # Arguments
    ///
    /// * `anchor_ids` - Slice of anchor UUIDs to fetch assets for
    ///
    /// # Returns
    ///
    /// `HashMap` where keys are `anchor_id` strings and values are vectors of assets.
    /// Anchors with no assets are not included in the map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let anchor_ids = vec![anchor1_id, anchor2_id, anchor3_id];
    /// let assets_map = db.get_assets_by_anchors(&anchor_ids).await?;
    ///
    /// for (anchor_id, assets) in assets_map {
    ///     println!("Anchor {} has {} assets", anchor_id, assets.len());
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// Uses dynamic SQL with IN clause. Efficient for batch operations.
    /// Retrieves all anchors from the database.
    ///
    /// # Returns
    /// Vector of all anchors sorted by name alphabetically.
    #[tracing::instrument(skip(self))]
    pub async fn get_all_anchors(&self) -> Result<Vec<Anchor>> {
        self.execute_with_timing("get_all_anchors", async {
            let anchors = sqlx::query_as::<_, Anchor>("SELECT * FROM anchors ORDER BY name ASC")
                .fetch_all(&self.pool)
                .await?;
            Ok(anchors)
        })
        .await
    }

    /// Retrieves assets for multiple anchors in a single query.
    ///
    /// Returns empty `HashMap` if `anchor_ids` is empty.
    #[tracing::instrument(skip(self), fields(anchor_count = anchor_ids.len()))]
    pub async fn get_assets_by_anchors(
        &self,
        anchor_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<String, Vec<Asset>>> {
        if anchor_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let anchor_id_strs: Vec<String> = anchor_ids
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        // Build "?1, ?2, ?3, ..." with a single pre-allocated String to avoid
        // repeated small allocations from format! inside a collect+join chain.
        let mut placeholders = String::with_capacity(anchor_id_strs.len() * 4);
        for i in 0..anchor_id_strs.len() {
            if i > 0 {
                placeholders.push_str(", ");
            }
            use std::fmt::Write as _;
            let _ = write!(placeholders, "?{}", i + 1);
        }

        let query_str = format!(
            "SELECT * FROM assets WHERE anchor_id IN ({placeholders}) ORDER BY anchor_id, asset_code ASC"
        );

        let mut query = sqlx::query_as::<_, Asset>(&query_str);
        for id in &anchor_id_strs {
            query = query.bind(id);
        }

        let assets = query
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!("Failed to get assets for {} anchor ids", anchor_ids.len()))?;

        let mut result: std::collections::HashMap<String, Vec<Asset>> =
            std::collections::HashMap::new();
        for asset in assets {
            result
                .entry(asset.anchor_id.clone())
                .or_default()
                .push(asset);
        }

        Ok(result)
    }

    /// Counts the number of assets associated with a given anchor.
    #[tracing::instrument(skip(self), fields(anchor_id = %anchor_id))]
    pub async fn count_assets_by_anchor(&self, anchor_id: Uuid) -> Result<i64> {
        self.execute_with_timing("count_assets_by_anchor", async {
            let count: (i64,) = sqlx::query_as(
                r"
            SELECT COUNT(*) FROM assets WHERE anchor_id = $1
            ",
            )
            .bind(anchor_id.to_string())
            .fetch_one(&self.pool)
            .await
            .with_context(|| format!("Failed to count assets for anchor_id: {}", anchor_id))?;
            Ok(count.0)
        })
        .await
    }

    /// Update anchor metrics from RPC ingestion.
    #[tracing::instrument(skip(self, params), fields(stellar_account = %params.stellar_account))]
    pub async fn update_anchor_from_rpc(&self, params: AnchorRpcUpdate) -> Result<()> {
        self.execute_with_timing("update_anchor_from_rpc", async {
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
        })
        .await
    }

    // Metrics history operations
    
    /// Records a new metric point for an anchor's history.
    #[tracing::instrument(skip(self, params), fields(anchor_id = %params.anchor_id))]
    pub async fn record_anchor_metrics_history(
        &self,
        params: AnchorMetricsParams,
    ) -> Result<AnchorMetricsHistory> {
        self.execute_with_timing("record_anchor_metrics_history", async {
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
        })
        .await
    }

    /// Retrieves the most recent metrics history entries for an anchor.
    #[tracing::instrument(skip(self), fields(anchor_id = %anchor_id, limit = limit))]
    pub async fn get_anchor_metrics_history(
        &self,
        anchor_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AnchorMetricsHistory>> {
        self.execute_with_timing("get_anchor_metrics_history", async {
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
        })
        .await
    }

    /// Retrieves detailed information about an anchor, including its assets and metrics history.
    #[tracing::instrument(skip(self), fields(anchor_id = %anchor_id))]
    pub async fn get_anchor_detail(&self, anchor_id: Uuid) -> Result<Option<AnchorDetailResponse>> {
        let anchor = match self
            .get_anchor_by_id(anchor_id)
            .await
            .with_context(|| format!("Failed to fetch anchor for detail view: {}", anchor_id))?
        {
            Some(a) => a,
            None => return Ok(None),
        };

        let assets = self
            .get_assets_by_anchor(anchor_id)
            .await
            .with_context(|| format!("Failed to fetch assets for anchor detail: {}", anchor_id))?;
        let metrics_history = self
            .get_anchor_metrics_history(anchor_id, 30)
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch metrics history for anchor detail: {}",
                    anchor_id
                )
            })?;

        Ok(Some(AnchorDetailResponse {
            anchor,
            assets,
            metrics_history,
        }))
    }

    // Corridor operations
    #[tracing::instrument(skip(self, req), fields(source = %req.source_asset_code, dest = %req.dest_asset_code))]
    pub async fn create_corridor(
        &self,
        req: crate::models::CreateCorridorRequest,
    ) -> Result<crate::models::corridor::Corridor> {
        self.execute_with_timing("create_corridor", async {
            let corridor = crate::models::corridor::Corridor::new(
                req.source_asset_code,
                req.source_asset_issuer,
                req.dest_asset_code,
                req.dest_asset_issuer,
            );
            sqlx::query(
                r"
            INSERT INTO corridors (
                id, source_asset_code, source_asset_issuer,
                destination_asset_code, destination_asset_issuer
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer)
            DO UPDATE SET updated_at = CURRENT_TIMESTAMP
            ",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&corridor.source_asset_code)
            .bind(&corridor.source_asset_issuer)
            .bind(&corridor.destination_asset_code)
            .bind(&corridor.destination_asset_issuer)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Failed to create corridor: {}:{} -> {}:{}", corridor.source_asset_code, corridor.source_asset_issuer, corridor.destination_asset_code, corridor.destination_asset_issuer))?;
            Ok(corridor)
        })
        .await
    }

    /// Lists all corridors with pagination, sorted by reliability score.
    #[tracing::instrument(skip(self), fields(limit = limit, offset = offset))]
    pub async fn list_corridors(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::corridor::Corridor>> {
        self.execute_with_timing("list_corridors", async {
            let records = sqlx::query_as::<_, CorridorRecord>(
                r"
            SELECT * FROM corridors ORDER BY reliability_score DESC LIMIT $1 OFFSET $2
            ",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to list corridors (limit={}, offset={})",
                    limit, offset
                )
            })?;

            let corridors = records
                .into_iter()
                .map(|r| {
                    crate::models::corridor::Corridor::new(
                        r.source_asset_code,
                        r.source_asset_issuer,
                        r.destination_asset_code,
                        r.destination_asset_issuer,
                    )
                })
                .collect::<Vec<_>>();
            Ok(corridors)
        })
        .await
    }

    /// Retrieves a corridor by its ID.
    #[tracing::instrument(skip(self), fields(corridor_id = %id))]
    pub async fn get_corridor_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::corridor::Corridor>> {
        self.execute_with_timing("get_corridor_by_id", async {
            let record = sqlx::query_as::<_, CorridorRecord>(
                r"
            SELECT * FROM corridors WHERE id = $1
            ",
            )
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .with_context(|| format!("Failed to fetch corridor with id: {}", id))?;

            Ok(record.map(|r| {
                crate::models::corridor::Corridor::new(
                    r.source_asset_code,
                    r.source_asset_issuer,
                    r.destination_asset_code,
                    r.destination_asset_issuer,
                )
            }))
        })
        .await
    }

    /// Updates the metrics for a corridor.
    #[tracing::instrument(skip(self))]
    pub async fn update_corridor_metrics(
        &self,
        id: Uuid,
        metrics: crate::models::corridor::CorridorMetrics,
        cache: &CacheManager,
    ) -> Result<crate::models::corridor::Corridor> {
        self.execute_with_timing("update_corridor_metrics", async {
            let record = sqlx::query_as::<_, CorridorRecord>(
                r"
            UPDATE corridors
            SET reliability_score = $1,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            RETURNING *
            ",
            )
            .bind(metrics.success_rate)
            .bind(id.to_string())
            .fetch_one(&self.pool)
            .await
            .with_context(|| format!("Failed to update corridor metrics for id: {}", id))?;

            let corridor = crate::models::corridor::Corridor::new(
                record.source_asset_code,
                record.source_asset_issuer,
                record.destination_asset_code,
                record.destination_asset_issuer,
            );

            // Invalidate cache
            let corridor_key = corridor.to_string_key();
            let _ = cache.invalidate_corridor(&corridor_key).await.map_err(|e| {
                tracing::warn!(
                    "Failed to invalidate cache for corridor {}: {}",
                    corridor_key,
                    e
                );
            });

            Ok(corridor)
        })
        .await
    }

    // Generic Metric operations
    
    /// Records a generic system or entity metric.
    #[tracing::instrument(skip(self), fields(metric_name = %name))]
    pub async fn record_metric(
        &self,
        name: &str,
        value: f64,
        entity_id: Option<String>,
        entity_type: Option<String>,
    ) -> Result<MetricRecord> {
        self.execute_with_timing("record_metric", async {
            let id = Uuid::new_v4().to_string();
            let metric = sqlx::query_as::<_, MetricRecord>(
                r"
            INSERT INTO metrics (id, name, value, entity_id, entity_type, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            ",
            )
            .bind(id)
            .bind(name)
            .bind(value)
            .bind(entity_id.clone())
            .bind(entity_type)
            .bind(Utc::now())
            .fetch_one(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to record metric: name={}, entity_id={:?}",
                    name, entity_id
                )
            })?;
            Ok(metric)
        })
        .await
    }

    // Snapshot operations
    
    /// Creates a snapshot of an entity's state at a specific point in time or epoch.
    #[tracing::instrument(skip(self, data), fields(entity_id = %entity_id, entity_type = %entity_type))]
    pub async fn create_snapshot(
        &self,
        entity_id: &str,
        entity_type: &str,
        data: serde_json::Value,
        hash: Option<String>,
        epoch: Option<i64>,
    ) -> Result<SnapshotRecord> {
        self.execute_with_timing("create_snapshot", async {
            let id = Uuid::new_v4().to_string();
            let snapshot = sqlx::query_as::<_, SnapshotRecord>(
                r"
            INSERT INTO snapshots (id, entity_id, entity_type, data, hash, epoch, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            ",
            )
            .bind(id)
            .bind(entity_id)
            .bind(entity_type)
            .bind(data.to_string())
            .bind(hash)
            .bind(epoch)
            .bind(Utc::now())
            .fetch_one(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to create snapshot: entity_id={}, entity_type={}",
                    entity_id, entity_type
                )
            })?;
            Ok(snapshot)
        })
        .await
    }

    /// Retrieves a snapshot by its epoch number.
    #[tracing::instrument(skip(self), fields(epoch = epoch))]
    pub async fn get_snapshot_by_epoch(&self, epoch: i64) -> Result<Option<SnapshotRecord>> {
        self.execute_with_timing("get_snapshot_by_epoch", async {
            let snapshot = sqlx::query_as::<_, SnapshotRecord>(
                r"
            SELECT * FROM snapshots WHERE epoch = $1 LIMIT 1
            ",
            )
            .bind(epoch)
            .fetch_optional(&self.pool)
            .await
            .with_context(|| format!("Failed to fetch snapshot for epoch: {}", epoch))?;
            Ok(snapshot)
        })
        .await
    }

    /// Lists snapshots with pagination.
    #[tracing::instrument(skip(self), fields(limit = limit, offset = offset))]
    pub async fn list_snapshots(&self, limit: i64, offset: i64) -> Result<Vec<SnapshotRecord>> {
        self.execute_with_timing("list_snapshots", async {
            let snapshots = sqlx::query_as::<_, SnapshotRecord>(
                r"
            SELECT * FROM snapshots
            WHERE epoch IS NOT NULL
            ORDER BY epoch DESC
            LIMIT $1 OFFSET $2
            ",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to list snapshots (limit={}, offset={})",
                    limit, offset
                )
            })?;
            Ok(snapshots)
        })
        .await
    }

    // Ingestion methods
    
    /// Retrieves the last processed cursor for a given ingestion task.
    #[tracing::instrument(skip(self), fields(task_name = %task_name))]
    pub async fn get_ingestion_cursor(&self, task_name: &str) -> Result<Option<String>> {
        self.execute_with_timing("get_ingestion_cursor", async {
            let state = sqlx::query_as::<_, crate::models::IngestionState>(
                r"
            SELECT * FROM ingestion_state WHERE task_name = $1
            ",
            )
            .bind(task_name)
            .fetch_optional(&self.pool)
            .await
            .with_context(|| format!("Failed to get ingestion cursor for task: {}", task_name))?;
            Ok(state.map(|s| s.last_cursor))
        })
        .await
    }

    /// Updates the last processed cursor for a given ingestion task.
    #[tracing::instrument(skip(self), fields(task_name = %task_name))]
    pub async fn update_ingestion_cursor(&self, task_name: &str, last_cursor: &str) -> Result<()> {
        self.execute_with_timing("update_ingestion_cursor", async {
            sqlx::query(
                r"
            INSERT INTO ingestion_state (task_name, last_cursor, updated_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (task_name) DO UPDATE SET
                last_cursor = EXCLUDED.last_cursor,
                updated_at = EXCLUDED.updated_at
            ",
            )
            .bind(task_name)
            .bind(last_cursor)
            .bind(Utc::now())
            .execute(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to update ingestion cursor for task: {}, cursor: {}",
                    task_name, last_cursor
                )
            })?;
            Ok(())
        })
        .await
    }

    /// Saves a batch of payment records to the database using a transaction.
    #[tracing::instrument(skip(self, payments), fields(payment_count = payments.len()))]
    pub async fn save_payments(&self, payments: Vec<crate::models::PaymentRecord>) -> Result<()> {
        self.execute_with_timing("save_payments", async {
            let payment_count = payments.len();
            let mut tx = self.pool.begin().await.with_context(|| {
                format!("Failed to begin transaction for save_payments ({payment_count} payments)")
            })?;
            if payments.is_empty() {
                return Ok(());
            }

            let mut tx = self.pool.begin().await?;
            let mut tx = self.pool.begin().await.with_context(|| format!("Failed to begin transaction for save_payments ({} payments)", payments.len()))?;

            for payment in &payments {
                sqlx::query(
                    r"
                    INSERT INTO payments (
                        id, transaction_hash, source_account, destination_account,
                        asset_type, asset_code, asset_issuer, amount, created_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    ON CONFLICT (id) DO NOTHING
                    ",
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
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
                .await
                .with_context(|| format!("Failed to save payment id: {}", payment.id))?;
            }

            tx.commit().await.with_context(|| {
                format!("Failed to commit transaction for save_payments ({payment_count} payments)")
            })?;
            Ok(())
        })
        .await
    }

    // Aggregation methods
    
    /// Returns a handle to the aggregation database.
    #[must_use]
    pub fn aggregation_db(&self) -> crate::db::aggregation::AggregationDb {
        crate::db::aggregation::AggregationDb::new(self.pool.clone())
    }

    /// Fetches payments within a specific timerange with a limit.
    #[tracing::instrument(skip(self), fields(start_time = %start_time, end_time = %end_time, limit = limit))]
    pub async fn fetch_payments_by_timerange(
        &self,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<crate::models::corridor::PaymentRecord>> {
        self.execute_with_timing("fetch_payments_by_timerange", async {
            self.aggregation_db()
                .fetch_payments_by_timerange(start_time, end_time, limit)
                .await
                .context("Failed to fetch payments by timerange")
        })
        .await
    }

    /// Upserts hourly corridor metrics.
    #[tracing::instrument(skip(self, metric), fields(corridor_id = %metric.corridor_key, hour = %metric.hour_bucket))]
    pub async fn upsert_hourly_corridor_metric(
        &self,
        metric: &crate::models::corridor::HourlyCorridorMetrics,
    ) -> Result<()> {
        self.execute_with_timing("upsert_hourly_corridor_metric", async {
            self.aggregation_db()
                .upsert_hourly_corridor_metric(metric)
                .await
                .context("Failed to upsert hourly corridor metric")
        })
        .await
    }

    /// Fetches hourly metrics within a specific timerange.
    #[tracing::instrument(skip(self), fields(start_time = %start_time, end_time = %end_time))]
    pub async fn fetch_hourly_metrics_by_timerange(
        &self,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<crate::models::corridor::HourlyCorridorMetrics>> {
        self.execute_with_timing("fetch_hourly_metrics_by_timerange", async {
            self.aggregation_db()
                .fetch_hourly_metrics_by_timerange(start_time, end_time)
                .await
                .context("Failed to fetch hourly metrics by timerange")
        })
        .await
    }

    /// Creates a new aggregation job.
    #[tracing::instrument(skip(self), fields(job_id = %job_id, job_type = %job_type))]
    pub async fn create_aggregation_job(&self, job_id: &str, job_type: &str) -> Result<()> {
        self.execute_with_timing("create_aggregation_job", async {
            self.aggregation_db()
                .create_aggregation_job(job_id, job_type)
                .await
                .with_context(|| format!("Failed to create aggregation job for ID: {}", job_id))
        })
        .await
    }

    /// Updates the status of an aggregation job.
    #[tracing::instrument(skip(self), fields(job_id = %job_id, status = %status))]
    pub async fn update_aggregation_job_status(
        &self,
        job_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.execute_with_timing("update_aggregation_job_status", async {
            self.aggregation_db()
                .update_aggregation_job_status(job_id, status, error_message)
                .await
                .with_context(|| {
                    format!("Failed to update aggregation job status for ID: {}", job_id)
                })
        })
        .await
    }

    /// Updates the last processed hour for an aggregation job.
    #[tracing::instrument(skip(self), fields(job_id = %job_id, last_hour = %last_hour))]
    pub async fn update_last_processed_hour(&self, job_id: &str, last_hour: &str) -> Result<()> {
        self.execute_with_timing("update_last_processed_hour", async {
            self.aggregation_db()
                .update_last_processed_hour(job_id, last_hour)
                .await
                .with_context(|| {
                    format!(
                        "Failed to update last processed hour for job ID: {}",
                        job_id
                    )
                })
        })
        .await
    }

    /// Retrieves the retry count for an aggregation job.
    #[tracing::instrument(skip(self), fields(job_id = %job_id))]
    pub async fn get_job_retry_count(&self, job_id: &str) -> Result<i32> {
        self.execute_with_timing("get_job_retry_count", async {
            self.aggregation_db()
                .get_job_retry_count(job_id)
                .await
                .with_context(|| format!("Failed to get job retry count for ID: {}", job_id))
        })
        .await
    }

    /// Increments the retry count for an aggregation job.
    #[tracing::instrument(skip(self), fields(job_id = %job_id))]
    pub async fn increment_job_retry_count(&self, job_id: &str) -> Result<()> {
        self.execute_with_timing("increment_job_retry_count", async {
            self.aggregation_db()
                .increment_job_retry_count(job_id)
                .await
                .with_context(|| format!("Failed to increment job retry count for ID: {}", job_id))
        })
        .await
    }

    /// Muxed account analytics: counts and top addresses from payments table.
    /// Uses M-address detection (starts with 'M', length 69).
    #[tracing::instrument(skip(self), fields(top_limit = top_limit))]
    pub async fn get_muxed_analytics(&self, top_limit: i64) -> Result<MuxedAccountAnalytics> {
        self.execute_with_timing("get_muxed_analytics", async {
            use crate::muxed;
            const MUXED_LEN: i64 = 69;

            let total_muxed_payments = sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM payments
                WHERE (source_account LIKE 'M%' AND LENGTH(source_account) = $1)
                   OR (destination_account LIKE 'M%' AND LENGTH(destination_account) = $1)
                ",
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
                r"
                SELECT source_account AS addr, COUNT(*) AS cnt FROM payments
                WHERE source_account LIKE 'M%' AND LENGTH(source_account) = $1
                GROUP BY source_account
                ORDER BY cnt DESC
                LIMIT $2
                ",
            )
            .bind(MUXED_LEN)
            .bind(top_limit)
            .fetch_all(&self.pool)
            .await?;

            let dest_counts: Vec<AddrCount> = sqlx::query_as(
                r"
                SELECT destination_account AS addr, COUNT(*) AS cnt FROM payments
                WHERE destination_account LIKE 'M%' AND LENGTH(destination_account) = $1
                GROUP BY destination_account
                ORDER BY cnt DESC
                LIMIT $2
                ",
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
            
            let limit = std::cmp::max(0, top_limit) as usize;
            top_muxed_by_activity.truncate(limit);

            let unique_muxed_addresses = sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT addr) FROM (
                    SELECT source_account AS addr FROM payments WHERE source_account LIKE 'M%' AND LENGTH(source_account) = $1
                    UNION
                    SELECT destination_account AS addr FROM payments WHERE destination_account LIKE 'M%' AND LENGTH(destination_account) = $1
                )
                ",
            )
            .bind(MUXED_LEN)
            .fetch_one(&self.pool)
            .await?;


            let total_muxed_payments = sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM payments
                WHERE (source_account LIKE 'M%' AND LENGTH(source_account) = $1)
                   OR (destination_account LIKE 'M%' AND LENGTH(destination_account) = $1)
                ",
            )
            .bind(MUXED_LEN)
            .fetch_one(&self.pool)
            .await
            .context("Failed to count total muxed payments")?;

            #[derive(sqlx::FromRow)]
            struct AddrCount {
                addr: String,
                cnt: i64,
            }

            let source_counts: Vec<AddrCount> = sqlx::query_as(
                r"
                SELECT source_account AS addr, COUNT(*) AS cnt FROM payments
                WHERE source_account LIKE 'M%' AND LENGTH(source_account) = $1
                GROUP BY source_account
                ORDER BY cnt DESC
                LIMIT $2
                ",
            )
            .bind(MUXED_LEN)
            .bind(top_limit)
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!(
                "Failed to fetch top muxed source accounts (limit={})",
                top_limit
            ))?;

            let dest_counts: Vec<AddrCount> = sqlx::query_as(
                r"
                SELECT destination_account AS addr, COUNT(*) AS cnt FROM payments
                WHERE destination_account LIKE 'M%' AND LENGTH(destination_account) = $1
                GROUP BY destination_account
                ORDER BY cnt DESC
                LIMIT $2
                ",
            )
            .bind(MUXED_LEN)
            .bind(top_limit)
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!(
                "Failed to fetch top muxed destination accounts (limit={})",
                top_limit
            ))?;

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
                r"
                SELECT COUNT(DISTINCT addr) FROM (
                    SELECT source_account AS addr FROM payments WHERE source_account LIKE 'M%' AND LENGTH(source_account) = $1
                    UNION
                    SELECT destination_account AS addr FROM payments WHERE destination_account LIKE 'M%' AND LENGTH(destination_account) = $1
                )
                ",
            )
            .bind(MUXED_LEN)
            .fetch_one(&self.pool)
            .await
            .context("Failed to count unique muxed addresses")?;

            let base_accounts_with_muxed: Vec<String> = top_muxed_by_activity
                .iter()
                .filter_map(|u| u.base_account.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();

            Ok(MuxedAccountAnalytics {
                total_muxed_accounts: None,
                active_accounts: None,
                top_accounts: None,
                total_muxed_payments: Some(total_muxed_payments),
                unique_muxed_addresses: Some(unique_muxed_addresses),
                top_muxed_by_activity: Some(top_muxed_by_activity),
                base_accounts_with_muxed: Some(base_accounts_with_muxed),
            })
        })
        .await
    }

    // =========================

    /// Creates a new pending transaction with required signatures.
    #[tracing::instrument(skip(self, xdr), fields(source_account = %source_account))]
    pub async fn create_pending_transaction(
        &self,
        source_account: &str,
        xdr: &str,
        required_signatures: i32,
    ) -> Result<crate::models::PendingTransaction> {
        self.execute_with_timing("create_pending_transaction", async {
            let id = Uuid::new_v4().to_string();
            let pending_transaction = sqlx::query_as::<_, crate::models::PendingTransaction>(
                r"
            INSERT INTO pending_transactions (id, source_account, xdr, required_signatures, status)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            ",
            )
            .bind(&id)
            .bind(source_account)
            .bind(xdr)
            .bind(required_signatures)
            .bind("pending")
            .fetch_one(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to create pending transaction for source_account: {}",
                    source_account
                )
            })?;
            Ok(pending_transaction)
        })
        .await
    }


    pub async fn get_pending_transaction(
        &self,
        id: &str,
    ) -> Result<Option<crate::models::PendingTransactionWithSignatures>> {
        self.execute_with_timing("get_pending_transaction", async {
            let pending_transaction = sqlx::query_as::<_, crate::models::PendingTransaction>(
                r"
            SELECT * FROM pending_transactions WHERE id = $1
            ",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .with_context(|| format!("Failed to fetch pending transaction with id: {}", id))?;

            if let Some(transaction) = pending_transaction {
                let signatures = sqlx::query_as::<_, crate::models::Signature>(
                    r"
                SELECT * FROM transaction_signatures WHERE transaction_id = $1
                ",
                )
                .bind(id)
                .fetch_all(&self.pool)
                .await
                .with_context(|| {
                    format!("Failed to fetch signatures for transaction id: {}", id)
                })?;

                Ok(Some(crate::models::PendingTransactionWithSignatures {
                    transaction,
                    collected_signatures: signatures,
                }))
            } else {
                Ok(None)
            }
        })
        .await
    }

    /// Adds a signature to a pending transaction.
    #[tracing::instrument(skip(self, signature), fields(transaction_id = %transaction_id, signer = %signer))]
    pub async fn add_transaction_signature(
        &self,
        transaction_id: &str,
        signer: &str,
        signature: &str,
    ) -> Result<()> {
        self.execute_with_timing("add_transaction_signature", async {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                r"
            INSERT INTO transaction_signatures (id, transaction_id, signer, signature)
            VALUES ($1, $2, $3, $4)
            ",
            )
            .bind(id)
            .bind(transaction_id)
            .bind(signer)
            .bind(signature)
            .execute(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to add signature for transaction_id: {}, signer: {}",
                    transaction_id, signer
                )
            })?;
            Ok(())
        })
        .await
    }

    /// Updates the status of a pending transaction.
    #[tracing::instrument(skip(self), fields(transaction_id = %id, status = %status))]
    pub async fn update_transaction_status(&self, id: &str, status: &str) -> Result<()> {
        self.execute_with_timing("update_transaction_status", async {
            sqlx::query(
                r"
            UPDATE pending_transactions
            SET status = $1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            ",
            )
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to update transaction status to '{}' for id: {}",
                    status, id
                )
            })?;
            Ok(())
        })
        .await
    }

    // API Key operations

    /// Creates a new API key.
    #[tracing::instrument(skip(self, req), fields(wallet_address = %wallet_address, key_name = %req.name))]
    pub async fn create_api_key(
        &self,
        wallet_address: &str,
        req: CreateApiKeyRequest,
    ) -> Result<CreateApiKeyResponse> {
        self.execute_with_timing("create_api_key", async {
            let id = Uuid::new_v4().to_string();
            let (plain_key, prefix, key_hash) = generate_api_key();
            let scopes = req.scopes.unwrap_or_else(|| "read".to_string());
            let now = Utc::now().to_rfc3339();

            sqlx::query(
                r"
            INSERT INTO api_keys (id, name, key_prefix, key_hash, wallet_address, scopes, status, created_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, 'active', $7, $8)
            ",
            )
            .bind(&id)
            .bind(&req.name)
            .bind(&prefix)
            .bind(&key_hash)
            .bind(wallet_address)
            .bind(&scopes)
            .bind(&now)
            .bind(&req.expires_at)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Failed to insert API key for wallet: {}, name: {}", wallet_address, req.name))?;

            let key = sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE id = $1")
                .bind(&id)
                .fetch_one(&self.pool)
                .await
                .with_context(|| format!("Failed to fetch newly created API key with id: {}", id))?;

            Ok(CreateApiKeyResponse {
                key: ApiKeyInfo::from(key),
                plain_key,
            })
        })
        .await
    }

    /// Lists all active API keys for a given wallet address.
    #[tracing::instrument(skip(self), fields(wallet_address = %wallet_address))]
    pub async fn list_api_keys(&self, wallet_address: &str) -> Result<Vec<ApiKeyInfo>> {
        self.execute_with_timing("list_api_keys", async {
            let keys = sqlx::query_as::<_, ApiKey>(
                r"
            SELECT * FROM api_keys
            WHERE wallet_address = $1
            ORDER BY created_at DESC
            ",
            )
            .bind(wallet_address)
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!("Failed to list API keys for wallet: {}", wallet_address))?;
            Ok(keys.into_iter().map(ApiKeyInfo::from).collect())
        })
        .await
    }

    /// Retrieves an API key by its ID for a specific wallet address.
    #[tracing::instrument(skip(self), fields(key_id = %id, wallet_address = %wallet_address))]
    pub async fn get_api_key_by_id(
        &self,
        id: &str,
        wallet_address: &str,
    ) -> Result<Option<ApiKeyInfo>> {
        self.execute_with_timing("get_api_key_by_id", async {
            let key = sqlx::query_as::<_, ApiKey>(
                "SELECT * FROM api_keys WHERE id = $1 AND wallet_address = $2",
            )
            .bind(id)
            .bind(wallet_address)
            .fetch_optional(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to get API key id: {} for wallet: {}",
                    id, wallet_address
                )
            })?;
            Ok(key.map(ApiKeyInfo::from))
        })
        .await
    }

    pub async fn revoke_api_key(&self, id: &str, wallet_address: &str) -> Result<bool> {
        self.execute_with_timing("revoke_api_key", async {
            let revoked_at = Utc::now().to_rfc3339();
            let result = sqlx::query(
                r"
                UPDATE api_keys
                SET status = 'revoked', revoked_at = $1
                WHERE id = $2 AND wallet_address = $3 AND status = 'active'
                ",
            )
            .bind(&revoked_at)
            .bind(id)
            .bind(wallet_address)
            .execute(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to revoke API key id: {} for wallet: {}",
                    id, wallet_address
                )
            })?;

            Ok(result.rows_affected() > 0)
        })
        .await
    }

    pub async fn rotate_api_key(
        &self,
        id: &str,
        wallet_address: &str,
    ) -> Result<Option<CreateApiKeyResponse>> {
        self.execute_with_timing("rotate_api_key", async {
            let mut tx = self
                .pool
                .begin()
                .await
                .with_context(|| format!(
                    "Failed to begin API key rotation transaction for id: {}",
                    id
                ))?;

            let existing = sqlx::query_as::<_, ApiKey>(
                r"
                SELECT * FROM api_keys
                WHERE id = $1 AND wallet_address = $2 AND status = 'active'
                ",
            )
            .bind(id)
            .bind(wallet_address)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| format!(
                "Failed to fetch API key id: {} for wallet: {} during rotation",
                id, wallet_address
            ))?;

            let Some(existing) = existing else {
                return Ok(None);
            };

            let revoked_at = Utc::now().to_rfc3339();
            sqlx::query(
                r"
                UPDATE api_keys
                SET status = 'revoked', revoked_at = $1
                WHERE id = $2
                ",
            )
            .bind(&revoked_at)
            .bind(&existing.id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!(
                "Failed to revoke existing API key during rotation for id: {}",
                existing.id
            ))?;

            let new_id = Uuid::new_v4().to_string();
            let (plain_key, prefix, key_hash) = generate_api_key();
            let now = Utc::now().to_rfc3339();

            sqlx::query(
                r"
                INSERT INTO api_keys (
                    id, name, key_prefix, key_hash, wallet_address, scopes, status, created_at, expires_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, 'active', $7, $8)
                ",
            )
            .bind(&new_id)
            .bind(&existing.name)
            .bind(&prefix)
            .bind(&key_hash)
            .bind(wallet_address)
            .bind(&existing.scopes)
            .bind(&now)
            .bind(&existing.expires_at)
            .execute(&mut *tx)
            .await
            .with_context(|| format!(
                "Failed to insert rotated API key for wallet: {} from id: {}",
                wallet_address, existing.id
            ))?;

            let key = sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE id = $1")
                .bind(&new_id)
                .fetch_one(&mut *tx)
                .await
                .with_context(|| format!(
                    "Failed to fetch rotated API key with id: {}",
                    new_id
                ))?;

            tx.commit().await.with_context(|| {
                format!("Failed to commit API key rotation transaction for id: {}", id)
            })?;

            Ok(Some(CreateApiKeyResponse {
                key: ApiKeyInfo::from(key),
                plain_key,
            }))
        })
        .await
    }

    /// Validates an API key using the plaintext key.
    #[tracing::instrument(skip(self, plain_key))]
    pub async fn validate_api_key(&self, plain_key: &str) -> Result<Option<ApiKey>> {
        self.execute_with_timing("validate_api_key", async {
            let key_hash = hash_api_key(plain_key);

            let key = sqlx::query_as::<_, ApiKey>(
                "SELECT * FROM api_keys WHERE key_hash = $1 AND status = 'active'",
            )
            .bind(&key_hash)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to validate API key")?;

            if let Some(ref k) = key {
                if let Some(ref expires_at) = k.expires_at {
                    match DateTime::parse_from_rfc3339(expires_at) {
                        Ok(exp) => {
                            if exp < Utc::now() {
                                return Ok(None);
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "API key {} has malformed expires_at '{}': {}. Treating as expired.",
                                k.id,
                                expires_at,
                                e
                            );
                            return Ok(None);
                        }
                    }
                }


                // last_used_at update is best-effort; a failure here should not block validation
                if let Err(e) = sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE id = $2")

                // last_used_at update is best-effort
                let _ = sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE id = $2")

                    .bind(Utc::now().to_rfc3339())
                    .bind(&k.id)
                    .execute(&self.pool)
                    .await
                    .with_context(|| format!("Failed to update API key last_used_at timestamp for ID: {}", k.id));
            }

            Ok(key)
        })
        .await
    }

    /// Revokes an API key for a specific wallet address.
    #[tracing::instrument(skip(self), fields(key_id = %id, wallet_address = %wallet_address))]
    pub async fn revoke_api_key(&self, id: &str, wallet_address: &str) -> Result<bool> {
        self.execute_with_timing("revoke_api_key", async {
            let result = sqlx::query(
                r"
            UPDATE api_keys
            SET status = 'revoked', revoked_at = $1
            WHERE id = $2 AND wallet_address = $3 AND status = 'active'
            ",
            )
            .bind(Utc::now().to_rfc3339())
            .bind(id)
            .bind(wallet_address)
            .execute(&self.pool)
            .await?;
            Ok(result.rows_affected() > 0)
        })
        .await
    }

    /// Rotates an API key for a given wallet address.
    #[tracing::instrument(skip(self), fields(key_id = %id, wallet_address = %wallet_address))]
    pub async fn rotate_api_key(
        &self,
        id: &str,
        wallet_address: &str,
    ) -> Result<Option<CreateApiKeyResponse>> {
        let old_key = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE id = $1 AND wallet_address = $2 AND status = 'active'",
        )
        .bind(id)
        .bind(wallet_address)
        .fetch_optional(&self.pool)
        .await?;

        let old_key = match old_key {
            Some(k) => k,
            None => return Ok(None),
        };

        self.revoke_api_key(id, wallet_address).await?;

        let new_key = self
            .create_api_key(
                wallet_address,
                CreateApiKeyRequest {
                    name: old_key.name,
                    scopes: Some(old_key.scopes),
                    expires_at: old_key.expires_at,
                },
            )
            .await?;

        Ok(Some(new_key))
    }

    /// Retrieves the recent performance metrics for a specific anchor.
    #[tracing::instrument(skip(self), fields(anchor_id = %anchor_id, minutes = minutes))]
    pub async fn get_recent_anchor_performance(
        &self,
        anchor_id: &str,
        minutes: i64,
    ) -> Result<crate::models::AnchorMetrics> {
        self.execute_with_timing("get_recent_anchor_performance", async {
            let start_time = Utc::now() - chrono::Duration::minutes(minutes);
            
            // Query aggregates from the normalized payments table.
            // This table stores only successful payment operations, so success count equals total count.
            // In a real system, we'd join with anchors/assets to filter by anchor_id.
            

            // Query for aggregates from payments table
            // In a real system, we'd join with anchors/assets to filter by anchor_id

            let row: (i64, i64, Option<f64>) = sqlx::query_as(
                r"
                SELECT 
                    COUNT(*) as total,
                    COUNT(*) as successful,
                    AVG(amount) as avg_latency
                FROM payments
                WHERE (source_account = $1 OR destination_account = $2)
                AND created_at >= $3
                "
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
                reliability_score: success_rate, // Simple mapping
                total_transactions,
                successful_transactions,
                failed_transactions,
                avg_settlement_time_ms,
                status,
            })
        })
        .await
    }
}
