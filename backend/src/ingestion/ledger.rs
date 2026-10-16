use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
use tracing::{info, warn};

fn retry_strategy() -> impl Iterator<Item = Duration> {
    ExponentialBackoff::from_millis(200)
        .max_delay(Duration::from_secs(30))
        .map(jitter)
        .take(10)
}

use crate::network::StellarNetwork;
use crate::rpc::{GetLedgersResult, RpcLedger, StellarRpcClient};
use crate::services::account_merge_detector::AccountMergeDetector;
use crate::services::fee_bump_tracker::FeeBumpTrackerService;
use crate::shutdown::ShutdownFlag;

/// Ledger ingestion service that fetches and persists ledgers sequentially.
///
/// Every row this service writes is tagged with `network` so that mainnet
/// and testnet ingestion can run against the same database without their
/// data mixing (see migration 048_add_network_to_ledger_ingestion.sql).
pub struct LedgerIngestionService {
    rpc_client: Arc<StellarRpcClient>,
    fee_bump_tracker: Arc<FeeBumpTrackerService>,
    account_merge_detector: Arc<AccountMergeDetector>,
    pool: SqlitePool,
    network: StellarNetwork,
    webhook_event_service: Option<Arc<crate::services::webhook_event_service::WebhookEventService>>,
    /// Checked between ledgers (never mid-ledger) so a SIGTERM stops
    /// ingestion at a ledger boundary instead of leaving a ledger's payments
    /// half-persisted (#2655). `None` when the caller hasn't wired one up —
    /// ingestion then always runs to the end of the fetched batch.
    shutdown: Option<ShutdownFlag>,
}

