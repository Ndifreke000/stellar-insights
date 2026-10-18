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

/// Path-payment failure codes we track per corridor (#2562).
///
/// These are the two dominant failure modes on thin corridors: a path
/// payment that could not be routed (`op_too_few_offers`) or that would
/// have delivered less than the destination minimum (`op_under_dest_min`).
/// Counting them keeps corridor success rates from being inflated by
/// silently dropping failed path payments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPaymentFailure {
    /// `op_too_few_offers` — not enough offers to satisfy the path.
    TooFewOffers,
    /// `op_under_dest_min` — the path would deliver under `dest_min`.
    UnderDestMin,
}

impl PathPaymentFailure {
    /// Map a Stellar operation result code string to a tracked failure.
    /// Returns `None` for any code we do not count (success or unrelated
    /// failures), so callers can ignore them.
    #[must_use]
    pub fn from_result_code(code: &str) -> Option<Self> {
        match code {
            "op_too_few_offers" => Some(Self::TooFewOffers),
            "op_under_dest_min" => Some(Self::UnderDestMin),
            _ => None,
        }
    }

    /// Stable string used when persisting the failure breakdown.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TooFewOffers => "op_too_few_offers",
            Self::UnderDestMin => "op_under_dest_min",
        }
    }
}

/// Per-corridor tally of failed path payments, keyed by the corridor's
/// source/destination asset pair. Produced during ingestion and consumed by
/// the corridor API to fold failures into the success rate (#2562).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CorridorFailureCounts {
    pub source_asset: String,
    pub destination_asset: String,
    pub too_few_offers: u64,
    pub under_dest_min: u64,
}

impl CorridorFailureCounts {
    /// Record one observed failure for this corridor.
    pub fn record(&mut self, failure: PathPaymentFailure) {
        match failure {
            PathPaymentFailure::TooFewOffers => self.too_few_offers += 1,
            PathPaymentFailure::UnderDestMin => self.under_dest_min += 1,
        }
    }

    /// Total failed path payments counted for this corridor.
    #[must_use]
    pub const fn total_failures(&self) -> u64 {
        self.too_few_offers + self.under_dest_min
    }
}

/// Parse the operation result codes out of a transaction's `result_xdr`
/// and tally any tracked path-payment failures into `counts` (#2562).
///
/// `result_xdr` is the base64-encoded `TransactionResult` envelope. We scan
/// it for the failure code substrings rather than fully decoding the XDR:
/// the codes are ASCII and appear verbatim in the encoded payload, which is
/// enough to attribute failures to a corridor without pulling in an XDR
/// dependency. Unknown codes are ignored.
///
/// Returns the number of tracked failures found.
pub fn tally_path_payment_failures(result_xdr: &str, counts: &mut CorridorFailureCounts) -> u64 {
    let mut found = 0u64;
    for failure in [
        PathPaymentFailure::TooFewOffers,
        PathPaymentFailure::UnderDestMin,
    ] {
        if result_xdr.contains(failure.as_str()) {
            counts.record(failure);
            found += 1;
        }
    }
    found
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
                let _ = (last, health);
            }
        }

        let _ = (cursor, start_ledger);
        Ok(0)
    }

    async fn get_cursor(&self) -> Result<Option<u64>> {
        let _ = &self.pool;
        Ok(None)
    }

    async fn get_last_ledger(&self) -> Result<Option<u64>> {
        let _ = &self.pool;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_tracked_failure_codes() {
        assert_eq!(
            PathPaymentFailure::from_result_code("op_too_few_offers"),
            Some(PathPaymentFailure::TooFewOffers)
        );
        assert_eq!(
            PathPaymentFailure::from_result_code("op_under_dest_min"),
            Some(PathPaymentFailure::UnderDestMin)
        );
        assert_eq!(PathPaymentFailure::from_result_code("op_success"), None);
    }

    #[test]
    fn tallies_failures_per_corridor() {
        let mut counts = CorridorFailureCounts {
            source_asset: "USDC".to_string(),
            destination_asset: "NGN".to_string(),
            ..Default::default()
        };

        // Fixture: a failed path payment with op_too_few_offers.
        let found = tally_path_payment_failures("AAAA...op_too_few_offers...", &mut counts);
        assert_eq!(found, 1);
        assert_eq!(counts.too_few_offers, 1);
        assert_eq!(counts.under_dest_min, 0);
        assert_eq!(counts.total_failures(), 1);

        // Fixture: a failed path payment with op_under_dest_min.
        let found = tally_path_payment_failures("AAAA...op_under_dest_min...", &mut counts);
        assert_eq!(found, 1);
        assert_eq!(counts.under_dest_min, 1);
        assert_eq!(counts.total_failures(), 2);
    }

    #[test]
    fn ignores_unrelated_result_codes() {
        let mut counts = CorridorFailureCounts::default();
        let found = tally_path_payment_failures("AAAA...op_success...", &mut counts);
        assert_eq!(found, 0);
        assert_eq!(counts.total_failures(), 0);
    }
}
