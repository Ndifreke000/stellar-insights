use async_graphql::*;
use sqlx::{QueryBuilder, SqlitePool};
use std::sync::Arc;

use super::types::*;

pub struct QueryRoot {
    pub pool: Arc<SqlitePool>,
}

#[Object]
impl QueryRoot {
    /// Get system health status
    async fn health(&self) -> HealthType {
        let db_status = match sqlx::query("SELECT 1")
            .fetch_one(self.pool.as_ref())
            .await
        {
            Ok(_) => "ok".to_string(),
            Err(_) => "error".to_string(),
        };

        HealthType {
            status: if db_status == "ok" { "ok" } else { "degraded" }.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            database: db_status,
            cache: "ok".to_string(),
            active_connections: 0,
        }
    }

    /// Get the total number of anchors in the system
    async fn anchor_count(&self) -> Result<i32> {
        let pool = &self.pool;
        let result: (i32,) = sqlx::query_as("SELECT COUNT(*) FROM anchors")
            .fetch_one(pool.as_ref())
            .await?;
        Ok(result.0)
    }

    /// Get a single anchor by ID
    async fn anchor(&self, id: String) -> Result<Option<AnchorType>> {
        let pool = &self.pool;
        let anchor = sqlx::query_as!(
            AnchorType,
            r#"
            SELECT
                id as "id!", name, stellar_account, home_domain,
                total_transactions as "total_transactions!",
                successful_transactions as "successful_transactions!",
                failed_transactions as "failed_transactions!",
                total_volume_usd as "total_volume_usd!",
                avg_settlement_time_ms as "avg_settlement_time_ms!",
                reliability_score as "reliability_score!",
                status as "status!",
                created_at as "created_at!: _", updated_at as "updated_at!: _"
            FROM anchors
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool.as_ref())
        .await?;
        Ok(anchor)
    }

    /// Get all anchors with optional filtering and pagination
    async fn anchors(
        &self,
        ctx: &Context<'_>,
        filter: Option<AnchorFilter>,
        pagination: Option<PaginationInput>,
    ) -> Result<AnchorsConnection> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(10)
            .min(100);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        let mut query_builder = QueryBuilder::new(
            "SELECT id, name, stellar_account, home_domain, total_transactions, successful_transactions, failed_transactions, total_volume_usd, avg_settlement_time_ms, reliability_score, status, created_at, updated_at FROM anchors WHERE 1=1"
        );

