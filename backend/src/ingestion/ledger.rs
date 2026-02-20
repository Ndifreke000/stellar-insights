use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{info, warn};

use crate::rpc::{GetLedgersResult, RpcLedger, StellarRpcClient};
use crate::services::account_merge_detector::AccountMergeDetector;
use crate::services::fee_bump_tracker::FeeBumpTrackerService;

/// Ledger ingestion service that fetches and persists ledgers sequentially
pub struct LedgerIngestionService {
    rpc_client: Arc<StellarRpcClient>,
    fee_bump_tracker: Arc<FeeBumpTrackerService>,
    account_merge_detector: Arc<AccountMergeDetector>,
    pool: SqlitePool,
}

/// Represents a payment operation extracted from a ledger
#[derive(Debug, Clone)]
pub struct ExtractedPayment {
    pub ledger_sequence: u64,
    pub transaction_hash: String,
    pub operation_type: String,
    pub source_account: String,
    pub destination: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    pub amount: String,
}

impl LedgerIngestionService {
    pub fn new(
        rpc_client: Arc<StellarRpcClient>,
        fee_bump_tracker: Arc<FeeBumpTrackerService>,
        account_merge_detector: Arc<AccountMergeDetector>,
        pool: SqlitePool,
    ) -> Self {
        Self {
            rpc_client,
            fee_bump_tracker,
            account_merge_detector,
            pool,
        }
    }

    /// I'm running the main ingestion loop - fetches ledgers and persists them
    pub async fn run_ingestion(&self, batch_size: u32) -> Result<u64> {
        let cursor = self.get_cursor().await?;
        let start_ledger = match self.get_last_ledger().await? {
            Some(l) => Some(l + 1),
            None => {
                let health = self
                    .rpc_client
                    .check_health()
                    .await
                    .context("Failed to check health")?;
                Some(health.oldest_ledger)
            }
        };

        info!(
            "Starting ingestion from ledger {:?}, cursor: {:?}",
            start_ledger, cursor
        );

        let result = self
            .rpc_client
            .fetch_ledgers(start_ledger, batch_size, cursor.as_deref())
            .await
            .context("Failed to fetch ledgers")?;

        let count = self.process_ledgers(&result).await?;

        // I'm saving cursor for restart safety
        if let Some(new_cursor) = &result.cursor {
            self.save_cursor(new_cursor, result.ledgers.last().map(|l| l.sequence))
                .await?;
        }

        Ok(count)
    }

    /// I'm processing and persisting fetched ledgers
    async fn process_ledgers(&self, result: &GetLedgersResult) -> Result<u64> {
        let mut count = 0u64;

        for ledger in &result.ledgers {
            if let Err(e) = self.persist_ledger(ledger).await {
                warn!("Failed to persist ledger {}: {}", ledger.sequence, e);
                continue;
            }

            // Fetch real payments from Horizon
            match self
                .rpc_client
                .fetch_payments_for_ledger(ledger.sequence)
                .await
            {
                Ok(payments) => {
                    for payment in payments {
                        // Convert RPC Payment to ExtractedPayment
                        let extracted = ExtractedPayment {
                            ledger_sequence: ledger.sequence,
                            transaction_hash: payment.transaction_hash,
                            operation_type: "payment".to_string(), // Horizon 'payments' endpoint returns payments
                            source_account: payment.source_account,
                            destination: payment.destination,
                            asset_code: payment.asset_code,
                            asset_issuer: payment.asset_issuer,
                            amount: payment.amount,
                        };

                        if let Err(e) = self.persist_payment(&extracted).await {
                            warn!("Failed to persist payment: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch payments for ledger {}: {}",
                        ledger.sequence, e
                    );
                    // Non-fatal, continue ingesting ledgers
                }
            }

            // Fetch and process transactions for fee bumps
            match self
                .rpc_client
                .fetch_transactions_for_ledger(ledger.sequence)
                .await
            {
                Ok(transactions) => {
                    if let Err(e) = self
                        .fee_bump_tracker
                        .process_transactions(&transactions)
                        .await
                    {
                        warn!("Failed to process transactions for fee bumps: {}", e);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch transactions for ledger {}: {}",
                        ledger.sequence, e
                    );
                }
            }

            if let Err(e) = self
                .account_merge_detector
                .process_ledger_operations(ledger.sequence)
                .await
            {
                warn!(
                    "Failed to process account merge operations for ledger {}: {}",
                    ledger.sequence, e
                );
            }

            count += 1;
        }

        info!("Processed {} ledgers", count);
        Ok(count)
    }

    /// I'm persisting a single ledger to the database
    async fn persist_ledger(&self, ledger: &RpcLedger) -> Result<()> {
        let close_time = self.parse_ledger_time(&ledger.ledger_close_time)?;

        sqlx::query(
            r#"
            INSERT INTO ledgers (sequence, hash, close_time, transaction_count, operation_count)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (sequence) DO NOTHING
            "#,
        )
        .bind(ledger.sequence as i64)
        .bind(&ledger.hash)
        .bind(close_time)
        .bind(0i32) // I'd get real counts from XDR parsing
        .bind(0i32)
        .execute(&self.pool)
        .await?;

        // I'm also storing a placeholder transaction for the ledger
        let tx_hash = format!("tx_{}", ledger.sequence);
        sqlx::query(
            r#"
            INSERT INTO transactions (hash, ledger_sequence, source_account, fee, operation_count, successful)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (hash) DO NOTHING
            "#,
        )
        .bind(&tx_hash)
        .bind(ledger.sequence as i64)
        .bind("GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")
        .bind(100i64)
        .bind(1i32)
        .bind(true)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// I'm persisting an extracted payment to the database
    async fn persist_payment(&self, payment: &ExtractedPayment) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO ledger_payments (ledger_sequence, transaction_hash, operation_type, source_account, destination, asset_code, asset_issuer, amount)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(payment.ledger_sequence as i64)
        .bind(&payment.transaction_hash)
        .bind(&payment.operation_type)
        .bind(&payment.source_account)
        .bind(&payment.destination)
        .bind(&payment.asset_code)
        .bind(&payment.asset_issuer)
        .bind(&payment.amount)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// I'm getting the last ingested ledger sequence for resume
    async fn get_last_ledger(&self) -> Result<Option<u64>> {
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT last_ledger_sequence FROM ingestion_cursor WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|r| r.0 as u64))
    }

    /// I'm getting the saved cursor for pagination
    async fn get_cursor(&self) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT cursor FROM ingestion_cursor WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.and_then(|r| r.0))
    }

    /// I'm saving cursor and last ledger for restart safety
    async fn save_cursor(&self, cursor: &str, last_ledger: Option<u64>) -> Result<()> {
        let seq = last_ledger.unwrap_or(0) as i64;
        sqlx::query(
            r#"
            INSERT INTO ingestion_cursor (id, last_ledger_sequence, cursor, updated_at)
            VALUES (1, $1, $2, CURRENT_TIMESTAMP)
            ON CONFLICT (id) DO UPDATE SET
                last_ledger_sequence = EXCLUDED.last_ledger_sequence,
                cursor = EXCLUDED.cursor,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(seq)
        .bind(cursor)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    fn parse_ledger_time(&self, timestamp_str: &str) -> Result<DateTime<Utc>> {
        // I'm parsing unix timestamp string to DateTime
        let ts: i64 = timestamp_str.parse().unwrap_or(0);
        Ok(Utc.timestamp_opt(ts, 0).single().unwrap_or_else(Utc::now))
    }
}
