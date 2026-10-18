use anyhow::Result;
use failsafe::futures::CircuitBreaker as _;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tracing::info;

use crate::models::{TrustlineMetrics, TrustlineSnapshot, TrustlineStat};
use crate::rpc::{circuit_breaker::rpc_circuit_breaker, StellarRpcClient};

pub struct TrustlineAnalyzer {
    pool: Pool<Sqlite>,
    rpc_client: Arc<StellarRpcClient>,
}

impl TrustlineAnalyzer {
    #[must_use]
    pub const fn new(pool: Pool<Sqlite>, rpc_client: Arc<StellarRpcClient>) -> Self {
        Self { pool, rpc_client }
    }

    /// Spawn the background loop that keeps `trustline_stats` /
    /// `trustline_snapshots` fresh (#2666).
    ///
    /// Before this, nothing in the codebase called `sync_assets` or
    /// `take_snapshots` — `/api/v1/trustlines/stats`, `/rankings` and
    /// `/{asset_code}/{asset_issuer}/history` were all wired up and
    /// correctly implemented, but queried permanently empty tables, so the
    /// "trustline growth" chart data the issue asks for never had anything
    /// to show and could never "match Horizon counts" (the acceptance
    /// criterion) since no Horizon sync ever ran.
    ///
    /// Mirrors `CorridorPerformanceMonitor::spawn`'s pattern of two
    /// independently-ticking intervals in one task.
    pub fn spawn(
        self: &Arc<Self>,
        sync_interval_secs: u64,
        snapshot_interval_secs: u64,
    ) -> tokio::task::JoinHandle<()> {
        let analyzer = Arc::clone(self);
        tokio::spawn(async move {
            let mut sync_ticker =
                tokio::time::interval(tokio::time::Duration::from_secs(sync_interval_secs));
            let mut snapshot_ticker =
                tokio::time::interval(tokio::time::Duration::from_secs(snapshot_interval_secs));
            sync_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            snapshot_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            loop {
                tokio::select! {
                    _ = sync_ticker.tick() => {
                        if let Err(e) = analyzer.sync_assets().await {
                            tracing::error!(error = %e, "Trustline sync from Horizon failed");
                        }
                    }
                    _ = snapshot_ticker.tick() => {
                        if let Err(e) = analyzer.take_snapshots().await {
                            tracing::error!(error = %e, "Trustline snapshot failed");
                        }
                    }
                }
            }
        })
    }

    // ========================================================================
    // Sync from Horizon
    // ========================================================================

    /// Fetch top assets from Horizon and upsert trustline stats
    pub async fn sync_assets(&self) -> Result<u64> {
        info!("Starting trustline stats sync from Horizon...");
        let circuit_breaker = rpc_circuit_breaker();
        // Fetch top 200 assets (by rating)
        // Wrapped in circuit breaker for resilience
        let assets: Vec<crate::rpc::HorizonAsset> = circuit_breaker
            .call(async {
                self.rpc_client
                    .fetch_assets(200, true)
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))
            })
            .await
            .map_err(|e| match e {
                failsafe::Error::Rejected => anyhow::anyhow!("Circuit breaker open"),
                failsafe::Error::Inner(err) => err,
            })?;

        let mut synced_count = 0;
        let mut tx = self.pool.begin().await?;

        for asset in assets {
            // we only track alphanumeric assets
            if asset.asset_type == "native" {
                continue;
            }

            let total_trustlines = asset.accounts.authorized
                + asset.accounts.unauthorized
                + asset.accounts.authorized_to_maintain_liabilities;
            let total_supply: f64 = asset.balances.authorized.parse().unwrap_or(0.0);

            sqlx::query(
                r"
                INSERT INTO trustline_stats (
                    asset_code, asset_issuer, total_trustlines, authorized_trustlines, unauthorized_trustlines, total_supply, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
                ON CONFLICT(asset_code, asset_issuer) DO UPDATE SET
                    total_trustlines = excluded.total_trustlines,
                    authorized_trustlines = excluded.authorized_trustlines,
                    unauthorized_trustlines = excluded.unauthorized_trustlines,
                    total_supply = excluded.total_supply,
                    updated_at = CURRENT_TIMESTAMP
                ",
            )
            .bind(&asset.asset_code)
            .bind(&asset.asset_issuer)
            .bind(total_trustlines)
            .bind(asset.accounts.authorized)
            .bind(asset.accounts.unauthorized)
            .bind(total_supply)
            .execute(&mut *tx)
            .await?;

            synced_count += 1;
        }

        tx.commit().await?;
        info!("Successfully synced {} assets trustlines", synced_count);

        Ok(synced_count)
    }

    /// Take a daily snapshot of all assets for historical charting
    pub async fn take_snapshots(&self) -> Result<u64> {
        info!("Taking trustline snapshots...");

        // Simply copy current state from trustline_stats to trustline_snapshots
        let result = sqlx::query(
            r"
            INSERT INTO trustline_snapshots (
                asset_code, asset_issuer, total_trustlines, authorized_trustlines, unauthorized_trustlines, total_supply, snapshot_at
            )
            SELECT
                asset_code, asset_issuer, total_trustlines, authorized_trustlines, unauthorized_trustlines, total_supply, CURRENT_TIMESTAMP
            FROM
                trustline_stats
            "
        )
        .execute(&self.pool)
        .await?;

        info!("Took {} trustline snapshots", result.rows_affected());
        Ok(result.rows_affected())
    }

    // ========================================================================
    // Query Methods
    // ========================================================================

    /// Get overall trustline metrics across the network (tracked assets)
    pub async fn get_metrics(&self) -> Result<TrustlineMetrics> {
        use sqlx::Row;

        let row = sqlx::query(
            r"
            SELECT
                COUNT(*) as total_assets,
                SUM(total_trustlines) as network_trustlines
            FROM trustline_stats
            ",
        )
        .fetch_one(&self.pool)
        .await?;

        let total_assets: i64 = row.get("total_assets");
        let network_trustlines: Option<i64> = row.get("network_trustlines");

        Ok(TrustlineMetrics {
            total_assets_tracked: total_assets,
            total_trustlines_across_network: network_trustlines.unwrap_or(0),
            active_assets: total_assets,
        })
    }

    /// Retrieve the assets ordered by total trustlines
    pub async fn get_trustline_rankings(&self, limit: i64) -> Result<Vec<TrustlineStat>> {
        let rankings = sqlx::query_as::<_, TrustlineStat>(
            r"
            SELECT * FROM trustline_stats
            ORDER BY total_trustlines DESC
            LIMIT ?1
            ",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rankings)
    }

    /// Retrieves historical snapshot data for a given asset
    pub async fn get_asset_history(
        &self,
        asset_code: &str,
        asset_issuer: &str,
        limit: i64,
    ) -> Result<Vec<TrustlineSnapshot>> {
        let history = sqlx::query_as::<_, TrustlineSnapshot>(
            r"
            SELECT * FROM trustline_snapshots
            WHERE asset_code = ?1 AND asset_issuer = ?2
            ORDER BY snapshot_at DESC
            LIMIT ?3
            ",
        )
        .bind(asset_code)
        .bind(asset_issuer)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(history)
    }
}
