use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite};
use tracing::{info, warn};

use crate::models::{FeeBumpStats, FeeBumpTransaction};
use crate::rpc::HorizonTransaction; // Changed from StellarRpcClient as we process data structs

pub struct FeeBumpTrackerService {
    pool: Pool<Sqlite>,
}

impl FeeBumpTrackerService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Process a batch of transactions and persist fee bump transactions
    pub async fn process_transactions(&self, transactions: &[HorizonTransaction]) -> Result<u64> {
        let mut count = 0;

        for tx in transactions {
            if let Some(fee_bump) = &tx.fee_bump_transaction {
                if let Some(inner) = &tx.inner_transaction {
                    // Extract data safely
                    let fee_charged = tx
                        .fee_charged
                        .as_ref()
                        .and_then(|f| f.parse::<i64>().ok())
                        .unwrap_or(0);

                    let max_fee = tx
                        .max_fee
                        .as_ref()
                        .and_then(|f| f.parse::<i64>().ok())
                        .unwrap_or(0);

                    let inner_max_fee = inner
                        .max_fee
                        .as_ref()
                        .and_then(|f| f.parse::<i64>().ok())
                        .unwrap_or(0);

                    // Parse created_at
                    let created_at = DateTime::parse_from_rfc3339(&tx.created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    let fee_bump_tx = FeeBumpTransaction {
                        transaction_hash: tx.hash.clone(),
                        ledger_sequence: tx.ledger as i64,
                        fee_source: tx
                            .fee_account
                            .clone()
                            .unwrap_or_else(|| tx.source_account.clone()),
                        fee_charged,
                        max_fee,
                        inner_transaction_hash: inner.hash.clone(),
                        inner_max_fee,
                        signatures_count: fee_bump.signatures.len() as i32,
                        created_at,
                    };

                    if let Err(e) = self.persist_fee_bump(&fee_bump_tx).await {
                        warn!("Failed to persist fee bump transaction {}: {}", tx.hash, e);
                    } else {
                        count += 1;
                    }
                }
            }
        }

        if count > 0 {
            info!("Processed {} fee bump transactions", count);
        }

        Ok(count)
    }

    /// Persist a single fee bump transaction
    async fn persist_fee_bump(&self, tx: &FeeBumpTransaction) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO fee_bump_transactions (
                transaction_hash, ledger_sequence, fee_source, fee_charged, max_fee,
                inner_transaction_hash, inner_max_fee, signatures_count, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (transaction_hash) DO NOTHING
            "#,
        )
        .bind(&tx.transaction_hash)
        .bind(tx.ledger_sequence)
        .bind(&tx.fee_source)
        .bind(tx.fee_charged)
        .bind(tx.max_fee)
        .bind(&tx.inner_transaction_hash)
        .bind(tx.inner_max_fee)
        .bind(tx.signatures_count)
        .bind(tx.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get recent fee bump transactions
    pub async fn get_recent_fee_bumps(&self, limit: i64) -> Result<Vec<FeeBumpTransaction>> {
        let transactions = sqlx::query_as::<_, FeeBumpTransaction>(
            r#"
            SELECT * FROM fee_bump_transactions
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(transactions)
    }

    /// Get fee bump statistics
    pub async fn get_fee_bump_stats(&self) -> Result<FeeBumpStats> {
        let row: (i64, f64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT 
                COUNT(*) as total_count,
                COALESCE(AVG(fee_charged), 0.0) as avg_fee,
                COALESCE(MAX(fee_charged), 0) as max_fee,
                COALESCE(MIN(fee_charged), 0) as min_fee,
                COUNT(DISTINCT fee_source) as unique_sources
            FROM fee_bump_transactions
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(FeeBumpStats {
            total_fee_bumps: row.0,
            avg_fee_charged: row.1,
            max_fee_charged: row.2,
            min_fee_charged: row.3,
            unique_fee_sources: row.4,
        })
    }
}
