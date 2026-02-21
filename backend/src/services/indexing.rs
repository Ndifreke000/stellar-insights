use anyhow::{Context, Result};
use chrono::DateTime;
use std::sync::Arc;
use tracing::info;

use crate::database::Database;
use crate::models::PaymentRecord;
use crate::rpc::StellarRpcClient;

pub struct IndexingService {
    rpc_client: Arc<StellarRpcClient>,
    db: Arc<Database>,
}

impl IndexingService {
    pub fn new(rpc_client: Arc<StellarRpcClient>, db: Arc<Database>) -> Self {
        Self { rpc_client, db }
    }

    /// Run payment ingestion starting from the last saved cursor
    pub async fn run_payment_ingestion(&self) -> Result<()> {
        let task_name = "payment_ingestion";
        let last_cursor = self.db.get_ingestion_cursor(task_name).await?;

        info!(
            "Starting payment ingestion. Last cursor: {:?}",
            last_cursor.as_deref().unwrap_or("none")
        );

        // Fetch payments from Horizon
        let payments = self
            .rpc_client
            .fetch_payments(100, last_cursor.as_deref())
            .await
            .context("Failed to fetch payments from RPC")?;

        if payments.is_empty() {
            info!("No new payments to ingest");
            return Ok(());
        }

        let last_paging_token = payments.last().map(|p| p.paging_token.clone());

        // Normalize payments
        let records: Vec<PaymentRecord> = payments
            .into_iter()
            .filter_map(|p| {
                let amount = p.amount.parse::<f64>().ok()?;
                let created_at = DateTime::parse_from_rfc3339(&p.created_at)
                    .ok()?
                    .with_timezone(&chrono::Utc);

                Some(PaymentRecord {
                    id: p.id,
                    transaction_hash: p.transaction_hash,
                    source_account: p.source_account,
                    destination_account: p.destination,
                    asset_type: p.asset_type.clone(),
                    asset_code: p.asset_code.clone(),
                    asset_issuer: p.asset_issuer.clone(),
                    source_asset_code: p.asset_code.clone().unwrap_or_default(),
                    source_asset_issuer: p.asset_issuer.clone().unwrap_or_default(),
                    destination_asset_code: p.asset_code.unwrap_or_default(),
                    destination_asset_issuer: p.asset_issuer.unwrap_or_default(),
                    amount,
                    successful: true,
                    timestamp: Some(created_at),
                    submission_time: None,
                    confirmation_time: None,
                    created_at,
                })
            })
            .collect();

        let count = records.len();

        // Persist idempotently
        self.db
            .save_payments(records)
            .await
            .context("Failed to save payments to database")?;

        // Update cursor
        if let Some(cursor) = last_paging_token {
            self.db
                .update_ingestion_cursor(task_name, &cursor)
                .await
                .context("Failed to update ingestion cursor")?;
            info!("Ingested {} payments. New cursor: {}", count, cursor);
        }

        Ok(())
    }
}
