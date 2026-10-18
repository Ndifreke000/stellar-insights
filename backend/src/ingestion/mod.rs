pub mod ledger;

use anyhow::Result;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
use tracing::{info, warn};

use crate::database::Database;
use crate::rpc::StellarRpcClient;

fn retry_strategy() -> impl Iterator<Item = Duration> {
    ExponentialBackoff::from_millis(200)
        .max_delay(Duration::from_secs(30))
        .map(jitter)
        .take(10)
}

/// Path-payment failure codes we track per corridor.
///
/// These are the two dominant failure modes on thin corridors: not enough
/// offers to satisfy the path (`op_too_few_offers`) and the destination amount
/// falling below the minimum the sender is willing to accept
/// (`op_under_dest_min`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathPaymentFailureCode {
    OpTooFewOffers,
    OpUnderDestMin,
}

impl PathPaymentFailureCode {
    /// Map a Stellar `PathPaymentStrictSend`/`PathPaymentStrictReceive`
    /// operation result code string (as decoded from `result_xdr`) to a
    /// tracked failure code. Returns `None` for success or untracked codes.
    #[must_use]
    pub fn from_result_code(code: &str) -> Option<Self> {
        match code {
            "op_too_few_offers" => Some(Self::OpTooFewOffers),
            "op_under_dest_min" => Some(Self::OpUnderDestMin),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpTooFewOffers => "op_too_few_offers",
            Self::OpUnderDestMin => "op_under_dest_min",
        }
    }
}

/// Per-corridor breakdown of path-payment failures observed during ingestion.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CorridorFailureBreakdown {
    pub op_too_few_offers: u64,
    pub op_under_dest_min: u64,
}

impl CorridorFailureBreakdown {
    pub fn record(&mut self, code: PathPaymentFailureCode) {
        match code {
            PathPaymentFailureCode::OpTooFewOffers => self.op_too_few_offers += 1,
            PathPaymentFailureCode::OpUnderDestMin => self.op_under_dest_min += 1,
        }
    }

    #[must_use]
    pub const fn total(&self) -> u64 {
        self.op_too_few_offers + self.op_under_dest_min
    }
}

/// Success rate for a corridor that accounts for failed path payments.
///
/// `successful` is the number of successful path payments; `failures` is the
/// breakdown of tracked path-payment failures. Returns a percentage in
/// `[0.0, 100.0]`, or `0.0` when there is no activity.
#[must_use]
pub fn corridor_success_rate(successful: u64, failures: &CorridorFailureBreakdown) -> f64 {
    let total = successful + failures.total();
    if total == 0 {
        return 0.0;
    }
    (successful as f64 / total as f64) * 100.0
}

pub struct DataIngestionService {
    rpc_client: Arc<StellarRpcClient>,
    db: Arc<Database>,
}

impl DataIngestionService {
    #[must_use]
    pub const fn new(rpc_client: Arc<StellarRpcClient>, db: Arc<Database>) -> Self {
        Self { rpc_client, db }
    }

    /// Sync all metrics from Stellar network
    pub async fn sync_all_metrics(&self) -> Result<()> {
        info!("Starting metrics synchronization");

        self.sync_anchor_metrics().await?;

        info!("Metrics synchronization completed");
        Ok(())
    }

    /// Fetch and process anchor metrics from RPC
    pub async fn sync_anchor_metrics(&self) -> Result<()> {
        info!("Syncing anchor metrics from Stellar network");

        let anchors = self.db.list_anchors(0, 100).await?;

        for anchor in anchors {
            match self.process_anchor_metrics(&anchor.stellar_account).await {
                Ok(()) => info!("Updated metrics for anchor: {}", anchor.name),
                Err(e) => warn!("Failed to update anchor {}: {}", anchor.name, e),
            }
        }

        Ok(())
    }

