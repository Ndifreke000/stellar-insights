use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{error, info, warn};

use crate::database::Database;
use crate::services::contract_listener::{ContractEventListener, ListenerConfig};
use crate::services::event_indexer::EventIndexer;

/// Configuration for contract event listener job
#[derive(Debug, Clone)]
pub struct ContractEventListenerConfig {
    /// Whether the job is enabled
    pub enabled: bool,
    /// Interval between job runs in seconds
    pub interval_seconds: u64,
    /// Contract ID to listen to
    pub contract_id: String,
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Start ledger number (optional)
    pub start_ledger: Option<u64>,
}

impl Default for ContractEventListenerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 10,
            contract_id: std::env::var("SNAPSHOT_CONTRACT_ID")
                .unwrap_or_else(|_| "default-contract-id".to_string()),
            rpc_url: std::env::var("SOROBAN_RPC_URL")
                .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string()),
            start_ledger: std::env::var("CONTRACT_EVENT_START_LEDGER")
                .ok()
                .and_then(|s| s.parse().ok()),
        }
    }
}

/// Contract event listener background job
pub struct ContractEventListenerJob {
    db: Arc<Database>,
    config: ContractEventListenerConfig,
}

impl ContractEventListenerJob {
    /// Create a new contract event listener job
    pub fn new(db: Arc<Database>, config: ContractEventListenerConfig) -> Self {
        Self { db, config }
    }

    /// Start the event listener job
    pub async fn start(self: Arc<Self>) {
        if !self.config.enabled {
            info!("Contract event listener job is disabled");
            return;
        }

        info!("Starting contract event listener job");
        info!("Contract ID: {}", self.config.contract_id);
        info!("RPC URL: {}", self.config.rpc_url);
        info!("Interval: {} seconds", self.config.interval_seconds);

        let mut interval = interval(TokioDuration::from_secs(self.config.interval_seconds));
        interval.tick().await; // Skip first immediate tick

        // Create services
        let event_indexer = Arc::new(EventIndexer::new(self.db.clone()));
        
        let listener_config = ListenerConfig {
            rpc_url: self.config.rpc_url.clone(),
            contract_id: self.config.contract_id.clone(),
            poll_interval_secs: self.config.interval_seconds,
            start_ledger: self.config.start_ledger,
        };

        // Note: In a real implementation, the ContractEventListener would run continuously
        // For this background job, we'll periodically check for missed events
        loop {
            interval.tick().await;

            match self.check_for_missed_events(&event_indexer).await {
                Ok(events_processed) => {
                    if events_processed > 0 {
                        info!("Processed {} missed contract events", events_processed);
                    }
                }
                Err(e) => {
                    error!("Error checking for missed events: {}", e);
                    // Continue running despite errors
                }
            }
        }
    }

    /// Check for missed events and process them
    async fn check_for_missed_events(&self, event_indexer: &Arc<EventIndexer>) -> Result<usize> {
        // Get the latest event from the database
        let recent_events = event_indexer.get_recent_events(1).await?;
        
        let start_ledger = if let Some(latest_event) = recent_events.first() {
            latest_event.ledger + 1
        } else {
            self.config.start_ledger.unwrap_or(0)
        };

        // In a real implementation, this would:
        // 1. Query the Stellar RPC for events since start_ledger
        // 2. Process each event through the event indexer
        // 3. Update verification status for snapshots
        
        // For now, we'll just log that we're checking
        debug!("Checking for events since ledger {}", start_ledger);
        
        // Return 0 events processed for now
        // In a real implementation, this would return the actual count
        Ok(0)
    }

    /// Get job statistics
    pub async fn get_stats(&self) -> Result<ContractEventListenerStats> {
        let event_indexer = Arc::new(EventIndexer::new(self.db.clone()));
        let event_stats = event_indexer.get_event_stats().await?;

        Ok(ContractEventListenerStats {
            enabled: self.config.enabled,
            interval_seconds: self.config.interval_seconds,
            contract_id: self.config.contract_id.clone(),
            rpc_url: self.config.rpc_url.clone(),
            total_events: event_stats.total_events,
            verified_snapshots: event_stats.verified_snapshots,
            failed_verifications: event_stats.failed_verifications,
            latest_epoch: event_stats.latest_epoch,
            latest_ledger: event_stats.latest_ledger,
            events_last_24h: event_stats.events_last_24h,
        })
    }
}

/// Statistics for the contract event listener job
#[derive(Debug, Clone)]
pub struct ContractEventListenerStats {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub contract_id: String,
    pub rpc_url: String,
    pub total_events: i64,
    pub verified_snapshots: i64,
    pub failed_verifications: i64,
    pub latest_epoch: Option<u64>,
    pub latest_ledger: Option<u64>,
    pub events_last_24h: i64,
}

/// Create and start the contract event listener job
pub async fn start_contract_event_listener_job(
    db: Arc<Database>,
) -> Result<Arc<ContractEventListenerJob>> {
    let config = ContractEventListenerConfig::default();
    let job = Arc::new(ContractEventListenerJob::new(db, config));
    
    let job_clone = job.clone();
    tokio::spawn(async move {
        job_clone.start().await;
    });

    info!("Contract event listener job started");
    Ok(job)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[tokio::test]
    async fn test_contract_event_listener_job_config() {
        let config = ContractEventListenerConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.interval_seconds, 10);
        assert!(!config.contract_id.is_empty());
        assert!(!config.rpc_url.is_empty());
    }

    #[tokio::test]
    async fn test_contract_event_listener_job_creation() {
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let config = ContractEventListenerConfig::default();
        
        let job = ContractEventListenerJob::new(db, config);
        
        assert_eq!(job.config.interval_seconds, 10);
        assert!(job.config.enabled);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let config = ContractEventListenerConfig::default();
        let job = ContractEventListenerJob::new(db, config);
        
        let stats = job.get_stats().await.unwrap();
        
        assert!(stats.enabled);
        assert_eq!(stats.interval_seconds, 10);
        assert_eq!(stats.total_events, 0); // No events in empty database
    }
}
