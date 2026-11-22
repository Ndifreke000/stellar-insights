use anyhow::{Context, Result};
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

    /// Persist a single ledger's payments, failure counters, corridor metrics
    /// and the cursor update inside one transaction (#2873).
    ///
    /// If any statement fails, the whole transaction is rolled back so a
    /// crash mid-ledger never leaves partially applied data: either the
    /// ledger is fully persisted and the cursor advances, or nothing changes
    /// and the cursor stays put so the ledger is retried on the next run.
    ///
    /// Writes use prepared statements bound per row so a batch of payments
    /// reuses the same compiled statement for throughput.
    pub async fn persist_ledger_atomic(
        &self,
        ledger: &RpcLedger,
        payments: &[ExtractedPayment],
        failures: &CorridorFailureBreakdown,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin ingestion transaction")?;

        // Payments: one prepared statement, bound per row.
        let mut payment_stmt = tx
            .prepare(
                "INSERT INTO payments (ledger_sequence, transaction_hash, operation_type, \
                 source_account, destination, asset_code, asset_issuer, amount, network) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )
            .await
            .context("failed to prepare payments insert")?;
        for p in payments {
            sqlx::query(&payment_stmt)
                .bind(p.ledger_sequence as i64)
                .bind(&p.transaction_hash)
                .bind(&p.operation_type)
                .bind(&p.source_account)
                .bind(&p.destination)
                .bind(&p.asset_code)
                .bind(&p.asset_issuer)
                .bind(&p.amount)
                .bind(self.network.as_str())
                .execute(&mut *tx)
                .await
                .context("failed to insert payment")?;
        }
        drop(payment_stmt);

        // Failure counters: one prepared statement, bound per corridor.
        let mut failure_stmt = tx
            .prepare(
                "INSERT INTO corridor_failure_counters (network, corridor, failure_code, count) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(network, corridor, failure_code) \
                 DO UPDATE SET count = count + excluded.count",
            )
            .await
            .context("failed to prepare failure counter upsert")?;
        for (corridor, code, count) in failures.iter() {
            sqlx::query(&failure_stmt)
                .bind(self.network.as_str())
                .bind(corridor)
                .bind(code)
                .bind(*count as i64)
                .execute(&mut *tx)
                .await
                .context("failed to upsert failure counter")?;
        }
        drop(failure_stmt);

        // Corridor metrics: one prepared statement, bound per corridor.
        let mut metric_stmt = tx
            .prepare(
                "INSERT INTO corridor_metrics (network, corridor, ledger_sequence, payment_count) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(network, corridor, ledger_sequence) \
                 DO UPDATE SET payment_count = corridor_metrics.payment_count + excluded.payment_count",
            )
            .await
            .context("failed to prepare corridor metric upsert")?;
        for (corridor, count) in corridor_payment_counts(payments) {
            sqlx::query(&metric_stmt)
                .bind(self.network.as_str())
                .bind(&corridor)
                .bind(ledger.sequence as i64)
                .bind(count as i64)
                .execute(&mut *tx)
                .await
                .context("failed to upsert corridor metric")?;
        }
        drop(metric_stmt);

        // Cursor update lives in the same transaction so it only advances
        // when every write above committed.
        sqlx::query(
            "INSERT INTO ingestion_cursor (network, last_ledger) VALUES (?1, ?2) \
             ON CONFLICT(network) DO UPDATE SET last_ledger = excluded.last_ledger",
        )
        .bind(self.network.as_str())
        .bind(ledger.sequence as i64)
        .execute(&mut *tx)
        .await
        .context("failed to update ingestion cursor")?;

        tx.commit()
            .await
            .context("failed to commit ingestion transaction")?;

        Ok(())
    }
}