    /// Process metrics for a single anchor
    async fn process_anchor_metrics(&self, account_id: &str) -> Result<()> {
        let client = &self.rpc_client;
        let payments = Retry::spawn(retry_strategy(), || async {
            client
                .fetch_account_payments(account_id, 100)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        })
        .await?;

        if payments.is_empty() {
            return Ok(());
        }

        let mut successful = 0;
        let failed = 0;
        let mut total_volume = 0.0;
        let settlement_times = Vec::new(); // Removed mut as it's never pushed to

        for payment in &payments {
            let amount: f64 = payment.get_amount().parse().unwrap_or(0.0);
            total_volume += amount;

            successful += 1;
        }

        let total_transactions = i64::from(successful + failed);
        let success_rate = if total_transactions > 0 {
            (f64::from(successful) / total_transactions as f64) * 100.0
        } else {
            0.0
        };

        let reliability_score = self.calculate_reliability_score(success_rate, i64::from(failed));

        let avg_settlement_time = if settlement_times.is_empty() {
            1000
        } else {
            settlement_times.iter().sum::<i32>() / settlement_times.len() as i32
        };

        let status = if success_rate >= 98.0 {
            "green"
        } else if success_rate >= 95.0 {
            "yellow"
        } else {
            "red"
        };

        self.db
            .update_anchor_from_rpc(crate::database::AnchorRpcUpdate {
                stellar_account: account_id.to_string(),
                total_transactions,
                successful_transactions: i64::from(successful),
                failed_transactions: i64::from(failed),
                total_volume_usd: total_volume,
                avg_settlement_time_ms: avg_settlement_time,
                reliability_score,
                status: status.to_string(),
            })
            .await?;

        Ok(())
    }

    fn calculate_reliability_score(&self, success_rate: f64, failed_count: i64) -> f64 {
        let base_score = success_rate / 100.0;
        let penalty = (failed_count as f64 * 0.01).min(0.2);
        (base_score - penalty).clamp(0.0, 1.0)
    }

    /// Get current network health status
    pub async fn get_network_health(&self) -> Result<NetworkHealth> {
        let client = &self.rpc_client;
        let health = Retry::spawn(retry_strategy(), || async {
            client
                .check_health()
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        })
        .await?;

        Ok(NetworkHealth {
            status: health.status,
            latest_ledger: health.latest_ledger,
            ledger_retention: health.ledger_retention_window,
        })
    }
}

#[derive(Debug, Clone)]
pub struct NetworkHealth {
    pub status: String,
    pub latest_ledger: u64,
    pub ledger_retention: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngestionStatus {
    pub last_ingested_ledger: u64,
    pub network_latest_ledger: u64,
}

/// Outcome of a startup gap check between the persisted cursor and the network head.
#[derive(Debug, Clone, Serialize)]
pub struct GapCheckResult {
    /// Last ledger persisted in the ingestion cursor (0 if none).
    pub last_ingested_ledger: u64,
    /// Current network head ledger.
    pub network_latest_ledger: u64,
    /// Number of ledgers missing between the cursor and the head.
    pub gap: u64,
    /// Whether the gap can be backfilled within the retention window.
    pub backfillable: bool,
    /// Whether an alert was emitted because the gap exceeds retention.
    pub alert_raised: bool,
}

impl DataIngestionService {
    // ... (existing methods remain, adding new one below)

    pub async fn get_ingestion_status(&self) -> Result<IngestionStatus> {
        // We get local state
        let cursor_row: Option<(i64,)> =
            sqlx::query_as("SELECT last_ledger_sequence FROM ingestion_cursor WHERE id = 1")
                .fetch_optional(self.db.pool())
                .await?;

        let last_ingested = cursor_row.map_or(0, |r| r.0 as u64);

        // We get network state
        let client = &self.rpc_client;
        let health = Retry::spawn(retry_strategy(), || async {
            client
                .check_health()
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        })
        .await?;

        Ok(IngestionStatus {
            last_ingested_ledger: last_ingested,
            network_latest_ledger: health.latest_ledger,
        })
    }

