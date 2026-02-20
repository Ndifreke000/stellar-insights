use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tracing::info;

use crate::models::{ClaimableBalance, ClaimableBalanceAnalytics, TopAssetClaimable};
use crate::rpc::StellarRpcClient;

pub struct ClaimableBalanceTracker {
    pool: Pool<Sqlite>,
    rpc_client: Arc<StellarRpcClient>,
}

impl ClaimableBalanceTracker {
    pub fn new(pool: Pool<Sqlite>, rpc_client: Arc<StellarRpcClient>) -> Self {
        Self { pool, rpc_client }
    }

    /// Parse Horizon asset string ("native" or "CODE:ISSUER") into (code, issuer)
    fn parse_asset(asset: &str) -> (String, Option<String>) {
        if asset == "native" {
            return ("XLM".to_string(), None);
        }
        if let Some((code, issuer)) = asset.split_once(':') {
            return (code.to_string(), Some(issuer.to_string()));
        }
        (asset.to_string(), None)
    }

    /// Extract expiration from claimant predicate (abs_before or abs_before_epoch)
    fn extract_expires_at(predicate: &serde_json::Value) -> Option<DateTime<Utc>> {
        if let Some(s) = predicate.get("abs_before").and_then(|v| v.as_str()) {
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Some(dt.with_timezone(&Utc));
            }
        }
        if let Some(epoch) = predicate
            .get("abs_before_epoch")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i64>().ok())
        {
            return Some(Utc.timestamp_opt(epoch, 0).single()?);
        }
        // Check nested and/or/not for abs_before
        for key in &["and", "or"] {
            if let Some(arr) = predicate.get(key).and_then(|v| v.as_array()) {
                for p in arr {
                    if let Some(dt) = Self::extract_expires_at(p) {
                        return Some(dt);
                    }
                }
            }
        }
        if let Some(inner) = predicate.get("not") {
            return Self::extract_expires_at(inner);
        }
        None
    }

    /// Fetch claimable balances from Horizon and upsert into the database.
    /// Marks balances no longer in Horizon as claimed.
    pub async fn sync_balances(&self) -> Result<u64> {
        info!("Starting claimable balance sync from Horizon...");

        let mut all_ids = Vec::new();
        let mut cursor: Option<String> = None;
        let page_size = 200u32;

        loop {
            let c = cursor.as_deref();
            let records = self
                .rpc_client
                .fetch_claimable_balances(page_size, c)
                .await?;

            if records.is_empty() {
                break;
            }

            let mut tx = self.pool.begin().await?;

            for cb in &records {
                all_ids.push(cb.id.clone());
                let (asset_code, asset_issuer) = Self::parse_asset(&cb.asset);
                let sponsor = cb.sponsor.clone().unwrap_or_default();
                let last_modified_ledger = cb.last_modified_ledger;
                let paging_token = cb.paging_token.clone();

                let mut expires_at: Option<String> = None;
                for claimant in &cb.claimants {
                    if let Some(dt) = Self::extract_expires_at(&claimant.predicate) {
                        expires_at = Some(dt.to_rfc3339());
                        break;
                    }
                }

                let created_at = cb
                    .last_modified_time
                    .as_ref()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                sqlx::query(
                    r#"
                    INSERT INTO claimable_balances (
                        id, asset_code, asset_issuer, amount, sponsor,
                        created_at, expires_at, claimed, last_modified_ledger, paging_token
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9)
                    ON CONFLICT(id) DO UPDATE SET
                        amount = excluded.amount,
                        sponsor = excluded.sponsor,
                        expires_at = excluded.expires_at,
                        last_modified_ledger = excluded.last_modified_ledger,
                        paging_token = excluded.paging_token
                    "#,
                )
                .bind(&cb.id)
                .bind(&asset_code)
                .bind(&asset_issuer)
                .bind(&cb.amount)
                .bind(&sponsor)
                .bind(created_at)
                .bind(&expires_at)
                .bind(last_modified_ledger)
                .bind(&paging_token)
                .execute(&mut *tx)
                .await?;

                sqlx::query("DELETE FROM claimable_balance_claimants WHERE balance_id = ?1")
                    .bind(&cb.id)
                    .execute(&mut *tx)
                    .await?;

                for claimant in &cb.claimants {
                    let pred_str = serde_json::to_string(&claimant.predicate).unwrap_or_default();
                    sqlx::query(
                        r#"
                        INSERT INTO claimable_balance_claimants (balance_id, destination, predicate)
                        VALUES (?1, ?2, ?3)
                        "#,
                    )
                    .bind(&cb.id)
                    .bind(&claimant.destination)
                    .bind(&pred_str)
                    .execute(&mut *tx)
                    .await?;
                }
            }

            cursor = records.last().and_then(|r| r.paging_token.clone());
            tx.commit().await?;

            if records.len() < page_size as usize {
                break;
            }
        }

        info!("Synced {} claimable balances", all_ids.len());
        Ok(all_ids.len() as u64)
    }

    /// List all claimable balances (optional filters)
    pub async fn list_balances(
        &self,
        claimed_filter: Option<bool>,
        asset_code: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ClaimableBalance>> {
        use sqlx::Row;

        let limit = limit.clamp(1, 500);
        let offset = offset.max(0);

        let rows = if let Some(code) = asset_code {
            let claimed_cond = match claimed_filter {
                Some(true) => " AND cb.claimed = 1",
                Some(false) => " AND cb.claimed = 0",
                None => "",
            };
            let sql = format!(
                r#"
                SELECT cb.id, cb.asset_code, cb.asset_issuer, cb.amount, cb.sponsor,
                       cb.created_at, cb.expires_at, cb.claimed, cb.claimed_at, cb.claimed_by,
                       cb.last_modified_ledger,
                       (SELECT COUNT(*) FROM claimable_balance_claimants cbc WHERE cbc.balance_id = cb.id) as claimant_count
                FROM claimable_balances cb
                WHERE cb.asset_code = ? {}
                ORDER BY cb.created_at DESC LIMIT ? OFFSET ?
                "#,
                claimed_cond
            );
            sqlx::query(&sql)
                .bind(code)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            let claimed_cond = match claimed_filter {
                Some(true) => " WHERE cb.claimed = 1",
                Some(false) => " WHERE cb.claimed = 0",
                None => "",
            };
            let sql = format!(
                r#"
                SELECT cb.id, cb.asset_code, cb.asset_issuer, cb.amount, cb.sponsor,
                       cb.created_at, cb.expires_at, cb.claimed, cb.claimed_at, cb.claimed_by,
                       cb.last_modified_ledger,
                       (SELECT COUNT(*) FROM claimable_balance_claimants cbc WHERE cbc.balance_id = cb.id) as claimant_count
                FROM claimable_balances cb
                {}
                ORDER BY cb.created_at DESC LIMIT ? OFFSET ?
                "#,
                claimed_cond
            );
            sqlx::query(&sql)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        };

        let balances = rows
            .into_iter()
            .map(|row| ClaimableBalance {
                id: row.get("id"),
                asset_code: row.get("asset_code"),
                asset_issuer: row.get("asset_issuer"),
                amount: row.get("amount"),
                sponsor: row.get("sponsor"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                claimed: row.get::<i32, _>("claimed") != 0,
                claimed_at: row.get("claimed_at"),
                claimed_by: row.get("claimed_by"),
                last_modified_ledger: row.get("last_modified_ledger"),
                claimant_count: row.get("claimant_count"),
            })
            .collect();

        Ok(balances)
    }

    /// Get a single claimable balance by ID
    pub async fn get_balance(&self, id: &str) -> Result<Option<ClaimableBalance>> {
        use sqlx::Row;

        let row = sqlx::query(
            r#"
            SELECT cb.id, cb.asset_code, cb.asset_issuer, cb.amount, cb.sponsor,
                   cb.created_at, cb.expires_at, cb.claimed, cb.claimed_at, cb.claimed_by,
                   cb.last_modified_ledger,
                   (SELECT COUNT(*) FROM claimable_balance_claimants cbc WHERE cbc.balance_id = cb.id) as claimant_count
            FROM claimable_balances cb
            WHERE cb.id = ?1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| ClaimableBalance {
            id: r.get("id"),
            asset_code: r.get("asset_code"),
            asset_issuer: r.get("asset_issuer"),
            amount: r.get("amount"),
            sponsor: r.get("sponsor"),
            created_at: r.get("created_at"),
            expires_at: r.get("expires_at"),
            claimed: r.get::<i32, _>("claimed") != 0,
            claimed_at: r.get("claimed_at"),
            claimed_by: r.get("claimed_by"),
            last_modified_ledger: r.get("last_modified_ledger"),
            claimant_count: r.get("claimant_count"),
        }))
    }

    /// Get balances expiring within the given number of days (future dates only)
    pub async fn get_expiring_soon(&self, days: i64) -> Result<Vec<ClaimableBalance>> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let cutoff = now + chrono::Duration::days(days);
        let cutoff_str = cutoff.to_rfc3339();

        use sqlx::Row;

        let rows = sqlx::query(
            r#"
            SELECT cb.id, cb.asset_code, cb.asset_issuer, cb.amount, cb.sponsor,
                   cb.created_at, cb.expires_at, cb.claimed, cb.claimed_at, cb.claimed_by,
                   cb.last_modified_ledger,
                   (SELECT COUNT(*) FROM claimable_balance_claimants cbc WHERE cbc.balance_id = cb.id) as claimant_count
            FROM claimable_balances cb
            WHERE cb.claimed = 0 AND cb.expires_at IS NOT NULL AND cb.expires_at > ?1 AND cb.expires_at <= ?2
            ORDER BY cb.expires_at ASC
            LIMIT 100
            "#,
        )
        .bind(&now_str)
        .bind(&cutoff_str)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ClaimableBalance {
                id: r.get("id"),
                asset_code: r.get("asset_code"),
                asset_issuer: r.get("asset_issuer"),
                amount: r.get("amount"),
                sponsor: r.get("sponsor"),
                created_at: r.get("created_at"),
                expires_at: r.get("expires_at"),
                claimed: r.get::<i32, _>("claimed") != 0,
                claimed_at: r.get("claimed_at"),
                claimed_by: r.get("claimed_by"),
                last_modified_ledger: r.get("last_modified_ledger"),
                claimant_count: r.get("claimant_count"),
            })
            .collect())
    }

    /// Get analytics for claimable balances
    pub async fn get_analytics(&self) -> Result<ClaimableBalanceAnalytics> {
        use sqlx::Row;

        let total_row = sqlx::query(
            r#"
            SELECT COUNT(*) as cnt FROM claimable_balances
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        let total_locked_count: i64 = total_row.get("cnt");

        let pending_row = sqlx::query(
            r#"
            SELECT COUNT(*) as cnt FROM claimable_balances WHERE claimed = 0
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        let pending_claims_count: i64 = pending_row.get("cnt");

        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let in_30_days = (now + chrono::Duration::days(30)).to_rfc3339();
        let expiring_row = sqlx::query(
            r#"
            SELECT COUNT(*) as cnt FROM claimable_balances
            WHERE claimed = 0 AND expires_at IS NOT NULL AND expires_at > ?1 AND expires_at <= ?2
            "#,
        )
        .bind(&now_str)
        .bind(&in_30_days)
        .fetch_one(&self.pool)
        .await?;
        let expiring_soon_count: i64 = expiring_row.get("cnt");

        let claimed_row = sqlx::query(
            r#"
            SELECT COUNT(*) as cnt FROM claimable_balances WHERE claimed = 1
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        let claimed_count: i64 = claimed_row.get("cnt");
        let claim_success_rate = if total_locked_count + claimed_count > 0 {
            (claimed_count as f64) / (total_locked_count + claimed_count) as f64 * 100.0
        } else {
            0.0
        };

        let top_assets_rows = sqlx::query(
            r#"
            SELECT asset_code, asset_issuer,
                   SUM(CAST(amount AS REAL)) as total_amount,
                   COUNT(*) as cnt
            FROM claimable_balances
            WHERE claimed = 0
            GROUP BY asset_code, asset_issuer
            ORDER BY total_amount DESC
            LIMIT 10
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let top_assets: Vec<TopAssetClaimable> = top_assets_rows
            .into_iter()
            .map(|r| TopAssetClaimable {
                asset_code: r.get("asset_code"),
                asset_issuer: r.get("asset_issuer"),
                total_amount: r.get::<f64, _>("total_amount"),
                count: r.get("cnt"),
            })
            .collect();

        Ok(ClaimableBalanceAnalytics {
            total_locked_count,
            pending_claims_count,
            expiring_soon_count,
            total_locked_value_usd: 0.0, // Would need price feed
            claim_success_rate,
            top_assets,
        })
    }
}