        if let Some(f) = &filter {
            if let Some(status) = &f.status {
                query_builder.push(" AND status = ");
                query_builder.push_bind(status);
            }
            if let Some(min_score) = f.min_reliability_score {
                query_builder.push(" AND reliability_score >= ");
                query_builder.push_bind(min_score);
            }
            if let Some(search) = &f.search {
                query_builder.push(" AND (name LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(" OR stellar_account LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(")");
            }
        }

        query_builder.push(" ORDER BY reliability_score DESC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let anchors = query_builder
            .build_query_as::<AnchorType>()
            .fetch_all(pool.as_ref())
            .await?;

        let mut count_builder = QueryBuilder::new("SELECT COUNT(*) as count FROM anchors WHERE 1=1");
        if let Some(f) = &filter {
            if let Some(status) = &f.status {
                count_builder.push(" AND status = ");
                count_builder.push_bind(status);
            }
            if let Some(min_score) = f.min_reliability_score {
                count_builder.push(" AND reliability_score >= ");
                count_builder.push_bind(min_score);
            }
            if let Some(search) = &f.search {
                count_builder.push(" AND (name LIKE ");
                count_builder.push_bind(format!("%{}%", search));
                count_builder.push(" OR stellar_account LIKE ");
                count_builder.push_bind(format!("%{}%", search));
                count_builder.push(")");
            }
        }

        let total: (i32,) = count_builder.build_query_as().fetch_one(pool.as_ref()).await?;

        Ok(AnchorsConnection {
            nodes: anchors,
            total_count: total.0,
            has_next_page: (offset + limit) < total.0,
        })
    }

    /// Get a single corridor by ID
    async fn corridor(&self, id: String) -> Result<Option<CorridorType>> {
        let pool = &self.pool;
        let corridor = sqlx::query_as!(
            CorridorType,
            r#"
            SELECT
                id as "id!", source_asset_code, source_asset_issuer,
                destination_asset_code, destination_asset_issuer,
                reliability_score as "reliability_score!", status as "status!",
                created_at as "created_at!: _", updated_at as "updated_at!: _"
            FROM corridors
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool.as_ref())
        .await?;
        Ok(corridor)
    }

    /// Get all corridors with optional filtering and pagination
    async fn corridors(
        &self,
        ctx: &Context<'_>,
        filter: Option<CorridorFilter>,
        pagination: Option<PaginationInput>,
    ) -> Result<CorridorsConnection> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(10)
            .min(100);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        let mut query_builder = QueryBuilder::new(
            "SELECT id, source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer, reliability_score, status, created_at, updated_at FROM corridors WHERE 1=1"
        );

        if let Some(f) = &filter {
            if let Some(source) = &f.source_asset_code {
                query_builder.push(" AND source_asset_code = ");
                query_builder.push_bind(source);
            }
            if let Some(dest) = &f.destination_asset_code {
                query_builder.push(" AND destination_asset_code = ");
                query_builder.push_bind(dest);
            }
            if let Some(status) = &f.status {
                query_builder.push(" AND status = ");
                query_builder.push_bind(status);
            }
            if let Some(min_score) = f.min_reliability_score {
                query_builder.push(" AND reliability_score >= ");
                query_builder.push_bind(min_score);
            }
        }

        query_builder.push(" ORDER BY reliability_score DESC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let corridors = query_builder
            .build_query_as::<CorridorType>()
            .fetch_all(pool.as_ref())
            .await?;

        let mut count_builder =
            QueryBuilder::new("SELECT COUNT(*) as count FROM corridors WHERE 1=1");
        if let Some(f) = &filter {
            if let Some(source) = &f.source_asset_code {
                count_builder.push(" AND source_asset_code = ");
                count_builder.push_bind(source);
            }
            if let Some(dest) = &f.destination_asset_code {
                count_builder.push(" AND destination_asset_code = ");
                count_builder.push_bind(dest);
            }
            if let Some(status) = &f.status {
                count_builder.push(" AND status = ");
                count_builder.push_bind(status);
            }
            if let Some(min_score) = f.min_reliability_score {
                count_builder.push(" AND reliability_score >= ");
                count_builder.push_bind(min_score);
            }
        }

        let total: (i32,) = count_builder
            .build_query_as()
            .fetch_one(pool.as_ref())
            .await?;

        Ok(CorridorsConnection {
            nodes: corridors,
            total_count: total.0,
            has_next_page: (offset + limit) < total.0,
        })
    }

    /// Get assets for a specific anchor
    async fn assets_by_anchor(&self, anchor_id: String) -> Result<Vec<AssetType>> {
        let pool = &self.pool;
        let assets = sqlx::query_as!(
            AssetType,
            r#"
            SELECT
                id as "id!", anchor_id, asset_code, asset_issuer,
                total_supply, num_holders as "num_holders!",
                created_at as "created_at!: _", updated_at as "updated_at!: _"
            FROM assets
            WHERE anchor_id = ?
            ORDER BY num_holders DESC
            "#,
            anchor_id
        )
        .fetch_all(pool.as_ref())
        .await?;
        Ok(assets)
    }

    /// Get metrics for an entity within a time range
    async fn metrics(
        &self,
        entity_id: Option<String>,
        entity_type: Option<String>,
        time_range: Option<TimeRangeInput>,
        pagination: Option<PaginationInput>,
    ) -> Result<Vec<MetricType>> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(100)
            .min(1000);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        let mut query_builder = QueryBuilder::new(
            "SELECT id, name, value, entity_id, entity_type, timestamp, created_at FROM metrics WHERE 1=1"
        );

        if let Some(eid) = &entity_id {
            query_builder.push(" AND entity_id = ");
            query_builder.push_bind(eid);
        }
        if let Some(etype) = &entity_type {
            query_builder.push(" AND entity_type = ");
            query_builder.push_bind(etype);
        }
        if let Some(tr) = &time_range {
            query_builder.push(" AND timestamp >= ");
            query_builder.push_bind(&tr.start);
            query_builder.push(" AND timestamp <= ");
            query_builder.push_bind(&tr.end);
        }

        query_builder.push(" ORDER BY timestamp DESC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let metrics = query_builder
            .build_query_as::<MetricType>()
            .fetch_all(pool.as_ref())
            .await?;

        Ok(metrics)
    }

    /// Get latest snapshot for an entity
    async fn latest_snapshot(
        &self,
        entity_id: String,
        entity_type: String,
    ) -> Result<Option<SnapshotType>> {
        let pool = &self.pool;
        let snapshot = sqlx::query_as!(
            SnapshotType,
            r#"
            SELECT
                id, entity_id as "entity_id!", entity_type as "entity_type!",
                data as "data!", hash, epoch,
                timestamp as "timestamp: _", created_at as "created_at!: _"
            FROM snapshots
            WHERE entity_id = ? AND entity_type = ?
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            entity_id,
            entity_type
        )
        .fetch_optional(pool.as_ref())
        .await?;
        Ok(snapshot)
    }

    /// Get paginated snapshots
    async fn snapshots(
        &self,
        pagination: Option<PaginationInput>,
    ) -> Result<SnapshotsConnection> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(10)
            .min(100);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        let nodes = sqlx::query_as!(
            SnapshotType,
            r#"
            SELECT
                id, entity_id as "entity_id!", entity_type as "entity_type!",
                data as "data!", hash, epoch,
                timestamp as "timestamp: _", created_at as "created_at!: _"
            FROM snapshots
            WHERE epoch IS NOT NULL
            ORDER BY epoch DESC
            LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(pool.as_ref())
        .await?;

        let total: (i32,) = sqlx::query_as("SELECT COUNT(*) FROM snapshots WHERE epoch IS NOT NULL")
            .fetch_one(pool.as_ref())
            .await?;

        Ok(SnapshotsConnection {
            nodes,
            total_count: total.0,
            has_next_page: (offset + limit) < total.0,
        })
    }

    /// Get a snapshot by epoch number
    async fn snapshot_by_epoch(&self, epoch: i64) -> Result<Option<SnapshotType>> {
        let pool = &self.pool;
        let snapshot = sqlx::query_as!(
            SnapshotType,
            r#"
            SELECT
                id, entity_id as "entity_id!", entity_type as "entity_type!",
                data as "data!", hash, epoch,
                timestamp as "timestamp: _", created_at as "created_at!: _"
            FROM snapshots
            WHERE epoch = ?
            LIMIT 1
            "#,
            epoch
        )
        .fetch_optional(pool.as_ref())
        .await?;
        Ok(snapshot)
    }

    /// Get payments with optional filtering and pagination
    async fn payments(
        &self,
        filter: Option<PaymentFilter>,
        pagination: Option<PaginationInput>,
    ) -> Result<PaymentsConnection> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(10)
            .min(100);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        let mut query_builder = QueryBuilder::new(
            "SELECT id, transaction_hash, source_account, destination_account, asset_type, asset_code, asset_issuer, source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer, amount, successful, timestamp, created_at FROM payments WHERE 1=1"
        );

        if let Some(f) = &filter {
            if let Some(src) = &f.source_account {
                query_builder.push(" AND source_account = ");
                query_builder.push_bind(src);
            }
            if let Some(dst) = &f.destination_account {
                query_builder.push(" AND destination_account = ");
                query_builder.push_bind(dst);
            }
            if let Some(code) = &f.asset_code {
                query_builder.push(" AND asset_code = ");
                query_builder.push_bind(code);
            }
            if let Some(success) = f.successful {
                query_builder.push(" AND successful = ");
                query_builder.push_bind(success);
            }
            if let Some(min) = f.min_amount {
                query_builder.push(" AND amount >= ");
                query_builder.push_bind(min);
            }
            if let Some(max) = f.max_amount {
                query_builder.push(" AND amount <= ");
                query_builder.push_bind(max);
            }
        }

        query_builder.push(" ORDER BY created_at DESC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let nodes = query_builder
            .build_query_as::<PaymentType>()
            .fetch_all(pool.as_ref())
            .await?;

        let mut count_builder =
            QueryBuilder::new("SELECT COUNT(*) as count FROM payments WHERE 1=1");
        if let Some(f) = &filter {
            if let Some(src) = &f.source_account {
                count_builder.push(" AND source_account = ");
                count_builder.push_bind(src);
            }
            if let Some(dst) = &f.destination_account {
                count_builder.push(" AND destination_account = ");
                count_builder.push_bind(dst);
            }
            if let Some(code) = &f.asset_code {
                count_builder.push(" AND asset_code = ");
                count_builder.push_bind(code);
            }
            if let Some(success) = f.successful {
                count_builder.push(" AND successful = ");
                count_builder.push_bind(success);
            }
            if let Some(min) = f.min_amount {
                count_builder.push(" AND amount >= ");
                count_builder.push_bind(min);
            }
            if let Some(max) = f.max_amount {
                count_builder.push(" AND amount <= ");
                count_builder.push_bind(max);
            }
        }

        let total: (i32,) = count_builder
            .build_query_as()
            .fetch_one(pool.as_ref())
            .await?;

        Ok(PaymentsConnection {
            nodes,
            total_count: total.0,
            has_next_page: (offset + limit) < total.0,
        })
    }

    /// Get liquidity pools with optional filtering and pagination
    async fn liquidity_pools(
        &self,
        filter: Option<LiquidityPoolFilter>,
        pagination: Option<PaginationInput>,
    ) -> Result<LiquidityPoolsConnection> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(10)
            .min(100);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        let mut query_builder = QueryBuilder::new(
            "SELECT pool_id, pool_type, fee_bp, total_trustlines, total_shares, reserve_a_asset_code, reserve_a_asset_issuer, reserve_a_amount, reserve_b_asset_code, reserve_b_asset_issuer, reserve_b_amount, total_value_usd, volume_24h_usd, fees_earned_24h_usd, apy, impermanent_loss_pct, trade_count_24h, last_synced_at, created_at, updated_at FROM liquidity_pools WHERE 1=1"
        );

        if let Some(f) = &filter {
            if let Some(code) = &f.asset_a_code {
                query_builder.push(" AND reserve_a_asset_code = ");
                query_builder.push_bind(code);
            }
            if let Some(code) = &f.asset_b_code {
                query_builder.push(" AND reserve_b_asset_code = ");
                query_builder.push_bind(code);
            }
            if let Some(min_apy) = f.min_apy {
                query_builder.push(" AND apy >= ");
                query_builder.push_bind(min_apy);
            }
            if let Some(min_val) = f.min_total_value_usd {
                query_builder.push(" AND total_value_usd >= ");
                query_builder.push_bind(min_val);
            }
        }

        query_builder.push(" ORDER BY total_value_usd DESC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let nodes = query_builder
            .build_query_as::<LiquidityPoolType>()
            .fetch_all(pool.as_ref())
            .await?;

        let mut count_builder =
            QueryBuilder::new("SELECT COUNT(*) as count FROM liquidity_pools WHERE 1=1");
        if let Some(f) = &filter {
            if let Some(code) = &f.asset_a_code {
                count_builder.push(" AND reserve_a_asset_code = ");
                count_builder.push_bind(code);
            }
            if let Some(code) = &f.asset_b_code {
                count_builder.push(" AND reserve_b_asset_code = ");
                count_builder.push_bind(code);
            }
            if let Some(min_apy) = f.min_apy {
                count_builder.push(" AND apy >= ");
                count_builder.push_bind(min_apy);
            }
            if let Some(min_val) = f.min_total_value_usd {
                count_builder.push(" AND total_value_usd >= ");
                count_builder.push_bind(min_val);
            }
        }

        let total: (i32,) = count_builder
            .build_query_as()
            .fetch_one(pool.as_ref())
            .await?;

        Ok(LiquidityPoolsConnection {
            nodes,
            total_count: total.0,
            has_next_page: (offset + limit) < total.0,
        })
    }

    /// Get aggregated liquidity pool statistics
    async fn liquidity_pool_stats(&self) -> Result<LiquidityPoolStatsType> {
        let pool = &self.pool;
        let result = sqlx::query_as!(
            LiquidityPoolStatsType,
            r#"
            SELECT
                COUNT(*) as "total_pools: _",
                COALESCE(SUM(total_value_usd), 0.0) as "total_liquidity_usd: _",
                COALESCE(AVG(total_value_usd), 0.0) as "avg_pool_size_usd: _",
                COALESCE(SUM(total_value_usd), 0.0) as "total_value_locked_usd: _",
                COALESCE(SUM(volume_24h_usd), 0.0) as "total_volume_24h_usd: _",
                COALESCE(SUM(fees_earned_24h_usd), 0.0) as "total_fees_24h_usd: _",
                COALESCE(AVG(apy), 0.0) as "avg_apy: _",
                COALESCE(AVG(impermanent_loss_pct), 0.0) as "avg_impermanent_loss: _"
            FROM liquidity_pools
            "#
        )
        .fetch_one(pool.as_ref())
        .await?;
        Ok(result)
    }

    /// Get trustline statistics with optional filtering and pagination
    async fn trustline_stats(
        &self,
        filter: Option<TrustlineFilter>,
        pagination: Option<PaginationInput>,
    ) -> Result<TrustlineStatsConnection> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(10)
            .min(100);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        let mut query_builder = QueryBuilder::new(
            "SELECT asset_code, asset_issuer, total_trustlines, authorized_trustlines, unauthorized_trustlines, total_supply, created_at, updated_at FROM trustline_stats WHERE 1=1"
        );

        if let Some(f) = &filter {
            if let Some(code) = &f.asset_code {
                query_builder.push(" AND asset_code = ");
                query_builder.push_bind(code);
            }
            if let Some(min_trustlines) = f.min_total_trustlines {
                query_builder.push(" AND total_trustlines >= ");
                query_builder.push_bind(min_trustlines);
            }
        }

        query_builder.push(" ORDER BY total_trustlines DESC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let nodes = query_builder
            .build_query_as::<TrustlineStatType>()
            .fetch_all(pool.as_ref())
            .await?;

        let mut count_builder =
            QueryBuilder::new("SELECT COUNT(*) as count FROM trustline_stats WHERE 1=1");
        if let Some(f) = &filter {
            if let Some(code) = &f.asset_code {
                count_builder.push(" AND asset_code = ");
                count_builder.push_bind(code);
            }
            if let Some(min_trustlines) = f.min_total_trustlines {
                count_builder.push(" AND total_trustlines >= ");
                count_builder.push_bind(min_trustlines);
            }
        }

        let total: (i32,) = count_builder
            .build_query_as()
            .fetch_one(pool.as_ref())
            .await?;

        Ok(TrustlineStatsConnection {
            nodes,
            total_count: total.0,
            has_next_page: (offset + limit) < total.0,
        })
    }

    /// Get aggregated trustline metrics
    async fn trustline_metrics(&self) -> Result<TrustlineMetricsType> {
        let pool = &self.pool;
        let result = sqlx::query_as!(
            TrustlineMetricsType,
            r#"
            SELECT
                COUNT(*) as "total_assets_tracked: _",
                COALESCE(SUM(total_trustlines), 0) as "total_trustlines_across_network: _",
                COUNT(CASE WHEN total_trustlines > 0 THEN 1 END) as "active_assets: _"
            FROM trustline_stats
            "#
        )
        .fetch_one(pool.as_ref())
        .await?;
        Ok(result)
    }

    /// Get trustline history for a specific asset
    async fn trustline_history(
        &self,
        asset_code: String,
        asset_issuer: String,
        pagination: Option<PaginationInput>,
    ) -> Result<Vec<TrustlineSnapshotType>> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(30)
            .min(365);

        let snapshots = sqlx::query_as!(
            TrustlineSnapshotType,
            r#"
            SELECT
                id, asset_code, asset_issuer, total_trustlines,
                authorized_trustlines, unauthorized_trustlines, total_supply,
                snapshot_at
            FROM trustline_snapshots
            WHERE asset_code = ? AND asset_issuer = ?
            ORDER BY snapshot_at DESC
            LIMIT ?
            "#,
            asset_code,
            asset_issuer,
            limit
        )
        .fetch_all(pool.as_ref())
        .await?;

        Ok(snapshots)
    }

