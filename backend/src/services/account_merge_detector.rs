use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tracing::{info, warn};

use crate::rpc::{HorizonOperation, StellarRpcClient};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AccountMergeEvent {
    pub operation_id: String,
    pub transaction_hash: String,
    pub ledger_sequence: i64,
    pub source_account: String,
    pub destination_account: String,
    pub merged_balance: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountMergeStats {
    pub total_merges: i64,
    pub total_merged_balance: f64,
    pub unique_sources: i64,
    pub unique_destinations: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DestinationAccountPattern {
    pub destination_account: String,
    pub merge_count: i64,
    pub total_merged_balance: f64,
}

pub struct AccountMergeDetector {
    pool: Pool<Sqlite>,
    rpc_client: Arc<StellarRpcClient>,
}

impl AccountMergeDetector {
    pub fn new(pool: Pool<Sqlite>, rpc_client: Arc<StellarRpcClient>) -> Self {
        Self { pool, rpc_client }
    }

    /// Fetches operations for a ledger, extracts account merges, and persists merge events.
    pub async fn process_ledger_operations(&self, ledger_sequence: u64) -> Result<u64> {
        let operations = self
            .rpc_client
            .fetch_operations_for_ledger(ledger_sequence)
            .await?;

        let mut inserted = 0_u64;

        for operation in operations
            .iter()
            .filter(|op| op.operation_type == "account_merge")
        {
            if self
                .persist_merge_from_operation(ledger_sequence, operation)
                .await?
            {
                inserted += 1;
            }
        }

        if inserted > 0 {
            info!(
                "Detected and stored {} account merge operations for ledger {}",
                inserted, ledger_sequence
            );
        }

        Ok(inserted)
    }

    async fn persist_merge_from_operation(
        &self,
        ledger_sequence: u64,
        operation: &HorizonOperation,
    ) -> Result<bool> {
        let destination_account = match operation.into.clone() {
            Some(account) => account,
            None => {
                warn!(
                    "Skipping account_merge operation {} without destination account",
                    operation.id
                );
                return Ok(false);
            }
        };

        let source_account = operation
            .account
            .clone()
            .unwrap_or_else(|| operation.source_account.clone());

        let merged_balance = self
            .resolve_merged_balance(&operation.id, &destination_account)
            .await;

        let created_at = DateTime::parse_from_rfc3339(&operation.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let event = AccountMergeEvent {
            operation_id: operation.id.clone(),
            transaction_hash: operation.transaction_hash.clone(),
            ledger_sequence: ledger_sequence as i64,
            source_account,
            destination_account,
            merged_balance,
            created_at,
        };

        self.persist_merge_event(&event).await
    }

    async fn resolve_merged_balance(&self, operation_id: &str, destination: &str) -> f64 {
        match self.rpc_client.fetch_operation_effects(operation_id).await {
            Ok(effects) => {
                let credited_amount: f64 = effects
                    .into_iter()
                    .filter(|effect| effect.effect_type == "account_credited")
                    .filter(|effect| effect.account.as_deref() == Some(destination))
                    .filter_map(|effect| effect.amount.and_then(|value| value.parse::<f64>().ok()))
                    .sum();

                if credited_amount > 0.0 {
                    return credited_amount;
                }
            }
            Err(error) => {
                warn!(
                    "Failed to fetch effects for operation {} while resolving merge amount: {}",
                    operation_id, error
                );
            }
        }

        0.0
    }

    async fn persist_merge_event(&self, event: &AccountMergeEvent) -> Result<bool> {
        let result = sqlx::query(
            r#"
            INSERT INTO account_merges (
                operation_id,
                transaction_hash,
                ledger_sequence,
                source_account,
                destination_account,
                merged_balance,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (operation_id) DO NOTHING
            "#,
        )
        .bind(&event.operation_id)
        .bind(&event.transaction_hash)
        .bind(event.ledger_sequence)
        .bind(&event.source_account)
        .bind(&event.destination_account)
        .bind(event.merged_balance)
        .bind(event.created_at)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_recent_merges(&self, limit: i64) -> Result<Vec<AccountMergeEvent>> {
        let rows = sqlx::query_as::<_, AccountMergeEvent>(
            r#"
            SELECT operation_id, transaction_hash, ledger_sequence, source_account, destination_account, merged_balance, created_at
            FROM account_merges
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn get_merge_stats(&self) -> Result<AccountMergeStats> {
        let row: (i64, f64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) AS total_merges,
                COALESCE(SUM(merged_balance), 0.0) AS total_merged_balance,
                COUNT(DISTINCT source_account) AS unique_sources,
                COUNT(DISTINCT destination_account) AS unique_destinations
            FROM account_merges
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(AccountMergeStats {
            total_merges: row.0,
            total_merged_balance: row.1,
            unique_sources: row.2,
            unique_destinations: row.3,
        })
    }

    pub async fn get_destination_patterns(
        &self,
        limit: i64,
    ) -> Result<Vec<DestinationAccountPattern>> {
        let rows = sqlx::query_as::<_, DestinationAccountPattern>(
            r#"
            SELECT
                destination_account,
                COUNT(*) AS merge_count,
                COALESCE(SUM(merged_balance), 0.0) AS total_merged_balance
            FROM account_merges
            GROUP BY destination_account
            ORDER BY merge_count DESC, total_merged_balance DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
