use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use sqlx::{Pool, Sqlite};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tracing::info;

use crate::models::{
    CohortWeekRetention, TrustlineCohortMatrix, TrustlineCohortRetention, TrustlineMetrics,
    TrustlineSnapshot, TrustlineStat,
};
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
            .map_err(crate::rpc::CallError::into_anyhow)?;

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

    // ========================================================================
    // Cohorts, Churn, and Spam Management (#2819)
    // ========================================================================

    /// Record a trustline event (create, remove, balance_zero, balance_positive)
    pub async fn record_event(
        &self,
        account_id: &str,
        asset_code: &str,
        asset_issuer: &str,
        event_type: &str,
        balance: f64,
        recorded_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let ts = recorded_at.unwrap_or_else(Utc::now);
        sqlx::query(
            r"
            INSERT INTO trustline_events (
                account_id, asset_code, asset_issuer, event_type, balance, recorded_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
        )
        .bind(account_id)
        .bind(asset_code)
        .bind(asset_issuer)
        .bind(event_type)
        .bind(balance)
        .bind(ts)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark an account as known spam to exclude from retention analytics
    pub async fn flag_spam_account(&self, account_id: &str, reason: &str) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO trustline_spam_accounts (account_id, reason, flagged_at)
            VALUES (?1, ?2, CURRENT_TIMESTAMP)
            ON CONFLICT(account_id) DO UPDATE SET
                reason = excluded.reason,
                flagged_at = CURRENT_TIMESTAMP
            ",
        )
        .bind(account_id)
        .bind(reason)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if an account is flagged as spam
    pub async fn is_spam_account(&self, account_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM trustline_spam_accounts WHERE account_id = ?1",
        )
        .bind(account_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Compute cohort retention matrix for an asset excluding known spam accounts.
    ///
    /// Cohorts are computed weekly. An account belongs to the cohort week of its earliest
    /// trustline creation. For each subsequent week N+k, an account is considered retained
    /// if its latest state at or before the end of week N+k had balance > 0 and was not removed.
    pub async fn get_cohort_retention(
        &self,
        asset_code: &str,
        asset_issuer: &str,
        max_cohort_weeks: i64,
    ) -> Result<TrustlineCohortMatrix> {
        use sqlx::Row;

        // Count excluded spam accounts for this asset
        let spam_count: i64 = sqlx::query_scalar(
            r"
            SELECT COUNT(DISTINCT e.account_id)
            FROM trustline_events e
            INNER JOIN trustline_spam_accounts s ON e.account_id = s.account_id
            WHERE e.asset_code = ?1 AND e.asset_issuer = ?2
            ",
        )
        .bind(asset_code)
        .bind(asset_issuer)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        // Fetch all non-spam events for this asset ordered chronologically
        let rows = sqlx::query(
            r"
            SELECT e.account_id, e.event_type, e.balance, e.recorded_at
            FROM trustline_events e
            LEFT JOIN trustline_spam_accounts s ON e.account_id = s.account_id
            WHERE e.asset_code = ?1 AND e.asset_issuer = ?2 AND s.account_id IS NULL
            ORDER BY e.recorded_at ASC
            ",
        )
        .bind(asset_code)
        .bind(asset_issuer)
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(TrustlineCohortMatrix {
                asset_code: asset_code.to_string(),
                asset_issuer: asset_issuer.to_string(),
                total_tracked_accounts: 0,
                spam_accounts_excluded: spam_count,
                cohorts: Vec::new(),
            });
        }

        // Account event struct for in-memory cohort processing
        struct AccountEvent {
            account_id: String,
            event_type: String,
            balance: f64,
            recorded_at: DateTime<Utc>,
        }

        let mut events: Vec<AccountEvent> = Vec::with_capacity(rows.len());
        let mut distinct_accounts = HashSet::new();

        for row in rows {
            let account_id: String = row.get("account_id");
            let event_type: String = row.get("event_type");
            let balance: f64 = row.get("balance");
            let recorded_at: DateTime<Utc> = row.get("recorded_at");

            distinct_accounts.insert(account_id.clone());
            events.push(AccountEvent {
                account_id,
                event_type,
                balance,
                recorded_at,
            });
        }

        // 1. Determine each account's creation time and cohort week
        // Cohort week format: "YYYY-Www"
        let mut account_creation: HashMap<String, DateTime<Utc>> = HashMap::new();
        for ev in &events {
            account_creation
                .entry(ev.account_id.clone())
                .or_insert(ev.recorded_at);
        }

        // Group accounts by cohort week start (Monday 00:00:00 UTC)
        // Map: cohort_week_string -> (cohort_start_datetime, Vec<account_id>)
        let mut cohort_groups: BTreeMap<String, (DateTime<Utc>, Vec<String>)> = BTreeMap::new();
        for (account, created_at) in &account_creation {
            let iso_week = created_at.iso_week();
            let week_key = format!("{}-W{:02}", iso_week.year(), iso_week.week());

            // Week start timestamp (Monday 00:00:00 UTC of that week)
            let days_since_mon = created_at.weekday().num_days_from_monday();
            let week_start = *created_at - Duration::days(i64::from(days_since_mon));
            let week_start_truncated = Utc
                .with_ymd_and_hms(
                    week_start.year(),
                    week_start.month(),
                    week_start.day(),
                    0,
                    0,
                    0,
                )
                .single()
                .unwrap_or(week_start);

            cohort_groups
                .entry(week_key)
                .or_insert_with(|| (week_start_truncated, Vec::new()))
                .1
                .push(account.clone());
        }

        let mut cohorts_retention: Vec<TrustlineCohortRetention> = Vec::new();
        let max_weeks = max_cohort_weeks.clamp(1, 52);

        for (cohort_week, (week_start, accounts)) in
            cohort_groups.into_iter().rev().take(max_weeks as usize)
        {
            let cohort_size = accounts.len() as i64;
            if cohort_size == 0 {
                continue;
            }

            let mut retention_weeks: Vec<CohortWeekRetention> = Vec::new();
            let now = Utc::now();
            let num_weeks_passed = ((now - week_start).num_days() / 7).max(0);

            for k in 0..=num_weeks_passed.min(12) {
                let week_end = week_start + Duration::days((k + 1) * 7);

                // An account in this cohort is active at week N+k if its latest event <= week_end
                // has balance > 0 and event_type != "remove"
                let mut active_count = 0i64;

                for acct in &accounts {
                    // Find latest event for this account <= week_end
                    let latest_ev = events
                        .iter()
                        .filter(|e| &e.account_id == acct && e.recorded_at <= week_end)
                        .last();

                    if let Some(ev) = latest_ev {
                        if ev.event_type != "remove" && ev.balance > 0.0 {
                            active_count += 1;
                        }
                    } else {
                        // If no event recorded before week_end, check if created before or equal
                        if let Some(created) = account_creation.get(acct) {
                            if created <= &week_end {
                                active_count += 1;
                            }
                        }
                    }
                }

                let retention_rate = (active_count as f64 / cohort_size as f64) * 100.0;
                let churn_rate = (100.0 - retention_rate).max(0.0);

                retention_weeks.push(CohortWeekRetention {
                    week_number: k as i32,
                    active_accounts: active_count,
                    retention_rate: (retention_rate * 100.0).round() / 100.0,
                    churn_rate: (churn_rate * 100.0).round() / 100.0,
                });
            }

            cohorts_retention.push(TrustlineCohortRetention {
                cohort_week,
                cohort_size,
                retention: retention_weeks,
            });
        }

        Ok(TrustlineCohortMatrix {
            asset_code: asset_code.to_string(),
            asset_issuer: asset_issuer.to_string(),
            total_tracked_accounts: distinct_accounts.len() as i64,
            spam_accounts_excluded: spam_count,
            cohorts: cohorts_retention,
        })
    }
}