    /// Get anchor metrics history
    async fn anchor_metrics_history(
        &self,
        anchor_id: String,
        pagination: Option<PaginationInput>,
    ) -> Result<Vec<AnchorMetricsHistoryType>> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(50)
            .max(1);

        let history = sqlx::query_as!(
            AnchorMetricsHistoryType,
            r#"
            SELECT
                id, anchor_id, timestamp, success_rate, failure_rate,
                reliability_score, total_transactions, successful_transactions,
                failed_transactions, avg_settlement_time_ms, volume_usd,
                created_at as "created_at: _"
            FROM anchor_metrics_history
            WHERE anchor_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
            anchor_id,
            limit
        )
        .fetch_all(pool.as_ref())
        .await?;

        Ok(history)
    }

    /// Search across anchors, corridors, and payments
    async fn search(
        &self,
        query: String,
        limit: Option<i32>,
    ) -> Result<SearchResults> {
        let pool = &self.pool;
        let search_limit = limit.unwrap_or(10).min(50);
        let search_pattern = format!("%{}%", query);

        let mut anchor_builder = QueryBuilder::new(
            "SELECT id, name, stellar_account, home_domain, total_transactions, successful_transactions, failed_transactions, total_volume_usd, avg_settlement_time_ms, reliability_score, status, created_at, updated_at FROM anchors WHERE name LIKE "
        );
        anchor_builder.push_bind(&search_pattern);
        anchor_builder.push(" OR stellar_account LIKE ");
        anchor_builder.push_bind(&search_pattern);
        anchor_builder.push(" LIMIT ");
        anchor_builder.push_bind(search_limit);

        let anchors = anchor_builder
            .build_query_as::<AnchorType>()
            .fetch_all(pool.as_ref())
            .await?;

        let mut corridor_builder = QueryBuilder::new(
            "SELECT id, source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer, reliability_score, status, created_at, updated_at FROM corridors WHERE source_asset_code LIKE "
        );
        corridor_builder.push_bind(&search_pattern);
        corridor_builder.push(" OR destination_asset_code LIKE ");
        corridor_builder.push_bind(&search_pattern);
        corridor_builder.push(" LIMIT ");
        corridor_builder.push_bind(search_limit);

        let corridors = corridor_builder
            .build_query_as::<CorridorType>()
            .fetch_all(pool.as_ref())
            .await?;

        let mut payment_builder = QueryBuilder::new(
            "SELECT id, transaction_hash, source_account, destination_account, asset_type, asset_code, asset_issuer, source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer, amount, successful, timestamp, created_at FROM payments WHERE source_account LIKE "
        );
        payment_builder.push_bind(&search_pattern);
        payment_builder.push(" OR destination_account LIKE ");
        payment_builder.push_bind(&search_pattern);
        payment_builder.push(" OR asset_code LIKE ");
        payment_builder.push_bind(&search_pattern);
        payment_builder.push(" LIMIT ");
        payment_builder.push_bind(search_limit);

        let payments = payment_builder
            .build_query_as::<PaymentType>()
            .fetch_all(pool.as_ref())
            .await?;

        Ok(SearchResults {
            anchors,
            corridors,
            payments,
        })
    }
}