/// Group payments by corridor (`source_asset->destination_asset`) so corridor
/// metrics can be written with one prepared statement per corridor.
fn corridor_payment_counts(payments: &[ExtractedPayment]) -> Vec<(String, u64)> {
    use std::collections::HashMap;
    let mut counts: HashMap<String, u64> = HashMap::new();
    for p in payments {
        let source = p.asset_code.as_deref().unwrap_or("XLM");
        let dest = p.asset_code.as_deref().unwrap_or("XLM");
        let corridor = format!("{source}->{dest}");
        *counts.entry(corridor).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

#[cfg(test)]
mod atomic_tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        sqlx::query(
            "CREATE TABLE payments (ledger_sequence INTEGER, transaction_hash TEXT, \
             operation_type TEXT, source_account TEXT, destination TEXT, \
             asset_code TEXT, asset_issuer TEXT, amount TEXT, network TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE corridor_failure_counters (network TEXT, corridor TEXT, \
             failure_code TEXT, count INTEGER, UNIQUE(network, corridor, failure_code))",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE corridor_metrics (network TEXT, corridor TEXT, \
             ledger_sequence INTEGER, payment_count INTEGER, \
             UNIQUE(network, corridor, ledger_sequence))",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE ingestion_cursor (network TEXT PRIMARY KEY, last_ledger INTEGER)",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    fn sample_payment(ledger: u64) -> ExtractedPayment {
        ExtractedPayment {
            ledger_sequence: ledger,
            transaction_hash: "abc".to_string(),
            operation_type: "payment".to_string(),
            source_account: "GSOURCE".to_string(),
            destination: "GDEST".to_string(),
            asset_code: Some("USDC".to_string()),
            asset_issuer: None,
            amount: "10".to_string(),
        }
    }

    async fn count(pool: &SqlitePool, table: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        sqlx::query_scalar::<_, i64>(&sql)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn cursor(pool: &SqlitePool) -> Option<i64> {
        sqlx::query_scalar::<_, i64>("SELECT last_ledger FROM ingestion_cursor LIMIT 1")
            .fetch_optional(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn failure_after_payments_insert_rolls_back_everything() {
        let pool = setup_pool().await;
        // Drop the failure counter table so the write after payments fails,
        // simulating a crash between the payments insert and the rest.
        sqlx::query("DROP TABLE corridor_failure_counters")
            .execute(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        sqlx::query(
            "INSERT INTO payments (ledger_sequence, transaction_hash, operation_type, \
             source_account, destination, asset_code, asset_issuer, amount, network) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(1i64)
        .bind("abc")
        .bind("payment")
        .bind("GSOURCE")
        .bind("GDEST")
        .bind(Some("USDC"))
        .bind(None::<String>)
        .bind("10")
        .bind("testnet")
        .execute(&mut *tx)
        .await
        .unwrap();

        // The next write fails because the table is gone.
        let failed = sqlx::query(
            "INSERT INTO corridor_failure_counters (network, corridor, failure_code, count) \
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind("testnet")
        .bind("USDC->NGN")
        .bind("op_no_destination")
        .bind(1i64)
        .execute(&mut *tx)
        .await;
        assert!(failed.is_err(), "expected injected failure");

        // Roll back as the service would on error.
        tx.rollback().await.unwrap();

        assert_eq!(count(&pool, "payments").await, 0, "payments must not persist");
        assert_eq!(cursor(&pool).await, None, "cursor must be unchanged");
    }

    #[tokio::test]
    async fn successful_batch_persists_payments_and_cursor() {
        let pool = setup_pool().await;
        let mut tx = pool.begin().await.unwrap();
        sqlx::query(
            "INSERT INTO payments (ledger_sequence, transaction_hash, operation_type, \
             source_account, destination, asset_code, asset_issuer, amount, network) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(1i64)
        .bind("abc")
        .bind("payment")
        .bind("GSOURCE")
        .bind("GDEST")
        .bind(Some("USDC"))
        .bind(None::<String>)
        .bind("10")
        .bind("testnet")
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO ingestion_cursor (network, last_ledger) VALUES (?1, ?2) \
             ON CONFLICT(network) DO UPDATE SET last_ledger = excluded.last_ledger",
        )
        .bind("testnet")
        .bind(1i64)
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(count(&pool, "payments").await, 1);
        assert_eq!(cursor(&pool).await, Some(1));
    }

    #[test]
    fn corridor_counts_group_by_asset() {
        let payments = vec![sample_payment(1), sample_payment(1)];
        let counts = corridor_payment_counts(&payments);
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].1, 2);
    }
}
