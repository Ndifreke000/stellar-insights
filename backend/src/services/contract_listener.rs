//! Contract Event Listener for Soroban snapshots
//!
//! Temporarily disabled due to compilation issues.
//! This service will be fully Restored once the workspace dependencies are stabilized.

use anyhow::Result;
use std::sync::Arc;
use crate::database::Database;
use crate::services::alert_service::AlertService;
use serde::{Deserialize, Serialize};

/// Configuration for the contract event listener
#[derive(Debug, Clone, Deserialize)]
pub struct ListenerConfig {
    pub rpc_url: String,
    pub contract_id: String,
    pub poll_interval_secs: u64,
    pub start_ledger: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEvent {
    pub epoch: u64,
    pub hash: String,
    pub timestamp: u64,
    pub ledger: u64,
    pub transaction_hash: String,
    pub contract_id: String,
    pub event_type: String,
}

pub struct ContractEventListener;

impl ContractEventListener {
    /// Create a new contract event listener (stub)
    pub fn new(
        _config: ListenerConfig,
        _db: Arc<Database>,
        _alert_service: Arc<AlertService>,
    ) -> Result<Self> {
        Ok(Self)
    }

    /// Create from environment variables (stub)
    pub fn from_env(_db: Arc<Database>, _alert_service: Arc<AlertService>) -> Result<Self> {
        Ok(Self)
    }

    /// Start listening to contract events (disabled)
    pub async fn start_listening(&mut self) -> Result<()> {
        info!("Contract listener is temporarily disabled");
        // Keep a perpetual loop to avoid service exit if expected to run forever
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    /// Verify a specific snapshot epoch (stub)
    pub async fn verify_snapshot(&self, _epoch: u64) -> Result<bool> {
        Ok(false)
    }

    /// Get recent events from database (stub)
    pub async fn get_recent_events(&self, _limit: i64) -> Result<Vec<SnapshotEvent>> {
        Ok(vec![])
    }
}

use tracing::info;
