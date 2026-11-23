pub mod adaptive_batch;
pub mod failures;
pub mod ledger;

use anyhow::Result;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
use tracing::{info, warn};

use crate::cache_invalidation::CacheInvalidationService;
use crate::database::Database;
use crate::rpc::StellarRpcClient;

/// Shared RPC retry policy for every ingestion path (#2766: previously
/// duplicated in `ledger.rs`).
pub(crate) fn retry_strategy() -> impl Iterator<Item = Duration> {
    ExponentialBackoff::from_millis(200)
        .max_delay(Duration::from_secs(30))
        .map(jitter)
        .take(10)
}

// The single path-payment failure taxonomy lives in `failures` (#2766).
pub use failures::{
    corridor_success_rate, decode_result_xdr, tally_path_payment_failures,
    CorridorFailureBreakdown, PathPaymentFailureCode, PathPaymentKind, PathPaymentOutcome,
};

pub struct DataIngestionService {
    rpc_client: Arc<StellarRpcClient>,
    db: Arc<Database>,
    /// Cache invalidation service wired in at construction time so ingestion
    /// can evict stale corridor/anchor/metrics keys immediately after each
    /// batch rather than waiting for TTL expiry (#2790).
    cache_invalidation: Option<Arc<CacheInvalidationService>>,
}

impl DataIngestionService {
    #[must_use]
    pub fn new(rpc_client: Arc<StellarRpcClient>, db: Arc<Database>) -> Self {
        Self {
            rpc_client,
            db,
            cache_invalidation: None,
        }
    }

    /// Attach a [`CacheInvalidationService`] so each ingestion batch evicts
    /// stale cache entries immediately rather than relying on TTL expiry (#2790).
    #[must_use]
    pub fn with_cache_invalidation(mut self, svc: Arc<CacheInvalidationService>) -> Self {
        self.cache_invalidation = Some(svc);
        self
    }

    /// Invalidate all corridor and anchor caches after a successful ingestion
    /// batch. Logs but does not propagate cache errors — a cache miss is
    /// preferable to blocking ingestion (#2790).
    async fn invalidate_after_ingestion(&self) {
        let Some(ref svc) = self.cache_invalidation else {
            return;
        };
        if let Err(e) = svc.invalidate_corridors().await {
            warn!("Cache invalidation (corridors) failed after ingestion: {e}");
        }
        if let Err(e) = svc.invalidate_preflight_corridors().await {
            warn!("Cache invalidation (preflight) failed after ingestion: {e}");
        }
        if let Err(e) = svc.invalidate_metrics().await {
            warn!("Cache invalidation (metrics) failed after ingestion: {e}");
        }
    }

    /// Sync all metrics from Stellar network
    pub async fn sync_all_metrics(&self) -> Result<()> {
        info!("Starting metrics synchronization");

        self.sync_anchor_metrics().await?;

        // Evict stale cache entries now that fresh data has landed (#2790).
        self.invalidate_after_ingestion().await;

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
        let payments = Retry::start(retry_strategy(), || async {
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
        let health = Retry::start(retry_strategy(), || async {
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
        let health = Retry::start(retry_strategy(), || async {
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

    /// Compare the persisted ingestion cursor against the network head and
    /// report whether the gap can be backfilled within the retention window.
    pub async fn check_ingestion_gap(&self) -> Result<GapCheckResult> {
        let status = self.get_ingestion_status().await?;
        let gap = status
            .network_latest_ledger
            .saturating_sub(status.last_ingested_ledger);

        let client = &self.rpc_client;
        let health = Retry::start(retry_strategy(), || async {
            client
                .check_health()
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        })
        .await?;

        let backfillable = gap <= health.ledger_retention_window;
        let alert_raised = !backfillable;

        if alert_raised {
            warn!(
                "Ingestion gap of {} ledgers exceeds retention window of {}",
                gap, health.ledger_retention_window
            );
        }

        Ok(GapCheckResult {
            last_ingested_ledger: status.last_ingested_ledger,
            network_latest_ledger: status.network_latest_ledger,
            gap,
            backfillable,
            alert_raised,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::failures::fixtures::*;
    use super::*;
    use crate::ingestion::ledger::tally_ledger_transaction;

    /// Whichever ingestion path sees a transaction, the corridor success
    /// rate must come out identical because both go through `failures`.
    #[test]
    fn success_rate_identical_across_ingestion_paths() {
        let txs = [
            tx_result_xdr(vec![strict_send(None)], false),
            tx_result_xdr(
                vec![strict_send(Some(PathPaymentFailureCode::UnderDestmin))],
                true,
            ),
            tx_result_xdr(
                vec![strict_receive(Some(PathPaymentFailureCode::OverSendmax))],
                true,
            ),
            tx_result_xdr(vec![strict_receive(None)], false),
        ];

        let mut via_mod = CorridorFailureBreakdown::new("USDC", "NGN");
        for tx in &txs {
            tally_path_payment_failures(tx, &mut via_mod).unwrap();
        }

        let mut via_ledger = CorridorFailureBreakdown::new("USDC", "NGN");
        for tx in &txs {
            tally_ledger_transaction(tx, &mut via_ledger);
        }

        assert_eq!(via_mod, via_ledger);
        assert_eq!(via_mod.success_rate(), via_ledger.success_rate());
        assert!((via_mod.success_rate() - 50.0).abs() < 1e-9);
    }
}
