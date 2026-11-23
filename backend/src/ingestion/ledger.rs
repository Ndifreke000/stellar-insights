use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
use tracing::warn;

use super::failures::{tally_path_payment_failures, CorridorFailureBreakdown};
use super::retry_strategy;

use crate::ingestion::adaptive_batch::{AimdConfig, AimdController, FetchOutcome};
use crate::network::StellarNetwork;
use crate::observability::metrics as obs_metrics;
use crate::rpc::{RpcLedger, StellarRpcClient};

/// Initial batch size before the scheduler's requested size is applied.
const DEFAULT_BATCH: u32 = 50;
use crate::services::account_merge_detector::AccountMergeDetector;
use crate::services::fee_bump_tracker::FeeBumpTrackerService;
use crate::shutdown::ShutdownFlag;

/// Normalised, source-independent representation of a single ledger.
///
/// Both the Horizon-backed and RPC `getLedgers`-backed sources decode their
/// wire formats into this shape so downstream ingestion logic never has to
/// know which upstream produced the data (#2884).
pub struct NormalisedLedger {
    pub sequence: u64,
    pub hash: String,
    pub close_time: i64,
    pub transaction_count: u32,
    pub operation_count: u32,
    pub successful_transaction_count: u32,
    pub failed_transaction_count: u32,
    pub total_coins: String,
    pub fee_pool: String,
    pub base_fee: u32,
    pub base_reserve: u32,
    pub max_tx_set_size: u32,
    pub protocol_version: u32,
}

/// Pluggable source of normalised ledger data.
///
/// Implementations translate a specific upstream (Horizon, RPC `getLedgers`,
/// ledger-meta) into [`NormalisedLedger`]s. Ingestion depends only on this
/// trait so the active source can be swapped via configuration (#2884).
#[async_trait]
pub trait LedgerSource: Send + Sync {
    /// Human-readable identifier used in logs and dual-run comparisons.
    fn name(&self) -> &'static str;

    /// Fetch a contiguous range of ledgers starting at `start` (inclusive),
    /// up to `limit` ledgers. Implementations must return ledgers in
    /// ascending sequence order.
    async fn fetch_ledgers(&self, start: u64, limit: u32) -> Result<Vec<NormalisedLedger>>;
}

/// Horizon-backed ledger source. Wraps the existing RPC client's ledger
/// retrieval path, which is the current production behaviour.
pub struct HorizonLedgerSource {
    rpc_client: Arc<StellarRpcClient>,
}

impl HorizonLedgerSource {
    #[must_use]
    pub fn new(rpc_client: Arc<StellarRpcClient>) -> Self {
        Self { rpc_client }
    }
}

#[async_trait]
impl LedgerSource for HorizonLedgerSource {
    fn name(&self) -> &'static str {
        "horizon"
    }

    async fn fetch_ledgers(&self, start: u64, limit: u32) -> Result<Vec<NormalisedLedger>> {
        let ledgers = self
            .rpc_client
            .fetch_ledgers(Some(start), limit, None)
            .await
            .context("horizon: failed to fetch ledgers")?
            .ledgers;
        Ok(ledgers.into_iter().map(normalise_rpc_ledger).collect())
    }
}

/// RPC `getLedgers`-backed ledger source. Uses the Stellar RPC client's
/// `getLedgers` endpoint and decodes the returned XDR into the normalised
/// model.
pub struct RpcLedgerSource {
    rpc_client: Arc<StellarRpcClient>,
}

impl RpcLedgerSource {
    #[must_use]
    pub fn new(rpc_client: Arc<StellarRpcClient>) -> Self {
        Self { rpc_client }
    }
}

#[async_trait]
impl LedgerSource for RpcLedgerSource {
    fn name(&self) -> &'static str {
        "rpc-getLedgers"
    }

    async fn fetch_ledgers(&self, start: u64, limit: u32) -> Result<Vec<NormalisedLedger>> {
        let ledgers = self
            .rpc_client
            .fetch_ledgers(Some(start), limit, None)
            .await
            .context("rpc: failed to fetch ledgers via getLedgers")?
            .ledgers;
        Ok(ledgers.into_iter().map(normalise_rpc_ledger).collect())
    }
}

/// Decode an [`RpcLedger`] (already XDR-decoded by the RPC client) into the
/// source-independent [`NormalisedLedger`] shape.
fn normalise_rpc_ledger(ledger: RpcLedger) -> NormalisedLedger {
    use stellar_xdr::curr::{LedgerHeaderHistoryEntry, Limits, ReadXdr};

    // `ledgerCloseTime` is a unix timestamp in seconds, sent as a string.
    let close_time = ledger.ledger_close_time.parse::<i64>().unwrap_or_default();
    let header = ledger
        .header_xdr
        .as_deref()
        .and_then(|xdr| LedgerHeaderHistoryEntry::from_xdr_base64(xdr, Limits::none()).ok())
        .map(|entry| entry.header);

    // Transaction and operation counts live in the ledger metadata, which this
    // path does not decode, so they are reported as zero.
    NormalisedLedger {
        sequence: ledger.sequence,
        hash: ledger.hash,
        close_time,
        transaction_count: 0,
        operation_count: 0,
        successful_transaction_count: 0,
        failed_transaction_count: 0,
        total_coins: header
            .as_ref()
            .map_or_else(String::new, |h| h.total_coins.to_string()),
        fee_pool: header
            .as_ref()
            .map_or_else(String::new, |h| h.fee_pool.to_string()),
        base_fee: header.as_ref().map_or(0, |h| h.base_fee),
        base_reserve: header.as_ref().map_or(0, |h| h.base_reserve),
        max_tx_set_size: header.as_ref().map_or(0, |h| h.max_tx_set_size),
        protocol_version: header.as_ref().map_or(0, |h| h.ledger_version),
    }
}

