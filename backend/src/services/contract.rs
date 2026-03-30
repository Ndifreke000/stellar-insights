//! Contract Service for submitting snapshots to Soroban smart contracts
//!
//! Temporarily disabled due to stellar_sdk 0.1 dependency issues.
//! This service will be fully Restored once a stable signing API is available.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResult {
    pub hash: String,
    pub transaction_hash: String,
    pub ledger: u64,
    pub timestamp: u64,
}

pub struct ContractService;

impl ContractService {
    pub fn new(_config: crate::services::contract_listener::ListenerConfig) -> Self {
        Self
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self)
    }

    pub async fn submit_snapshot(&self, _hash: [u8; 32], _ledger: u64) -> Result<SubmissionResult> {
        Err(anyhow::anyhow!("Contract service is temporarily disabled due to stellar_sdk 0.1 dependency issues"))
    }

    pub async fn submit_snapshot_hash(&self, _hash: [u8; 32], _ledger: u64) -> Result<SubmissionResult> {
        Err(anyhow::anyhow!("Contract service is temporarily disabled due to stellar_sdk 0.1 dependency issues"))
    }

    pub async fn health_check(&self) -> Result<bool> {
        Ok(false)
    }

    pub async fn verify_snapshot_exists(&self, _hash: &str, _ledger: u64) -> Result<bool> {
        Err(anyhow::anyhow!("Contract service is temporarily disabled"))
    }

    pub async fn get_snapshot_by_epoch(&self, _epoch: u64) -> Result<Option<String>> {
        Err(anyhow::anyhow!("Contract service is temporarily disabled"))
    }
}
