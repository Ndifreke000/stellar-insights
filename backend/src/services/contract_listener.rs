//! Contract Event Listener Service
//!
//! This service listens to Soroban contract events in real-time,
//! indexes them, and provides verification capabilities for snapshot submissions.

use crate::database::Database;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Configuration for the contract event listener
#[derive(Clone, Debug)]
pub struct ListenerConfig {
    /// Soroban RPC endpoint URL
    pub rpc_url: String,
    /// Contract address (ID) on Stellar
    pub contract_id: String,
    /// Polling interval in seconds (default: 10)
    pub poll_interval_secs: u64,
    /// Start ledger number (optional, will use current if not specified)
    pub start_ledger: Option<u64>,
}

/// Snapshot event data from contract
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

/// Contract event from Soroban
#[derive(Debug, Deserialize)]
struct ContractEvent {
    #[serde(rename = "type")]
    event_type: String,
    ledger: String,
    #[serde(rename = "ledgerClosedAt")]
    ledger_closed_at: String,
    #[serde(rename = "contractId")]
    contract_id: String,
    id: String,
    #[serde(rename = "pagingToken")]
    paging_token: String,
    topic: Vec<String>,
    value: serde_json::Value,
}

/// RPC request structure for Soroban
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// RPC response structure
#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<T>,
    #[serde(default)]
    error: Option<RpcError>,
}

/// RPC error details
#[derive(Debug, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

/// Service for listening to Soroban contract events
pub struct ContractEventListener {
    client: Client,
    config: ListenerConfig,
    db: Arc<Database>,
    last_ledger: u64,
}

impl ContractEventListener {
    /// Create a new contract event listener
    pub fn new(config: ListenerConfig, db: Arc<Database>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        info!(
            "Initialized ContractEventListener for contract {} on RPC {}",
            config.contract_id, config.rpc_url
        );

        Ok(Self {
            client,
            config,
            db,
            last_ledger: config.start_ledger.unwrap_or(0),
        })
    }

    /// Start listening to contract events
    pub async fn start_listening(&mut self) -> Result<()> {
        info!("Starting contract event listener");

        // Get current ledger if not specified
        if self.last_ledger == 0 {
            self.last_ledger = self.get_latest_ledger().await?;
            info!("Starting from ledger {}", self.last_ledger);
        }

        let mut interval = interval(Duration::from_secs(self.config.poll_interval_secs));
        interval.tick().await; // Skip first immediate tick

        loop {
            interval.tick().await;

            match self.poll_for_events().await {
                Ok(events_processed) => {
                    if events_processed > 0 {
                        info!("Processed {} contract events", events_processed);
                    }
                }
                Err(e) => {
                    error!("Error polling for events: {}", e);
                    // Continue polling despite errors
                }
            }
        }
    }

    /// Poll for new events since last ledger
    async fn poll_for_events(&mut self) -> Result<usize> {
        let current_ledger = self.get_latest_ledger().await?;

        if current_ledger <= self.last_ledger {
            return Ok(0); // No new ledgers
        }

        debug!("Polling events from ledger {} to {}", self.last_ledger + 1, current_ledger);

        let events = self.get_events_for_ledger_range(self.last_ledger + 1, current_ledger).await?;

        let mut events_processed = 0;

        for event in events {
            match self.process_event(event).await {
                Ok(_) => events_processed += 1,
                Err(e) => {
                    error!("Failed to process event: {}", e);
                    // Continue processing other events
                }
            }
        }

        self.last_ledger = current_ledger;
        Ok(events_processed)
    }