// ── Mutations ─────────────────────────────────────────────────────────────────

pub struct MutationRoot {
    pub pool: Arc<SqlitePool>,
}

#[Object]
impl MutationRoot {
    /// Create a new anchor
    async fn create_anchor(
        &self,
        input: CreateAnchorInput,
    ) -> Result<CreateAnchorPayload> {
        if input.name.is_empty() || input.name.len() > 100 {
            return Err("Name must be between 1 and 100 characters".into());
        }
        if input.stellar_account.len() != 56 {
            return Err("Stellar account must be exactly 56 characters".into());
        }

        let pool = &self.pool;
        let id = uuid::Uuid::new_v4().to_string();

        let anchor = sqlx::query_as!(
            AnchorType,
            r#"
            INSERT INTO anchors (id, name, stellar_account, home_domain)
            VALUES (?, ?, ?, ?)
            RETURNING
                id, name, stellar_account, home_domain,
                total_transactions, successful_transactions, failed_transactions,
                total_volume_usd, avg_settlement_time_ms, reliability_score,
                status, created_at as "created_at: _", updated_at as "updated_at: _"
            "#,
            id,
            input.name,
            input.stellar_account,
            input.home_domain
        )
        .fetch_one(pool.as_ref())
        .await
        .map_err(|e| format!("Failed to create anchor: {}", e))?;

        Ok(CreateAnchorPayload {
            anchor,
            success: true,
            message: "Anchor created successfully".to_string(),
        })
    }

