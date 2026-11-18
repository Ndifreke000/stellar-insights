use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::warn;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;

fn retry_strategy() -> impl Iterator<Item = Duration> {
    ExponentialBackoff::from_millis(200)
        .max_delay(Duration::from_secs(30))
        .map(jitter)
        .take(10)
}

use crate::ingestion::adaptive_batch::{AimdConfig, AimdController, FetchOutcome};
use crate::network::StellarNetwork;
use crate::observability::metrics as obs_metrics;
use crate::rpc::{RpcLedger, StellarRpcClient};

/// Initial batch size before the scheduler's requested size is applied.
const DEFAULT_BATCH: u32 = 50;
use crate::services::account_merge_detector::AccountMergeDetector;
use crate::services::fee_bump_tracker::FeeBumpTrackerService;
use crate::shutdown::ShutdownFlag;

/// Ledger ingestion service that fetches and persists ledgers sequentially.
///
/// Every row this service writes is tagged with `network` so that mainnet
/// and testnet ingestion can run against the same database without their
/// data mixing (see migration 052_add_network_to_ledger_ingestion.sql).
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
    /// AIMD batch sizing state, carried across `run_ingestion` calls (#2767).
    batch_controller: Mutex<AimdController>,
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
    pub fn new(
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
            batch_controller: Mutex::new(AimdController::new(AimdConfig::default(), DEFAULT_BATCH)),
        }
    }

    #[must_use]
    pub fn new_with_webhooks(
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
            batch_controller: Mutex::new(AimdController::new(AimdConfig::default(), DEFAULT_BATCH)),
        }
    }

    /// Override the AIMD batch-sizing tunables (ceiling, step, lag threshold,
    /// latency budget). See [`AimdConfig`] (#2767).
    #[must_use]
    pub fn with_batch_config(self, config: AimdConfig) -> Self {
        let requested = self
            .batch_controller
            .lock()
            .expect("batch controller poisoned")
            .batch_size();
        *self.batch_controller.lock().expect("batch controller poisoned") =
            AimdController::new(config, requested);
        self
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
        let health = Retry::start(retry_strategy(), || async {
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

    /// Run the ingestion loop with adaptive batch sizing (#2767).
    ///
    /// `batch_size` is the scheduler's requested base batch size. It seeds an
    /// [`AimdController`] which grows the `getLedgers` page size while we are
    /// behind the network head (and RPC latency is within budget) and halves
    /// it on 429/5xx/timeouts. The configured ceiling
    /// ([`AimdConfig::max_batch`]) is never exceeded.
    ///
    /// Each fetched batch is persisted — ledgers *and* the cursor — in a
    /// single DB transaction, so a crash mid-batch leaves the cursor at the
    /// last fully committed batch and a restart resumes from `cursor + 1`
    /// with no duplicates or skipped ledgers. Only the contiguous prefix of a
    /// batch starting at the expected sequence is committed.
    ///
    /// Returns the number of ledgers committed by this call.
    pub async fn run_ingestion(&self, batch_size: u32) -> Result<u64> {
        let mut next = match self.get_last_ledger().await? {
            Some(l) => l.saturating_add(1),
            None => {
                let client = &self.rpc_client;
                let health = Retry::start(retry_strategy(), || async {
                    client
                        .check_health()
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"))
                })
                .await
                .context("Failed to check health")?;
                health.oldest_ledger
            }
        };

        let mut total = 0u64;
        for _ in 0..MAX_ROUNDS_PER_RUN {
            if self.shutdown.as_ref().is_some_and(ShutdownFlag::is_set) {
                break;
            }

            let limit = {
                let mut ctrl = self.batch_controller.lock().expect("batch controller poisoned");
                ctrl.set_requested(batch_size);
                ctrl.batch_size()
            };
            obs_metrics::set_ingestion_batch_size(limit);

            let started = Instant::now();
            let result = match self.rpc_client.fetch_ledgers(Some(next), limit, None).await {
                Ok(r) => r,
                Err(e) => {
                    let outcome = FetchOutcome::from_rpc_error(&e);
                    let new_size = self
                        .batch_controller
                        .lock()
                        .expect("batch controller poisoned")
                        .observe(outcome);
                    obs_metrics::set_ingestion_batch_size(new_size);
                    warn!(error = %e, batch = limit, next_batch = new_size, "getLedgers failed");
                    if total > 0 {
                        return Ok(total);
                    }
                    return Err(anyhow::anyhow!("getLedgers failed: {e}"));
                }
            };
            let latency = started.elapsed();

            let batch = contiguous_prefix(&result.ledgers, next);
            let committed = self.persist_batch(batch).await?;
            if let Some(last) = batch.last() {
                next = last.sequence.saturating_add(1);
            }
            total += committed;

            let lag = result.latest_ledger.saturating_sub(next.saturating_sub(1));
            obs_metrics::set_ingestion_lag_ledgers(lag);
            let new_size = self
                .batch_controller
                .lock()
                .expect("batch controller poisoned")
                .observe(FetchOutcome::Success { latency, lag });
            obs_metrics::set_ingestion_batch_size(new_size);

            if batch.is_empty() || lag == 0 {
                break;
            }
        }
        Ok(total)
    }

    /// Persist a batch of ledgers and advance the cursor in one transaction.
    /// The cursor only ever moves forward (`MAX`), so a stale or retried
    /// batch can't rewind it. Returns how many new ledger rows were written.
    async fn persist_batch(&self, ledgers: &[RpcLedger]) -> Result<u64> {
        let Some(last) = ledgers.last() else {
            return Ok(0);
        };
        let network = self.network.to_string();
        let mut tx = self.pool.begin().await.context("begin ingestion tx")?;
        let mut inserted = 0u64;
        for l in ledgers {
            let res = sqlx::query(
                "INSERT OR IGNORE INTO ledgers (sequence, hash, close_time, network) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(i64::try_from(l.sequence).unwrap_or(i64::MAX))
            .bind(&l.hash)
            .bind(&l.ledger_close_time)
            .bind(&network)
            .execute(&mut *tx)
            .await
            .context("insert ledger")?;
            inserted += res.rows_affected();
        }
        sqlx::query(
            "INSERT INTO ingestion_cursor (last_ledger_sequence, cursor, network, updated_at) \
             VALUES (?, ?, ?, CURRENT_TIMESTAMP) \
             ON CONFLICT(network) DO UPDATE SET \
               last_ledger_sequence = MAX(last_ledger_sequence, excluded.last_ledger_sequence), \
               cursor = excluded.cursor, \
               updated_at = CURRENT_TIMESTAMP",
        )
        .bind(i64::try_from(last.sequence).unwrap_or(i64::MAX))
        .bind(last.sequence.to_string())
        .bind(&network)
        .execute(&mut *tx)
        .await
        .context("advance ingestion cursor")?;
        tx.commit().await.context("commit ingestion tx")?;
        Ok(inserted)
    }

    async fn get_last_ledger(&self) -> Result<Option<u64>> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT last_ledger_sequence FROM ingestion_cursor WHERE network = ?",
        )
        .bind(self.network.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("read ingestion cursor")?;
        Ok(row.and_then(|(v,)| u64::try_from(v).ok()))
    }
}

/// Upper bound on fetch rounds per `run_ingestion` call so the scheduler
/// regains control periodically even during a long catch-up.
const MAX_ROUNDS_PER_RUN: usize = 100;

/// Longest prefix of `ledgers` that is contiguous starting at `expected`.
/// Anything after a hole is dropped and refetched next round, so a gap in an
/// RPC response can never cause the cursor to skip ledgers.
fn contiguous_prefix(ledgers: &[RpcLedger], expected: u64) -> &[RpcLedger] {
    let mut want = expected;
    let mut n = 0;
    for l in ledgers {
        if l.sequence != want {
            break;
        }
        want = want.saturating_add(1);
        n += 1;
    }
    &ledgers[..n]
}

#[cfg(test)]
mod tests {
    use super::*;

    use sqlx::sqlite::SqlitePoolOptions;

    fn ledger(seq: u64) -> RpcLedger {
        RpcLedger {
            hash: format!("h{seq}"),
            sequence: seq,
            ledger_close_time: "0".into(),
            header_xdr: None,
            metadata_xdr: None,
        }
    }

    #[test]
    fn contiguous_prefix_stops_at_hole() {
        let ls = vec![ledger(10), ledger(11), ledger(13), ledger(14)];
        assert_eq!(contiguous_prefix(&ls, 10).len(), 2);
        assert!(contiguous_prefix(&ls, 9).is_empty());
        assert!(contiguous_prefix(&[], 1).is_empty());
    }

    async fn test_service() -> LedgerIngestionService {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE ledgers (sequence INTEGER PRIMARY KEY, hash TEXT NOT NULL, \
             close_time TEXT NOT NULL, network TEXT NOT NULL DEFAULT 'mainnet')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE ingestion_cursor (id INTEGER PRIMARY KEY AUTOINCREMENT, \
             last_ledger_sequence INTEGER NOT NULL, cursor TEXT, updated_at TEXT, \
             network TEXT NOT NULL DEFAULT 'mainnet')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("CREATE UNIQUE INDEX idx_c ON ingestion_cursor(network)")
            .execute(&pool)
            .await
            .unwrap();
        let rpc = Arc::new(StellarRpcClient::new(String::new(), String::new(), true));
        LedgerIngestionService::new(
            Arc::clone(&rpc),
            Arc::new(FeeBumpTrackerService::new(pool.clone())),
            Arc::new(AccountMergeDetector::new(pool.clone(), rpc)),
            pool,
            StellarNetwork::Testnet,
        )
    }

    async fn count(svc: &LedgerIngestionService) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM ledgers")
            .fetch_one(&svc.pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn ingests_mock_range_and_resumes_without_duplicates() {
        use crate::rpc::mock_stellar::{MOCK_LATEST_LEDGER, MOCK_OLDEST_LEDGER};
        let svc = test_service().await;
        let expected = MOCK_LATEST_LEDGER - MOCK_OLDEST_LEDGER + 1;

        let n = svc.run_ingestion(7).await.unwrap();
        assert_eq!(n, expected);
        assert_eq!(svc.get_last_ledger().await.unwrap(), Some(MOCK_LATEST_LEDGER));
        assert_eq!(count(&svc).await as u64, expected);

        // A second run (restart) finds nothing new and writes no duplicates.
        assert_eq!(svc.run_ingestion(7).await.unwrap(), 0);
        assert_eq!(count(&svc).await as u64, expected);
    }

    #[tokio::test]
    async fn crash_mid_batch_resumes_from_last_committed_batch() {
        use crate::rpc::mock_stellar::{MOCK_LATEST_LEDGER, MOCK_OLDEST_LEDGER};
        let svc = test_service().await;
        // Simulate a committed first batch, then a crash before the next one.
        let first: Vec<_> = (MOCK_OLDEST_LEDGER..MOCK_OLDEST_LEDGER + 5).map(ledger).collect();
        svc.persist_batch(&first).await.unwrap();
        // Replaying the same batch (retry after crash) is a no-op.
        assert_eq!(svc.persist_batch(&first).await.unwrap(), 0);

        let n = svc.run_ingestion(10).await.unwrap();
        assert_eq!(n, MOCK_LATEST_LEDGER - MOCK_OLDEST_LEDGER + 1 - 5);
        assert_eq!(
            count(&svc).await as u64,
            MOCK_LATEST_LEDGER - MOCK_OLDEST_LEDGER + 1
        );
    }

    #[tokio::test]
    async fn cursor_never_moves_backwards() {
        let svc = test_service().await;
        svc.persist_batch(&[ledger(100)]).await.unwrap();
        svc.persist_batch(&[ledger(50)]).await.unwrap();
        assert_eq!(svc.get_last_ledger().await.unwrap(), Some(100));
    }

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
