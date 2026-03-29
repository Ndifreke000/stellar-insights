use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{
    MetricRecord, MuxedAccountAnalytics, MuxedAccountUsage, SnapshotRecord,
};

pub struct MetricsDb {
    pool: SqlitePool,
}

impl MetricsDb {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn record_metric(
        &self,
        name: &str,
        value: f64,
        entity_id: Option<String>,
        entity_type: Option<String>,
    ) -> Result<MetricRecord> {
        sqlx::query_as::<_, MetricRecord>(
            "INSERT INTO metrics (id, name, value, entity_id, entity_type, timestamp)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(name)
        .bind(value)
        .bind(entity_id.clone())
        .bind(entity_type)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to record metric: {name}, entity={entity_id:?}"))
    }

    pub async fn create_snapshot(
        &self,
        entity_id: &str,
        entity_type: &str,
        data: serde_json::Value,
        hash: Option<String>,
        epoch: Option<i64>,
    ) -> Result<SnapshotRecord> {
        sqlx::query_as::<_, SnapshotRecord>(
            "INSERT INTO snapshots (id, entity_id, entity_type, data, hash, epoch, timestamp)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(entity_id)
        .bind(entity_type)
        .bind(data.to_string())
        .bind(hash)
        .bind(epoch)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to create snapshot: {entity_id}/{entity_type}"))
    }

    pub async fn get_snapshot_by_epoch(&self, epoch: i64) -> Result<Option<SnapshotRecord>> {
        sqlx::query_as::<_, SnapshotRecord>(
            "SELECT * FROM snapshots WHERE epoch = $1 LIMIT 1",
        )
        .bind(epoch)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch snapshot for epoch: {epoch}"))
    }

    pub async fn list_snapshots(&self, limit: i64, offset: i64) -> Result<Vec<SnapshotRecord>> {
        sqlx::query_as::<_, SnapshotRecord>(
            "SELECT * FROM snapshots WHERE epoch IS NOT NULL
             ORDER BY epoch DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to list snapshots (limit={limit}, offset={offset})"))
    }

    pub async fn get_ingestion_cursor(&self, task_name: &str) -> Result<Option<String>> {
        let state = sqlx::query_as::<_, crate::models::IngestionState>(
            "SELECT * FROM ingestion_state WHERE task_name = $1",
        )
        .bind(task_name)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to get ingestion cursor: {task_name}"))?;
        Ok(state.map(|s| s.last_cursor))
    }

    pub async fn update_ingestion_cursor(&self, task_name: &str, last_cursor: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO ingestion_state (task_name, last_cursor, updated_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (task_name) DO UPDATE
             SET last_cursor = EXCLUDED.last_cursor, updated_at = EXCLUDED.updated_at",
        )
        .bind(task_name)
        .bind(last_cursor)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to update ingestion cursor: {task_name}"))?;
        Ok(())
    }

    pub async fn save_payments(&self, payments: Vec<crate::models::PaymentRecord>) -> Result<()> {
        if payments.is_empty() {
            return Ok(());
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin transaction for save_payments")?;

        for payment in &payments {
            sqlx::query(
                "INSERT INTO payments (
                    id, transaction_hash, source_account, destination_account,
                    asset_type, asset_code, asset_issuer, amount, created_at
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (id) DO NOTHING",
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
            .await
            .with_context(|| format!("Failed to save payment: {}", payment.id))?;
        }

        tx.commit()
            .await
            .with_context(|| format!("Failed to commit save_payments ({} payments)", payments.len()))
    }

    pub async fn create_pending_transaction(
        &self,
        source_account: &str,
        xdr: &str,
        required_signatures: i32,
    ) -> Result<crate::models::PendingTransaction> {
        sqlx::query_as::<_, crate::models::PendingTransaction>(
            "INSERT INTO pending_transactions (id, source_account, xdr, required_signatures, status)
             VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(source_account)
        .bind(xdr)
        .bind(required_signatures)
        .bind("pending")
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to create pending transaction: {source_account}"))
    }

    pub async fn get_pending_transaction(
        &self,
        id: &str,
    ) -> Result<Option<crate::models::PendingTransactionWithSignatures>> {
        let tx = sqlx::query_as::<_, crate::models::PendingTransaction>(
            "SELECT * FROM pending_transactions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch pending transaction: {id}"))?;

        let Some(transaction) = tx else {
            return Ok(None);
        };

        let signatures = sqlx::query_as::<_, crate::models::Signature>(
            "SELECT * FROM transaction_signatures WHERE transaction_id = $1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch signatures for transaction: {id}"))?;

        Ok(Some(crate::models::PendingTransactionWithSignatures {
            transaction,
            collected_signatures: signatures,
        }))
    }

    pub async fn add_transaction_signature(
        &self,
        transaction_id: &str,
        signer: &str,
        signature: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO transaction_signatures (id, transaction_id, signer, signature)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(transaction_id)
        .bind(signer)
        .bind(signature)
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!("Failed to add signature: tx={transaction_id}, signer={signer}")
        })?;
        Ok(())
    }

    pub async fn update_transaction_status(&self, id: &str, status: &str) -> Result<()> {
        sqlx::query(
            "UPDATE pending_transactions SET status = $1, updated_at = CURRENT_TIMESTAMP
             WHERE id = $2",
        )
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to update transaction status '{status}': {id}"))?;
        Ok(())
    }

    pub async fn get_muxed_analytics(&self, top_limit: i64) -> Result<MuxedAccountAnalytics> {
        use crate::muxed;
        const MUXED_LEN: i64 = 69;

        let total_muxed_payments = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM payments
             WHERE (source_account LIKE 'M%' AND LENGTH(source_account) = $1)
                OR (destination_account LIKE 'M%' AND LENGTH(destination_account) = $1)",
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
            "SELECT source_account AS addr, COUNT(*) AS cnt FROM payments
             WHERE source_account LIKE 'M%' AND LENGTH(source_account) = $1
             GROUP BY source_account ORDER BY cnt DESC LIMIT $2",
        )
        .bind(MUXED_LEN)
        .bind(top_limit)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch top muxed source accounts (limit={top_limit})"))?;

        let dest_counts: Vec<AddrCount> = sqlx::query_as(
            "SELECT destination_account AS addr, COUNT(*) AS cnt FROM payments
             WHERE destination_account LIKE 'M%' AND LENGTH(destination_account) = $1
             GROUP BY destination_account ORDER BY cnt DESC LIMIT $2",
        )
        .bind(MUXED_LEN)
        .bind(top_limit)
        .fetch_all(&self.pool)
        .await
        .with_context(|| {
            format!("Failed to fetch top muxed destination accounts (limit={top_limit})")
        })?;

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
        top_muxed_by_activity.truncate(std::cmp::max(0, top_limit) as usize);

        let unique_muxed_addresses = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(DISTINCT addr) FROM (
                SELECT source_account AS addr FROM payments
                WHERE source_account LIKE 'M%' AND LENGTH(source_account) = $1
                UNION
                SELECT destination_account AS addr FROM payments
                WHERE destination_account LIKE 'M%' AND LENGTH(destination_account) = $1
             )",
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
    }
}