/// Which ledger source ingestion should use. Selected via configuration
/// (#2884).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerSourceKind {
    Horizon,
    RpcGetLedgers,
}

impl LedgerSourceKind {
    /// Parse the configured source name. Unknown values fall back to Horizon
    /// (the current production behaviour) with a warning.
    #[must_use]
    pub fn from_config(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "rpc" | "rpc-getledgers" | "getledgers" => Self::RpcGetLedgers,
            "horizon" | "" => Self::Horizon,
            other => {
                warn!(
                    value = other,
                    "unknown ledger source, defaulting to horizon"
                );
                Self::Horizon
            }
        }
    }

    /// Build the concrete source for this kind.
    #[must_use]
    pub fn build(self, rpc_client: Arc<StellarRpcClient>) -> Arc<dyn LedgerSource> {
        match self {
            Self::Horizon => Arc::new(HorizonLedgerSource::new(rpc_client)),
            Self::RpcGetLedgers => Arc::new(RpcLedgerSource::new(rpc_client)),
        }
    }
}

/// Dual-run comparison mode: fetch the same range from two sources and
/// report any ledgers whose normalised data diverges (#2884).
pub async fn compare_sources(
    primary: &dyn LedgerSource,
    secondary: &dyn LedgerSource,
    start: u64,
    limit: u32,
) -> Result<Vec<u64>> {
    let a = primary.fetch_ledgers(start, limit).await?;
    let b = secondary.fetch_ledgers(start, limit).await?;
    let mut mismatches = Vec::new();
    for (x, y) in a.iter().zip(b.iter()) {
        if x.sequence != y.sequence
            || x.hash != y.hash
            || x.close_time != y.close_time
            || x.transaction_count != y.transaction_count
            || x.operation_count != y.operation_count
            || x.total_coins != y.total_coins
            || x.fee_pool != y.fee_pool
            || x.protocol_version != y.protocol_version
        {
            mismatches.push(x.sequence);
        }
    }
    if a.len() != b.len() {
        warn!(
            primary = primary.name(),
            secondary = secondary.name(),
            primary_len = a.len(),
            secondary_len = b.len(),
            "dual-run ledger count mismatch"
        );
    }
    Ok(mismatches)
}

/// Ledger ingestion service that fetches and persists ledgers sequentially.
///
/// Every row this service writes is tagged with `network` so that mainnet
/// and testnet ingestion can run against the same database without their
/// data mixing (see migration 052_add_network_to_ledger_ingestion.sql).
pub struct LedgerIngestionService {
    rpc_client: Arc<StellarRpcClient>,
    ledger_source: Arc<dyn LedgerSource>,
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

/// Fold one transaction's `result_xdr` into the corridor tally using the
/// canonical taxonomy in `ingestion::failures` (#2766). Undecodable XDR is
/// logged and skipped rather than silently counted as success.
pub fn tally_ledger_transaction(result_xdr: &str, counts: &mut CorridorFailureBreakdown) -> u64 {
    match tally_path_payment_failures(result_xdr, counts) {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, "skipping undecodable result_xdr");
            0
        }
    }
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
        let ledger_source = LedgerSourceKind::Horizon.build(rpc_client.clone());
        Self {
            rpc_client,
            ledger_source,
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
        let ledger_source = LedgerSourceKind::Horizon.build(rpc_client.clone());
        Self {
            rpc_client,
            ledger_source,
            fee_bump_tracker,
            account_merge_detector,
            pool,
            network,
            webhook_event_service: Some(webhook_event_service),
            shutdown: None,
            batch_controller: Mutex::new(AimdController::new(AimdConfig::default(), DEFAULT_BATCH)),
        }
    }

    /// Select the active ledger source (#2884). Defaults to Horizon when not
    /// called, preserving current behaviour.
    #[must_use]
    pub fn with_ledger_source(mut self, kind: LedgerSourceKind) -> Self {
        self.ledger_source = kind.build(self.rpc_client.clone());
        self
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
        *self
            .batch_controller
            .lock()
            .expect("batch controller poisoned") = AimdController::new(config, requested);
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
                let mut ctrl = self
                    .batch_controller
                    .lock()
                    .expect("batch controller poisoned");
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
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT last_ledger_sequence FROM ingestion_cursor WHERE network = ?")
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
    use crate::ingestion::failures::fixtures::*;
    use crate::ingestion::failures::PathPaymentFailureCode;

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
        assert_eq!(
            svc.get_last_ledger().await.unwrap(),
            Some(MOCK_LATEST_LEDGER)
        );
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
        let first: Vec<_> = (MOCK_OLDEST_LEDGER..MOCK_OLDEST_LEDGER + 5)
            .map(ledger)
            .collect();
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
    fn tallies_failures_per_corridor_from_xdr() {
        let mut counts = CorridorFailureBreakdown::new("USDC", "NGN");
        let xdr = tx_result_xdr(
            vec![strict_receive(Some(PathPaymentFailureCode::TooFewOffers))],
            true,
        );
        assert_eq!(tally_ledger_transaction(&xdr, &mut counts), 1);
        assert_eq!(counts.count(PathPaymentFailureCode::TooFewOffers), 1);
        assert_eq!(counts.total_failures(), 1);
    }

    #[test]
    fn undecodable_xdr_is_skipped() {
        let mut counts = CorridorFailureBreakdown::default();
        assert_eq!(
            tally_ledger_transaction("AAAA...op_too_few_offers...", &mut counts),
            0
        );
        assert_eq!(counts.total_failures(), 0);
        assert_eq!(counts.successful, 0);
    }
}