    /// Create a new corridor
    async fn create_corridor(
        &self,
        input: CreateCorridorInput,
    ) -> Result<CreateCorridorPayload> {
        if input.source_asset_code.is_empty() || input.source_asset_code.len() > 12 {
            return Err("Source asset code must be between 1 and 12 characters".into());
        }
        if input.source_asset_issuer.len() != 56 {
            return Err("Source asset issuer must be exactly 56 characters".into());
        }
        if input.destination_asset_code.is_empty() || input.destination_asset_code.len() > 12 {
            return Err("Destination asset code must be between 1 and 12 characters".into());
        }
        if input.destination_asset_issuer.len() != 56 {
            return Err("Destination asset issuer must be exactly 56 characters".into());
        }

        let pool = &self.pool;
        let id = uuid::Uuid::new_v4().to_string();

        let corridor = sqlx::query_as!(
            CorridorType,
            r#"
            INSERT INTO corridors (id, source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer)
            DO UPDATE SET updated_at = CURRENT_TIMESTAMP
            RETURNING
                id, source_asset_code, source_asset_issuer,
                destination_asset_code, destination_asset_issuer,
                reliability_score, status,
                created_at as "created_at: _", updated_at as "updated_at: _"
            "#,
            id,
            input.source_asset_code,
            input.source_asset_issuer,
            input.destination_asset_code,
            input.destination_asset_issuer
        )
        .fetch_one(pool.as_ref())
        .await
        .map_err(|e| format!("Failed to create corridor: {}", e))?;

        Ok(CreateCorridorPayload {
            corridor,
            success: true,
            message: "Corridor created successfully".to_string(),
        })
    }