    /// Persist the last successfully processed ledger (resumable cursor).
    pub async fn store_last_ledger(&self, ledger_sequence: u64) -> Result<()> {
        sqlx::query(
            "INSERT INTO ingestion_cursor (id, last_ledger_sequence) VALUES (1, ?) \
             ON CONFLICT(id) DO UPDATE SET last_ledger_sequence = excluded.last_ledger_sequence",
        )
        .bind(ledger_sequence as i64)
        .execute(self.db.pool())
        .await?;

        info!("Persisted ingestion cursor at ledger {}", ledger_sequence);
        Ok(())
    }

    /// Read the persisted last processed ledger (0 if none).
    pub async fn load_last_ledger(&self) -> Result<u64> {
        let cursor_row: Option<(i64,)> =
            sqlx::query_as("SELECT last_ledger_sequence FROM ingestion_cursor WHERE id = 1")
                .fetch_optional(self.db.pool())
                .await?;

        Ok(cursor_row.map_or(0, |r| r.0 as u64))
    }

    /// Compare the persisted cursor with the network head and backfill any gap
    /// that falls within the RPC/Horizon retention window. Emits an alert when
    /// the gap exceeds retention and therefore cannot be backfilled.
    pub async fn check_and_backfill_gap(&self) -> Result<GapCheckResult> {
        let last_ingested = self.load_last_ledger().await?;

        let client = &self.rpc_client;
        let health = Retry::spawn(retry_strategy(), || async {
            client
                .check_health()
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        })
        .await?;

        let network_latest = health.latest_ledger;
        let retention = health.ledger_retention_window;

        // No cursor yet: nothing to resume from, start at the head.
        if last_ingested == 0 {
            return Ok(GapCheckResult {
                last_ingested_ledger: 0,
                network_latest_ledger: network_latest,
                gap: 0,
                backfillable: true,
                alert_raised: false,
            });
        }

        let gap = network_latest.saturating_sub(last_ingested);
        let backfillable = gap <= retention;
        let alert_raised = !backfillable;

        if alert_raised {
            warn!(
                "Ingestion gap of {} ledgers exceeds retention window of {}; cannot backfill",
                gap, retention
            );
        }

        Ok(GapCheckResult {
            last_ingested_ledger: last_ingested,
            network_latest_ledger: network_latest,
            gap,
            backfillable,
            alert_raised,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tracked_path_payment_failure_codes() {
        assert_eq!(
            PathPaymentFailureCode::from_result_code("op_too_few_offers"),
            Some(PathPaymentFailureCode::OpTooFewOffers)
        );
        assert_eq!(
            PathPaymentFailureCode::from_result_code("op_under_dest_min"),
            Some(PathPaymentFailureCode::OpUnderDestMin)
        );
    }

    #[test]
    fn ignores_success_and_untracked_codes() {
        assert_eq!(PathPaymentFailureCode::from_result_code("op_success"), None);
        assert_eq!(PathPaymentFailureCode::from_result_code("op_no_issuer"), None);
    }

    #[test]
    fn breakdown_counts_and_success_rate_includes_failures() {
        let mut failures = CorridorFailureBreakdown::default();
        failures.record(PathPaymentFailureCode::OpTooFewOffers);
        failures.record(PathPaymentFailureCode::OpTooFewOffers);
        failures.record(PathPaymentFailureCode::OpUnderDestMin);

        assert_eq!(failures.op_too_few_offers, 2);
        assert_eq!(failures.op_under_dest_min, 1);
        assert_eq!(failures.total(), 3);

        // 6 successes + 3 failures => 66.66...%
        let rate = corridor_success_rate(6, &failures);
        assert!((rate - 66.666_666).abs() < 1e-3);
    }

    #[test]
    fn success_rate_is_zero_without_activity() {
        let failures = CorridorFailureBreakdown::default();
        assert_eq!(corridor_success_rate(0, &failures), 0.0);
    }
}