/// Outcome of processing one fetched batch of ledgers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProcessLedgersOutcome {
    /// How many ledgers in the batch were fully persisted.
    processed: u64,
    /// `true` if ingestion stopped before the end of the batch because
    /// shutdown was requested (see `ShutdownFlag`), rather than finishing
    /// every fetched ledger.
    stopped_early: bool,
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
    /// Create a new ingestion service for the given network. Callers must
    /// pass the network explicitly rather than relying on a default, since
    /// defaulting silently is exactly how mainnet/testnet rows used to end
    /// up tagged the same way.
    #[must_use]
    pub const fn new(
        rpc_client: Arc<StellarRpcClient>,
        fee_bump_tracker: Arc<FeeBumpTrackerService>,
        account_merge_detector: Arc<AccountMergeDetector>,
        pool: SqlitePool,
        network: StellarNetwork,
    ) -> Self {
        Self {
            rpc_client,
            fee_bump_tracker,
            account_merge_detector,
            pool,
            network,
            webhook_event_service: None,
            shutdown: None,
        }
    }

    #[must_use]
    pub const fn new_with_webhooks(
        rpc_client: Arc<StellarRpcClient>,
        fee_bump_tracker: Arc<FeeBumpTrackerService>,
        account_merge_detector: Arc<AccountMergeDetector>,
        pool: SqlitePool,
        network: StellarNetwork,
        webhook_event_service: Arc<crate::services::webhook_event_service::WebhookEventService>,
    ) -> Self {
        Self {
            rpc_client,
            fee_bump_tracker,
            account_merge_detector,
            pool,
            network,
            webhook_event_service: Some(webhook_event_service),
            shutdown: None,
        }
    }

    /// Attaches a shutdown flag so `run_ingestion` stops at a ledger
    /// boundary once shutdown has been triggered, instead of running to the
    /// end of the fetched batch regardless. See `ShutdownFlag` (#2655).
    #[must_use]
    pub fn with_shutdown_flag(mut self, flag: ShutdownFlag) -> Self {
        self.shutdown = Some(flag);
        self
    }

    /// I'm running the main ingestion loop - fetches ledgers and persists them
    pub async fn run_ingestion(&self, batch_size: u32) -> Result<u64> {
        let cursor = self.get_cursor().await?;
        let start_ledger = if let Some(l) = self.get_last_ledger().await? {
            Some(l + 1)
        } else {
            let client = &self.rpc_client;
            let health = Retry::spawn(retry_strategy(), || async {
                client
                    .check_health()
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))
            })
            .await
            .context("Failed to check health")?;
            Some(health.oldest_ledger)
        };

        info!(
            "Starting ingestion from ledger {:?}, cursor: {:?}",
            start_ledger, cursor
        );

        let client = &self.rpc_client;
        let cursor_ref = cursor.as_deref();
        let result = Retry::spawn(retry_strategy(), || async {
            client
                .fetch_ledgers(start_ledger, batch_size, cursor_ref)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        })
        .await
        .context("Failed to fetch ledgers")?;

        let outcome = self.process_ledgers(&result).await?;

        // I'm saving cursor for restart safety — but only when the whole
        // fetched batch was processed. If shutdown cut ingestion short
        // (`stopped_early`), advancing the cursor/last-ledger checkpoint to
        // `result`'s end would make the next run start *after* ledgers we
        // never actually touched, silently dropping them. Leaving the
        // checkpoint where it was means the next run just re-fetches this
        // same batch from the start — safe, since `persist_ledger` upserts
        // ledgers/transactions with `ON CONFLICT (..) DO NOTHING` (#2655).
        if outcome.stopped_early {
            info!(
                "Ingestion stopped at a ledger boundary after {} ledger(s) due to shutdown; \
                 checkpoint left unchanged so the next run resumes this batch",
                outcome.processed
            );
        } else if let Some(new_cursor) = &result.cursor {
            self.save_cursor(new_cursor, result.ledgers.last().map(|l| l.sequence))
                .await?;
        }

        Ok(outcome.processed)
    }

    /// I'm processing and persisting fetched ledgers
    async fn process_ledgers(&self, result: &GetLedgersResult) -> Result<ProcessLedgersOutcome> {
        let mut count = 0u64;

        for ledger in &result.ledgers {
            // Checked *between* ledgers only — never mid-ledger — so a
            // shutdown always lands on a clean ledger boundary (#2655).
            if self.shutdown.as_ref().is_some_and(ShutdownFlag::is_set) {
                info!(
                    "Shutdown requested; stopping ledger ingestion before ledger {} \
                     ({} ledger(s) already persisted this batch)",
                    ledger.sequence, count
                );
                return Ok(ProcessLedgersOutcome {
                    processed: count,
                    stopped_early: true,
                });
            }

            if let Err(e) = self.persist_ledger(ledger).await {
                warn!("Failed to persist ledger {}: {}", ledger.sequence, e);
                continue;
            }

            // Fetch real payments from Horizon
            let seq = ledger.sequence;
            let client = &self.rpc_client;
            match Retry::spawn(retry_strategy(), || async {
                client
                    .fetch_payments_for_ledger(seq)
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))
            })
            .await
            {
                Ok(payments) => {
                    for payment in payments {
                        // Convert RPC Payment to ExtractedPayment
                        // Uses helper methods to support both old and new Horizon formats
                        let extracted = ExtractedPayment {
                            ledger_sequence: ledger.sequence,
                            transaction_hash: payment.transaction_hash.clone(),
                            operation_type: "payment".to_string(), // Horizon 'payments' endpoint returns payments
                            source_account: payment.source_account.clone(),
                            destination: payment.get_destination().unwrap_or_default(),
                            asset_code: payment.get_asset_code(),
                            asset_issuer: payment.get_asset_issuer(),
                            amount: payment.get_amount(),
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
            match Retry::spawn(retry_strategy(), || async {
                client
                    .fetch_transactions_for_ledger(seq)
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))
            })
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
        Ok(ProcessLedgersOutcome {
            processed: count,
            stopped_early: false,
        })
    }

    /// I'm persisting a single ledger to the database
    async fn persist_ledger(&self, ledger: &RpcLedger) -> Result<()> {
        let close_time = self.parse_ledger_time(&ledger.ledger_close_time)?;

        let network = self.network.to_string();

        sqlx::query(
            r"
            INSERT INTO ledgers (sequence, hash, close_time, transaction_count, operation_count, network)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (sequence) DO NOTHING
            ",
        )
        .bind(ledger.sequence as i64)
        .bind(&ledger.hash)
        .bind(close_time)
        .bind(0i32) // I'd get real counts from XDR parsing
        .bind(0i32)
        .bind(&network)
        .execute(&self.pool)
        .await?;

        // I'm also storing a placeholder transaction for the ledger.
        // Note: `tx_hash` is derived only from the ledger sequence, so this
        // placeholder collides across networks at the same sequence number
        // just like `ledgers.sequence` does (see migration 048's note on
        // natural-key primary keys) — tagging it with `network` at least
        // lets reads filter correctly once ingestion no longer no-ops here.
        let tx_hash = format!("tx_{}", ledger.sequence);
        sqlx::query(
            r"
            INSERT INTO transactions (hash, ledger_sequence, source_account, fee, operation_count, successful, network)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (hash) DO NOTHING
            ",
        )
        .bind(&tx_hash)
        .bind(ledger.sequence as i64)
        .bind("GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")
        .bind(100i64)
        .bind(1i32)
        .bind(true)
        .bind(&network)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// I'm persisting an extracted payment to the database.
    ///
    /// Payments to/from a muxed (M-...) address are resolved to their base
    /// G-address here via `crate::muxed`, so corridor/anchor attribution can
    /// key off the underlying account rather than the muxed sub-account
    /// (which would never match an anchor's known G-address).
    async fn persist_payment(&self, payment: &ExtractedPayment) -> Result<()> {
        let source_base_account = crate::muxed::parse_muxed_address(&payment.source_account)
            .and_then(|info| info.base_account);
        let destination_base_account = crate::muxed::parse_muxed_address(&payment.destination)
            .and_then(|info| info.base_account);

        sqlx::query(
            r"
            INSERT INTO ledger_payments (ledger_sequence, transaction_hash, operation_type, source_account, destination, asset_code, asset_issuer, amount, network, source_base_account, destination_base_account)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ",
        )
        .bind(payment.ledger_sequence as i64)
        .bind(&payment.transaction_hash)
        .bind(&payment.operation_type)
        .bind(&payment.source_account)
        .bind(&payment.destination)
        .bind(&payment.asset_code)
        .bind(&payment.asset_issuer)
        .bind(&payment.amount)
        .bind(self.network.to_string())
        .bind(&source_base_account)
        .bind(&destination_base_account)
        .execute(&self.pool)
        .await?;

        // Trigger webhook event for payment creation
        if let Some(webhook_service) = &self.webhook_event_service {
            if payment.operation_type == "payment" {
                let asset_code = payment.asset_code.as_deref().unwrap_or("XLM");
                let asset_issuer = payment.asset_issuer.as_deref().unwrap_or("native");
                let amount = payment.amount.parse::<f64>().unwrap_or(0.0);

                let webhook_service = webhook_service.clone();
                let payment_id =
                    format!("{}-{}", payment.transaction_hash, payment.ledger_sequence);
                let source = payment.source_account.clone();
                let destination = payment.destination.clone();
                let asset_code = asset_code.to_string();
                let asset_issuer = asset_issuer.to_string();
                let timestamp = Utc::now().to_rfc3339();

                tokio::spawn(async move {
                    if let Err(e) = webhook_service
                        .trigger_payment_created(
                            &payment_id,
                            &source,
                            &destination,
                            &asset_code,
                            &asset_issuer,
                            amount,
                            &timestamp,
                        )
                        .await
                    {
                        tracing::error!("Failed to trigger payment created webhook: {}", e);
                    }
                });
            }
        }

        Ok(())
    }

    /// I'm getting the last ingested ledger sequence for resume, scoped to
    /// this service's network so mainnet and testnet ingestion each resume
    /// from their own position instead of sharing (and clobbering) one
    /// cursor row.
    async fn get_last_ledger(&self) -> Result<Option<u64>> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT last_ledger_sequence FROM ingestion_cursor WHERE network = $1",
        )
        .bind(self.network.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0 as u64))
    }

    /// I'm getting the saved cursor for pagination, scoped to this network.
    async fn get_cursor(&self) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT cursor FROM ingestion_cursor WHERE network = $1")
                .bind(self.network.to_string())
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.and_then(|r| r.0))
    }

    /// I'm saving cursor and last ledger for restart safety, upserting the
    /// row for this service's network (unique index on `network` from
    /// migration 048) rather than the single hardcoded `id = 1` row.
    async fn save_cursor(&self, cursor: &str, last_ledger: Option<u64>) -> Result<()> {
        let seq = last_ledger.unwrap_or(0) as i64;
        sqlx::query(
            r"
            INSERT INTO ingestion_cursor (last_ledger_sequence, cursor, network, updated_at)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
            ON CONFLICT (network) DO UPDATE SET
                last_ledger_sequence = EXCLUDED.last_ledger_sequence,
                cursor = EXCLUDED.cursor,
                updated_at = CURRENT_TIMESTAMP
            ",
        )
        .bind(seq)
        .bind(cursor)
        .bind(self.network.to_string())
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