    /// Update anchor metrics
    async fn update_anchor_metrics(
        &self,
        input: UpdateAnchorMetricsInput,
    ) -> Result<UpdateAnchorMetricsPayload> {
        let pool = &self.pool;

        // Compute reliability score: 100 * (successful / total) if total > 0
        let reliability_score = if input.total_transactions > 0 {
            (input.successful_transactions as f64 / input.total_transactions as f64) * 100.0
        } else {
            0.0
        };

        // Determine status based on reliability score
        let status = if reliability_score > 98.0 {
            "green"
        } else if reliability_score >= 95.0 {
            "yellow"
        } else {
            "red"
        };

        let anchor = sqlx::query_as!(
            AnchorType,
            r#"
            UPDATE anchors
            SET total_transactions = ?,
                successful_transactions = ?,
                failed_transactions = ?,
                avg_settlement_time_ms = COALESCE(?, avg_settlement_time_ms),
                reliability_score = ?,
                status = ?,
                total_volume_usd = COALESCE(?, total_volume_usd),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            RETURNING
                id, name, stellar_account, home_domain,
                total_transactions, successful_transactions, failed_transactions,
                total_volume_usd, avg_settlement_time_ms, reliability_score,
                status, created_at as "created_at: _", updated_at as "updated_at: _"
            "#,
            input.total_transactions,
            input.successful_transactions,
            input.failed_transactions,
            input.avg_settlement_time_ms,
            reliability_score,
            status,
            input.volume_usd,
            input.anchor_id
        )
        .fetch_optional(pool.as_ref())
        .await
        .map_err(|e| format!("Failed to update anchor metrics: {}", e))?
        .ok_or("Anchor not found")?;

        // Record metrics history
        let history_id = uuid::Uuid::new_v4().to_string();
        let _ = sqlx::query(
            r#"
            INSERT INTO anchor_metrics_history (id, anchor_id, success_rate, failure_rate, reliability_score, total_transactions, successful_transactions, failed_transactions, avg_settlement_time_ms, volume_usd)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&history_id)
        .bind(&input.anchor_id)
        .bind(reliability_score)
        .bind(100.0 - reliability_score)
        .bind(reliability_score)
        .bind(input.total_transactions)
        .bind(input.successful_transactions)
        .bind(input.failed_transactions)
        .bind(input.avg_settlement_time_ms)
        .bind(input.volume_usd)
        .execute(pool.as_ref())
        .await;

        Ok(UpdateAnchorMetricsPayload {
            anchor,
            success: true,
            message: "Anchor metrics updated successfully".to_string(),
        })
    }

    /// Delete an anchor by ID
    async fn delete_anchor(&self, id: String) -> Result<bool> {
        let pool = &self.pool;
        let result = sqlx::query("DELETE FROM anchors WHERE id = ?")
            .bind(&id)
            .execute(pool.as_ref())
            .await
            .map_err(|e| format!("Failed to delete anchor: {}", e))?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a corridor by ID
    async fn delete_corridor(&self, id: String) -> Result<bool> {
        let pool = &self.pool;
        let result = sqlx::query("DELETE FROM corridors WHERE id = ?")
            .bind(&id)
            .execute(pool.as_ref())
            .await
            .map_err(|e| format!("Failed to delete corridor: {}", e))?;

        Ok(result.rows_affected() > 0)
    }
}