    /// Get events for a specific ledger range
    async fn get_events_for_ledger_range(
        &self,
        start_ledger: u64,
        end_ledger: u64,
    ) -> Result<Vec<ContractEvent>> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getEvents".to_string(),
            params: json!({
                "startLedger": start_ledger.to_string(),
                "endLedger": end_ledger.to_string(),
                "filters": [
                    {
                        "type": "contract",
                        "contractIds": [self.config.contract_id]
                    }
                ]
            }),
        };

        let response = self
            .client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send getEvents request")?;

        let body: JsonRpcResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse getEvents response")?;

        if let Some(error) = body.error {
            return Err(anyhow::anyhow!(
                "getEvents failed: {} (code: {})",
                error.message,
                error.code
            ));
        }

        if let Some(result) = body.result {
            let events: Vec<ContractEvent> = serde_json::from_value(result)
                .context("Failed to deserialize events")?;
            Ok(events)
        } else {
            Ok(vec![])
        }
    }

    /// Process a single contract event
    pub async fn process_event(&self, event: ContractEvent) -> Result<()> {
        debug!("Processing contract event: {:?}", event);

        // Check if this is a snapshot submission event
        if event.topic.contains(&"SNAP_SUB".to_string()) {
            self.process_snapshot_event(event).await?;
        } else {
            debug!("Ignoring non-snapshot event: {:?}", event.topic);
        }

        Ok(())
    }

    /// Process a snapshot submission event
    async fn process_snapshot_event(&self, event: ContractEvent) -> Result<()> {
        let event_value = &event.value;

        // Extract event data
        let epoch = event_value
            .get("epoch")
            .and_then(|e| e.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing epoch in event"))?;

        let hash = event_value
            .get("hash")
            .and_then(|h| h.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing hash in event"))?
            .to_string();

        let timestamp = event_value
            .get("timestamp")
            .and_then(|t| t.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing timestamp in event"))?;

        let ledger = event.ledger
            .parse::<u64>()
            .context("Invalid ledger number")?;

        let snapshot_event = SnapshotEvent {
            epoch,
            hash,
            timestamp,
            ledger,
            transaction_hash: event.id.clone(),
            contract_id: event.contract_id.clone(),
            event_type: "SNAP_SUB".to_string(),
        };

        info!(
            "Received snapshot submission: epoch {}, hash {}, ledger {}",
            epoch, hash, ledger
        );

        // Store event in database
        self.store_snapshot_event(&snapshot_event).await?;

        // Verify against backend data
        self.verify_snapshot_with_backend(epoch, &hash).await?;

        Ok(())
    }

    /// Store snapshot event in database
    async fn store_snapshot_event(&self, event: &SnapshotEvent) -> Result<()> {
        let query = r#"
            INSERT OR REPLACE INTO contract_events (
                id, contract_id, event_type, epoch, hash, timestamp, 
                ledger, transaction_hash, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(&event.transaction_hash)
            .bind(&event.contract_id)
            .bind(&event.event_type)
            .bind(event.epoch as i64)
            .bind(&event.hash)
            .bind(event.timestamp as i64)
            .bind(event.ledger as i64)
            .bind(&event.transaction_hash)
            .bind(Utc::now())
            .execute(self.db.pool())
            .await
            .context("Failed to store contract event")?;

        debug!("Stored contract event: {}", event.transaction_hash);
        Ok(())
    }

    /// Verify snapshot hash against backend data
    async fn verify_snapshot_with_backend(&self, epoch: u64, on_chain_hash: &str) -> Result<bool> {
        debug!("Verifying snapshot epoch {} against backend data", epoch);

        // Get snapshot from database
        let query = r#"
            SELECT hash, canonical_json 
            FROM snapshots 
            WHERE epoch = ? 
            ORDER BY created_at DESC 
            LIMIT 1
        "#;

        let row = sqlx::query(query)
            .bind(epoch as i64)
            .fetch_optional(self.db.pool())
            .await
            .context("Failed to query snapshot from database")?;

        if let Some(row) = row {
            let backend_hash: String = row.get("hash");
            let canonical_json: String = row.get("canonical_json");

            debug!("Backend hash: {}, On-chain hash: {}", backend_hash, on_chain_hash);

            let is_verified = backend_hash == on_chain_hash;

            if is_verified {
                info!("✓ Snapshot verification passed for epoch {}", epoch);
            } else {
                error!("✗ Snapshot verification failed for epoch {} - hash mismatch", epoch);
                error!("Expected (backend): {}", backend_hash);
                error!("Actual (on-chain): {}", on_chain_hash);
                
                // Calculate hash to verify our data
                let calculated_hash = self.calculate_hash(&canonical_json)?;
                error!("Recalculated hash: {}", calculated_hash);
                
                // TODO: Send alert via AlertService
                // This would require passing AlertService to the listener
            }

            // Update verification status
            self.update_verification_status(epoch, is_verified).await?;

            Ok(is_verified)
        } else {
            warn!("No snapshot found in database for epoch {}", epoch);
            // TODO: Send missing snapshot alert
            Ok(false)
        }
    }

    /// Calculate SHA-256 hash of data
    fn calculate_hash(&self, data: &str) -> Result<String> {
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    /// Update verification status in database
    async fn update_verification_status(&self, epoch: u64, is_verified: bool) -> Result<()> {
        let query = r#"
            UPDATE snapshots 
            SET verification_status = ?, verified_at = ?
            WHERE epoch = ?
        "#;

        sqlx::query(query)
            .bind(if is_verified { "verified" } else { "failed" })
            .bind(Utc::now())
            .bind(epoch as i64)
            .execute(self.db.pool())
            .await
            .context("Failed to update verification status")?;

        Ok(())
    }

    /// Get the latest ledger number from the network
    async fn get_latest_ledger(&self) -> Result<u64> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getLatestLedger".to_string(),
            params: json!({}),
        };

        let response = self
            .client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to get latest ledger")?;

        let body: JsonRpcResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse latest ledger response")?;

        if let Some(error) = body.error {
            return Err(anyhow::anyhow!(
                "getLatestLedger failed: {} (code: {})",
                error.message,
                error.code
            ));
        }

        if let Some(result) = body.result {
            let ledger = result
                .get("sequence")
                .and_then(|s| s.as_u64())
                .ok_or_else(|| anyhow::anyhow!("Invalid ledger sequence"))?;
            Ok(ledger)
        } else {
            Err(anyhow::anyhow!("No ledger result returned"))
        }
    }

    /// Verify a specific snapshot epoch
    pub async fn verify_snapshot(&self, epoch: u64) -> Result<bool> {
        info!("Verifying snapshot for epoch {}", epoch);

        // Get on-chain hash from contract
        let on_chain_hash = self.get_snapshot_from_contract(epoch).await?;

        if let Some(hash) = on_chain_hash {
            self.verify_snapshot_with_backend(epoch, &hash).await
        } else {
            warn!("No snapshot found on-chain for epoch {}", epoch);
            Ok(false)
        }
    }

    /// Get snapshot hash from contract
    async fn get_snapshot_from_contract(&self, epoch: u64) -> Result<Option<String>> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "simulateTransaction".to_string(),
            params: json!({
                "transaction": {
                    "contractId": self.config.contract_id,
                    "function": "get_snapshot",
                    "args": [
                        {
                            "type": "u64",
                            "value": epoch.to_string()
                        }
                    ]
                }
            }),
        };

        let response = self
            .client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to get snapshot from contract")?;

        let body: JsonRpcResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse contract response")?;

        if let Some(error) = body.error {
            if error.message.contains("not found") {
                return Ok(None);
            }
            return Err(anyhow::anyhow!("Contract query failed: {}", error.message));
        }

        if let Some(result) = body.result {
            let hash = result
                .get("returnValue")
                .and_then(|rv| rv.as_str())
                .map(|s| s.to_string());
            Ok(hash)
        } else {
            Ok(None)
        }
    }

    /// Get recent events from database
    pub async fn get_recent_events(&self, limit: i64) -> Result<Vec<SnapshotEvent>> {
        let query = r#"
            SELECT contract_id, event_type, epoch, hash, timestamp, 
                   ledger, transaction_hash
            FROM contract_events
            ORDER BY created_at DESC
            LIMIT ?
        "#;

        let rows = sqlx::query(query)
            .bind(limit)
            .fetch_all(self.db.pool())
            .await
            .context("Failed to fetch recent events")?;

        let mut events = Vec::new();

        for row in rows {
            let event = SnapshotEvent {
                epoch: row.get::<i64, _>("epoch") as u64,
                hash: row.get("hash"),
                timestamp: row.get::<i64, _>("timestamp") as u64,
                ledger: row.get::<i64, _>("ledger") as u64,
                transaction_hash: row.get("transaction_hash"),
                contract_id: row.get("contract_id"),
                event_type: row.get("event_type"),
            };
            events.push(event);
        }

        Ok(events)
    }

    /// Create from environment variables
    pub fn from_env(db: Arc<Database>) -> Result<Self> {
        let config = ListenerConfig {
            rpc_url: std::env::var("SOROBAN_RPC_URL")
                .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string()),
            contract_id: std::env::var("SNAPSHOT_CONTRACT_ID")
                .context("SNAPSHOT_CONTRACT_ID environment variable not set")?,
            poll_interval_secs: std::env::var("CONTRACT_EVENT_POLL_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            start_ledger: std::env::var("CONTRACT_EVENT_START_LEDGER")
                .ok()
                .and_then(|s| s.parse().ok()),
        };

        Self::new(config, db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_calculate_hash() {
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let config = ListenerConfig {
            rpc_url: "https://test.com".to_string(),
            contract_id: "test-contract".to_string(),
            poll_interval_secs: 10,
            start_ledger: None,
        };

        let listener = ContractEventListener::new(config, db).unwrap();
        let data = r#"{"test": "data"}"#;
        let hash = listener.calculate_hash(data).unwrap();

        // Should be 64 characters (32 bytes × 2 hex chars)
        assert_eq!(hash.len(), 64);
        
        // Should only contain valid hex characters
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn test_contract_event_listener_from_env() {
        // Set environment variables for testing
        std::env::set_var("SNAPSHOT_CONTRACT_ID", "test-contract-id");
        std::env::set_var("CONTRACT_EVENT_POLL_INTERVAL", "15");
        std::env::set_var("CONTRACT_EVENT_START_LEDGER", "2000");

        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let listener = ContractEventListener::from_env(db).unwrap();
        
        assert_eq!(listener.config.contract_id, "test-contract-id");
        assert_eq!(listener.config.poll_interval_secs, 15);
        assert_eq!(listener.config.start_ledger, Some(2000));
        
        // Clean up
        std::env::remove_var("SNAPSHOT_CONTRACT_ID");
        std::env::remove_var("CONTRACT_EVENT_POLL_INTERVAL");
        std::env::remove_var("CONTRACT_EVENT_START_LEDGER");
    }
}
