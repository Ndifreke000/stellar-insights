use anyhow::Result;
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tracing::info;

use crate::models::{LiquidityPool, LiquidityPoolSnapshot, LiquidityPoolStats};
use crate::rpc::StellarRpcClient;

pub struct LiquidityPoolAnalyzer {
    pool: Pool<Sqlite>,
    rpc_client: Arc<StellarRpcClient>,
}

impl LiquidityPoolAnalyzer {
    pub fn new(pool: Pool<Sqlite>, rpc_client: Arc<StellarRpcClient>) -> Self {
        Self { pool, rpc_client }
    }

    // ========================================================================
    // Sync from Horizon
    // ========================================================================

    /// Fetch liquidity pools from Horizon and upsert into the database.
    /// Returns the number of pools synced.
    pub async fn sync_pools(&self) -> Result<u64> {
        let horizon_pools = self.rpc_client.fetch_liquidity_pools(50, None).await?;
        let mut count = 0u64;

        for hp in &horizon_pools {
            let (asset_a_code, asset_a_issuer) = Self::parse_asset(&hp.reserves[0].asset);
            let (asset_b_code, asset_b_issuer) = Self::parse_asset(&hp.reserves[1].asset);
            let reserve_a: f64 = hp.reserves[0].amount.parse().unwrap_or(0.0);
            let reserve_b: f64 = hp.reserves[1].amount.parse().unwrap_or(0.0);

            // Estimate total value (simplified: assume both sides equivalent for AMM)
            let total_value_usd = reserve_a + reserve_b; // Simplified valuation

            // Compute volume from recent trades
            let trades = self
                .rpc_client
                .fetch_pool_trades(&hp.id, 100)
                .await
                .unwrap_or_default();
            let volume_24h_usd: f64 = trades
                .iter()
                .map(|t| {
                    t.base_amount.parse::<f64>().unwrap_or(0.0)
                        + t.counter_amount.parse::<f64>().unwrap_or(0.0)
                })
                .sum();

            let trade_count_24h = trades.len() as i32;

            // Compute fees earned (fee_bp basis points applied to volume)
            let fee_rate = hp.fee_bp as f64 / 10_000.0;
            let fees_earned_24h = volume_24h_usd * fee_rate;

            // Compute APY: annualize daily fees relative to TVL
            let apy = if total_value_usd > 0.0 {
                (fees_earned_24h / total_value_usd) * 365.0 * 100.0
            } else {
                0.0
            };

            // Compute impermanent loss (requires initial reserves, use snapshot if available)
            let il = self
                .compute_impermanent_loss_for_pool(&hp.id, reserve_a, reserve_b)
                .await;

            let now = Utc::now();

            sqlx::query(
                r#"
                INSERT INTO liquidity_pools (
                    pool_id, pool_type, fee_bp, total_trustlines, total_shares,
                    reserve_a_asset_code, reserve_a_asset_issuer, reserve_a_amount,
                    reserve_b_asset_code, reserve_b_asset_issuer, reserve_b_amount,
                    total_value_usd, volume_24h_usd, fees_earned_24h_usd, apy,
                    impermanent_loss_pct, trade_count_24h, last_synced_at, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
                ON CONFLICT (pool_id) DO UPDATE SET
                    total_trustlines = excluded.total_trustlines,
                    total_shares = excluded.total_shares,
                    reserve_a_amount = excluded.reserve_a_amount,
                    reserve_b_amount = excluded.reserve_b_amount,
                    total_value_usd = excluded.total_value_usd,
                    volume_24h_usd = excluded.volume_24h_usd,
                    fees_earned_24h_usd = excluded.fees_earned_24h_usd,
                    apy = excluded.apy,
                    impermanent_loss_pct = excluded.impermanent_loss_pct,
                    trade_count_24h = excluded.trade_count_24h,
                    last_synced_at = excluded.last_synced_at,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&hp.id)
            .bind(&hp.pool_type)
            .bind(hp.fee_bp as i32)
            .bind(hp.total_trustlines as i32)
            .bind(&hp.total_shares)
            .bind(&asset_a_code)
            .bind(&asset_a_issuer)
            .bind(reserve_a)
            .bind(&asset_b_code)
            .bind(&asset_b_issuer)
            .bind(reserve_b)
            .bind(total_value_usd)
            .bind(volume_24h_usd)
            .bind(fees_earned_24h)
            .bind(apy)
            .bind(il)
            .bind(trade_count_24h)
            .bind(now)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await?;

            count += 1;
        }

        if count > 0 {
            info!("Synced {} liquidity pools from Horizon", count);
        }

        Ok(count)
    }

    /// Take a snapshot of all current pools for historical tracking
    pub async fn take_snapshots(&self) -> Result<u64> {
        let pools = self.get_all_pools().await?;
        let mut count = 0u64;
        let now = Utc::now();

        for pool in &pools {
            sqlx::query(
                r#"
                INSERT INTO liquidity_pool_snapshots (
                    pool_id, reserve_a_amount, reserve_b_amount, total_value_usd,
                    volume_usd, fees_usd, apy, impermanent_loss_pct, trade_count, snapshot_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#,
            )
            .bind(&pool.pool_id)
            .bind(pool.reserve_a_amount)
            .bind(pool.reserve_b_amount)
            .bind(pool.total_value_usd)
            .bind(pool.volume_24h_usd)
            .bind(pool.fees_earned_24h_usd)
            .bind(pool.apy)
            .bind(pool.impermanent_loss_pct)
            .bind(pool.trade_count_24h)
            .bind(now)
            .execute(&self.pool)
            .await?;
            count += 1;
        }

        if count > 0 {
            info!("Created {} liquidity pool snapshots", count);
        }
        Ok(count)
    }

    // ========================================================================
    // Query Methods
    // ========================================================================

    /// Get all pools from the database
    pub async fn get_all_pools(&self) -> Result<Vec<LiquidityPool>> {
        let pools = sqlx::query_as::<_, LiquidityPool>(
            "SELECT * FROM liquidity_pools ORDER BY total_value_usd DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(pools)
    }

    /// Get a single pool by ID with its historical snapshots
    pub async fn get_pool_detail(
        &self,
        pool_id: &str,
    ) -> Result<(LiquidityPool, Vec<LiquidityPoolSnapshot>)> {
        let pool =
            sqlx::query_as::<_, LiquidityPool>("SELECT * FROM liquidity_pools WHERE pool_id = $1")
                .bind(pool_id)
                .fetch_one(&self.pool)
                .await?;

        let snapshots = self.get_pool_snapshots(pool_id, 100).await?;

        Ok((pool, snapshots))
    }

    /// Get pool snapshots for historical charts
    pub async fn get_pool_snapshots(
        &self,
        pool_id: &str,
        limit: i64,
    ) -> Result<Vec<LiquidityPoolSnapshot>> {
        let snapshots = sqlx::query_as::<_, LiquidityPoolSnapshot>(
            r#"
            SELECT * FROM liquidity_pool_snapshots
            WHERE pool_id = $1
            ORDER BY snapshot_at DESC
            LIMIT $2
            "#,
        )
        .bind(pool_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(snapshots)
    }

    /// Get pools ranked by a specific metric
    pub async fn get_pool_rankings(&self, sort_by: &str, limit: i64) -> Result<Vec<LiquidityPool>> {
        let order_clause = match sort_by {
            "apy" => "apy DESC",
            "volume" => "volume_24h_usd DESC",
            "fees" => "fees_earned_24h_usd DESC",
            "tvl" => "total_value_usd DESC",
            "il" => "impermanent_loss_pct ASC",
            _ => "apy DESC",
        };

        let query = format!(
            "SELECT * FROM liquidity_pools ORDER BY {} LIMIT $1",
            order_clause
        );

        let pools = sqlx::query_as::<_, LiquidityPool>(&query)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        Ok(pools)
    }

    /// Get aggregate pool statistics
    pub async fn get_pool_stats(&self) -> Result<LiquidityPoolStats> {
        let row: (i64, f64, f64, f64, f64, f64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total_pools,
                COALESCE(SUM(total_value_usd), 0.0) as total_tvl,
                COALESCE(SUM(volume_24h_usd), 0.0) as total_volume,
                COALESCE(SUM(fees_earned_24h_usd), 0.0) as total_fees,
                COALESCE(AVG(apy), 0.0) as avg_apy,
                COALESCE(AVG(impermanent_loss_pct), 0.0) as avg_il
            FROM liquidity_pools
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(LiquidityPoolStats {
            total_pools: row.0,
            total_value_locked_usd: row.1,
            total_volume_24h_usd: row.2,
            total_fees_24h_usd: row.3,
            avg_apy: row.4,
            avg_impermanent_loss: row.5,
        })
    }

    // ========================================================================
    // Computation Helpers
    // ========================================================================

    /// Compute impermanent loss given initial and current reserves.
    /// IL = 2 * sqrt(price_ratio) / (1 + price_ratio) - 1
    /// where price_ratio = (current_a/current_b) / (initial_a/initial_b)
    pub fn compute_impermanent_loss(
        initial_a: f64,
        initial_b: f64,
        current_a: f64,
        current_b: f64,
    ) -> f64 {
        if initial_a <= 0.0 || initial_b <= 0.0 || current_a <= 0.0 || current_b <= 0.0 {
            return 0.0;
        }

        let initial_ratio = initial_a / initial_b;
        let current_ratio = current_a / current_b;
        let price_ratio = current_ratio / initial_ratio;

        let sqrt_ratio = price_ratio.sqrt();
        let il = 2.0 * sqrt_ratio / (1.0 + price_ratio) - 1.0;

        // IL is typically negative (representing loss), return as positive percentage
        (il.abs()) * 100.0
    }

    /// Look up the earliest snapshot for a pool to use as "initial" reserves
    async fn compute_impermanent_loss_for_pool(
        &self,
        pool_id: &str,
        current_a: f64,
        current_b: f64,
    ) -> f64 {
        let initial = sqlx::query_as::<_, (f64, f64)>(
            r#"
            SELECT reserve_a_amount, reserve_b_amount
            FROM liquidity_pool_snapshots
            WHERE pool_id = $1
            ORDER BY snapshot_at ASC
            LIMIT 1
            "#,
        )
        .bind(pool_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        match initial {
            Some((initial_a, initial_b)) => {
                Self::compute_impermanent_loss(initial_a, initial_b, current_a, current_b)
            }
            None => 0.0, // No historical data yet
        }
    }

    /// Parse a Horizon asset string ("native" or "CODE:ISSUER")
    fn parse_asset(asset_str: &str) -> (String, Option<String>) {
        if asset_str == "native" {
            ("XLM".to_string(), None)
        } else if let Some((code, issuer)) = asset_str.split_once(':') {
            (code.to_string(), Some(issuer.to_string()))
        } else {
            (asset_str.to_string(), None)
        }
    }
}
