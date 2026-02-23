// I'm exporting the ledger ingestion module as required by issue #2
pub mod ledger;

use anyhow::{Context, Result};
use serde::Serialize;
use std::sync::Arc;
use tracing::{info, warn};

use crate::database::Database;
use crate::rpc::StellarRpcClient;

pub struct DataIngestionService {
    rpc_client: Arc<StellarRpcClient>,
    db: Arc<Database>,
}

impl DataIngestionService {
    pub fn new(rpc_client: Arc<StellarRpcClient>, db: Arc<Database>) -> Self {
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
                Ok(_) => info!("Updated metrics for anchor: {}", anchor.name),
                Err(e) => warn!("Failed to update anchor {}: {}", anchor.name, e),
            }
        }

        Ok(())
    }

    /// Process metrics for a single anchor
    async fn process_anchor_metrics(&self, account_id: &str) -> Result<()> {
        let payments = self
            .rpc_client
            .fetch_account_payments(account_id, 100)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        if payments.is_empty() {
            return Ok(());
        }

        let mut successful = 0;
        let failed = 0;
        let mut total_volume = 0.0;
        let settlement_times = Vec::new(); // Removed mut as it's never pushed to

        for payment in &payments {
            let amount: f64 = payment.amount.parse().unwrap_or(0.0);
            total_volume += amount;

            successful += 1;
        }

        let total_transactions = (successful + failed) as i64;
        let success_rate = if total_transactions > 0 {
            (successful as f64 / total_transactions as f64) * 100.0
        } else {
            0.0
        };

        let reliability_score = self.calculate_reliability_score(success_rate, failed as i64);

        let avg_settlement_time = if !settlement_times.is_empty() {
            settlement_times.iter().sum::<i32>() / settlement_times.len() as i32
        } else {
            1000
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
                successful_transactions: successful as i64,
                failed_transactions: failed as i64,
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
        let health = self
            .rpc_client
            .check_health()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

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

impl DataIngestionService {
    // ... (existing methods remain, adding new one below)

    pub async fn get_ingestion_status(&self) -> Result<IngestionStatus> {
        // We get local state
        let cursor_row: Option<(i64,)> =
            sqlx::query_as("SELECT last_ledger_sequence FROM ingestion_cursor WHERE id = 1")
                .fetch_optional(self.db.pool())
                .await?;

        let last_ingested = cursor_row.map(|r| r.0 as u64).unwrap_or(0);

        // We get network state
        let health = self
            .rpc_client
            .check_health()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(IngestionStatus {
            last_ingested_ledger: last_ingested,
            network_latest_ledger: health.latest_ledger,
        })
    }
}
