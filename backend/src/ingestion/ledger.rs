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

/// Result of comparing the persisted cursor against the network head on
/// startup. Used to detect gaps after downtime (#2602).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GapCheck {
    /// Last ledger we have persisted (the resumable cursor), if any.
    pub last_ledger: Option<u64>,
    /// Current network head ledger reported by RPC health.
    pub head_ledger: u64,
    /// Oldest ledger still retrievable from RPC/Horizon retention.
    pub oldest_ledger: u64,
    /// Number of ledgers missing between `last_ledger` and `head_ledger`.
    pub gap: u64,
    /// `true` when the gap can be backfilled within retention; `false` when
    /// the missing ledgers have aged out and cannot be recovered.
    pub backfillable: bool,
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

    /// Compare the persisted last ledger against the network head and report
    /// whether the gap (if any) can be backfilled within RPC/Horizon
    /// retention (#2602).
    ///
    /// This is a pure read: it does not advance the cursor. `run_ingestion`
    /// uses the same information to decide where to resume, and callers can
    /// invoke this on startup to log/alert on gaps after downtime.
    pub async fn check_gap(&self) -> Result<GapCheck> {
        let last_ledger = self.get_last_ledger().await?;

        let client = &self.rpc_client;
        let health = Retry::spawn(retry_strategy(), || async {
            client
                .check_health()
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        })
        .await
        .context("Failed to check health")?;

        let head_ledger = health.latest_ledger;
        let oldest_ledger = health.oldest_ledger;

        let gap = match last_ledger {
            Some(last) if head_ledger > last => head_ledger - last,
            _ => 0,
        };

        // A gap is backfillable when the first missing ledger (last + 1) is
        // still within retention, i.e. at or after the oldest retrievable
        // ledger. If the missing ledgers have aged out, we cannot recover
        // them and must alert instead.
        let backfillable = match last_ledger {
            Some(last) => last.saturating_add(1) >= oldest_ledger,
            None => true,
        };

        Ok(GapCheck {
            last_ledger,
            head_ledger,
            oldest_ledger,
            gap,
            backfillable,
        })
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

        // Gap detection after downtime (#2602): compare the persisted cursor
        // with the network head. If the gap is within retention we backfill
        // automatically (the fetch below resumes from `last + 1`); if it has
        // aged out we cannot recover it and emit an alert instead.
        if let Some(last) = self.get_last_ledger().await? {
            let client = &self.rpc_client;
            if let Ok(health) = Retry::spawn(retry_strategy(), || async {
                client
                    .check_health()
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))
            })
            .await
            {
                let head = health.latest_ledger;
                let oldest = health.oldest_ledger;
                if head > last {
                    let gap = head - last;
                    if last.saturating_add(1) >= oldest {
                        info!(
                            "Detected backfillable gap of {} ledger(s) after downtime \
                             (last={}, head={}, oldest={}); resuming from {}",
                            gap,
                            last,
                            head,
                            oldest,
                            last + 1
                        );
                    } else {
                        warn!(
                            "ALERT: ledger gap of {} ledger(s) exceeds RPC/Horizon retention \
                             (last={}, head={}, oldest={}); missing ledgers cannot be backfilled",
                            gap,
                            last,
                            head,
                            oldest
                        );
                    }
                }
            }
        }

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
            .awa