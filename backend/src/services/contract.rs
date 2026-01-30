//! Contract Service for submitting snapshots to Soroban smart contracts
//! 
//! This service handles:
//! - Connecting to Soroban RPC endpoints
//! - Submitting snapshot hashes on-chain
//! - Retry logic with exponential backoff
//! - Comprehensive error handling and logging

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tracing::{debug, error, info, warn};

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;
const BACKOFF_MULTIPLIER: u64 = 2;
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Configuration for the contract service
#[derive(Clone, Debug)]
pub struct ContractConfig {
    /// Soroban RPC endpoint URL
    pub rpc_url: String,
    /// Contract address (ID) on Stellar
    pub contract_id: String,
    /// Network passphrase (e.g., "Test SDF Network ; September 2015" for testnet)
    pub network_passphrase: String,
    /// Source account secret key for signing transactions
    pub source_secret_key: String,
}

/// Service for interacting with the Soroban snapshot contract
#[derive(Clone)]
pub struct ContractService {
    client: Client,
    config: ContractConfig,
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
#[allow(dead_code)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<T>,
    #[serde(default)]
    error: Option<RpcError>,
}

/// RPC error details
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct RpcError {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RPC Error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

/// Result of a successful snapshot submission
#[derive(Debug, Clone)]
pub struct SubmissionResult {
    /// Transaction hash
    pub transaction_hash: String,
    /// Epoch number
    pub epoch: u64,
    /// Ledger number where the transaction was included
    pub ledger: u64,
    /// Timestamp from the contract
    pub timestamp: u64,
}

impl ContractService {
    /// Create a new contract service instance
    pub fn new(config: ContractConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .context("Failed to create HTTP client")?;

        info!(
            "Initialized ContractService with RPC URL: {}, Contract ID: {}",
            config.rpc_url, config.contract_id
        );

        Ok(Self { client, config })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let config = ContractConfig {
            rpc_url: std::env::var("SOROBAN_RPC_URL")
                .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string()),
            contract_id: std::env::var("SNAPSHOT_CONTRACT_ID")
                .context("SNAPSHOT_CONTRACT_ID environment variable not set")?,
            network_passphrase: std::env::var("STELLAR_NETWORK_PASSPHRASE")
                .unwrap_or_else(|_| "Test SDF Network ; September 2015".to_string()),
            source_secret_key: std::env::var("STELLAR_SOURCE_SECRET_KEY")
                .context("STELLAR_SOURCE_SECRET_KEY environment variable not set")?,
        };

        Self::new(config)
    }

    /// Submit a snapshot hash to the on-chain contract
    /// 
    /// This function will:
    /// 1. Build and simulate the transaction
    /// 2. Sign the transaction
    /// 3. Submit to the network
    /// 4. Wait for confirmation
    /// 5. Retry on transient failures
    /// 
    /// # Arguments
    /// * `hash` - 32-byte snapshot hash
    /// * `epoch` - Epoch identifier
    /// 
    /// # Returns
    /// Result containing submission details or error
    pub async fn submit_snapshot(
        &self,
        hash: [u8; 32],
        epoch: u64,
    ) -> Result<SubmissionResult> {
        self.submit_snapshot_hash(hash, epoch).await
    }

    /// Submit a snapshot hash to the on-chain contract
    /// 
    /// This function will:
    /// 1. Build and simulate the transaction
    /// 2. Sign the transaction
    /// 3. Submit to the network
    /// 4. Wait for confirmation
    /// 5. Retry on transient failures
    /// 
    /// # Arguments
    /// * `hash` - 32-byte snapshot hash
    /// * `epoch` - Epoch identifier
    /// 
    /// # Returns
    /// Result containing submission details or error
    pub async fn submit_snapshot_hash(
        &self,
        hash: [u8; 32],
        epoch: u64,
    ) -> Result<SubmissionResult> {
        info!(
            "Submitting snapshot hash for epoch {}: {}",
            epoch,
            hex::encode(hash)
        );

        let mut attempt = 0;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        loop {
            attempt += 1;
            
            match self.try_submit_snapshot(hash, epoch).await {
                Ok(result) => {
                    info!(
                        "✓ Successfully submitted snapshot for epoch {} (tx: {}, ledger: {})",
                        epoch, result.transaction_hash, result.ledger
                    );
                    return Ok(result);
                }
                Err(e) => {
                    if attempt >= MAX_RETRIES {
                        error!(
                            "✗ Failed to submit snapshot for epoch {} after {} attempts: {}",
                            epoch, MAX_RETRIES, e
                        );
                        return Err(e).context(format!(
                            "Failed to submit snapshot after {} retries",
                            MAX_RETRIES
                        ));
                    }

                    warn!(
                        "Attempt {}/{} failed for epoch {}: {}. Retrying in {}ms...",
                        attempt, MAX_RETRIES, epoch, e, backoff_ms
                    );

                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms *= BACKOFF_MULTIPLIER;
                }
            }
        }
    }

    /// Single attempt to submit snapshot (without retry logic)
    async fn try_submit_snapshot(
        &self,
        hash: [u8; 32],
        epoch: u64,
    ) -> Result<SubmissionResult> {
        // Step 1: Build the contract invocation
        debug!("Building contract invocation for epoch {}", epoch);
        let invoke_args = self.build_invoke_args(hash, epoch)?;

        // Step 2: Simulate the transaction
        debug!("Simulating transaction");
        let simulated = self.simulate_transaction(&invoke_args).await?;

        // Step 3: Prepare and sign the transaction
        debug!("Preparing and signing transaction");
        let signed_xdr = self.prepare_and_sign_transaction(&simulated)?;

        // Step 4: Send the transaction
        debug!("Sending transaction to network");
        let tx_hash = self.send_transaction(&signed_xdr).await?;

        // Step 5: Wait for transaction confirmation
        debug!("Waiting for transaction confirmation: {}", tx_hash);
        let result = self.wait_for_transaction(&tx_hash, epoch).await?;

        Ok(result)
    }

    /// Build contract invocation arguments
    fn build_invoke_args(&self, hash: [u8; 32], epoch: u64) -> Result<serde_json::Value> {
        // Convert hash to hex for the contract call
        let hash_hex = hex::encode(hash);
        
        // Build Soroban contract invocation parameters
        // Format: invoke contract_id submit_snapshot [hash_bytes, epoch_u64]
        Ok(json!({
            "contractId": self.config.contract_id,
            "function": "submit_snapshot",
            "args": [
                {
                    "type": "bytes",
                    "value": hash_hex
                },
                {
                    "type": "u64",
                    "value": epoch.to_string()
                }
            ]
        }))
    }

    /// Simulate the transaction to get resource estimates
    async fn simulate_transaction(&self, invoke_args: &serde_json::Value) -> Result<serde_json::Value> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "simulateTransaction".to_string(),
            params: json!({
                "transaction": invoke_args
            }),
        };

        let response = self
            .client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send simulation request")?;

        let status = response.status();
        let body: JsonRpcResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse simulation response")?;

        if let Some(error) = body.error {
            return Err(anyhow::anyhow!(
                "Transaction simulation failed: {} (code: {})",
                error.message,
                error.code
            ));
        }

        body.result
            .ok_or_else(|| anyhow::anyhow!("No simulation result returned (status: {})", status))
    }

    /// Prepare and sign the transaction
    fn prepare_and_sign_transaction(&self, _simulated: &serde_json::Value) -> Result<String> {
        // In a real implementation, this would:
        // 1. Extract the transaction envelope from simulation
        // 2. Set appropriate fees and sequence number
        // 3. Sign with the source account's secret key
        // 4. Return the signed XDR
        
        // For now, return a placeholder that would need stellar-sdk integration
        // TODO: Integrate stellar-sdk for proper transaction signing
        
        warn!("Transaction signing not yet implemented - requires stellar-sdk integration");
        Err(anyhow::anyhow!(
            "Transaction signing requires stellar-sdk library integration"
        ))
    }

    /// Send the signed transaction to the network
    async fn send_transaction(&self, signed_xdr: &str) -> Result<String> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "sendTransaction".to_string(),
            params: json!({
                "transaction": signed_xdr
            }),
        };

        let response = self
            .client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send transaction")?;

        let body: JsonRpcResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse send transaction response")?;

        if let Some(error) = body.error {
            return Err(anyhow::anyhow!(
                "Transaction submission failed: {} (code: {})",
                error.message,
                error.code
            ));
        }

        let result = body
            .result
            .ok_or_else(|| anyhow::anyhow!("No transaction hash returned"))?;

        // Extract transaction hash from result
        let tx_hash = result
            .get("hash")
            .and_then(|h| h.as_str())
            .ok_or_else(|| anyhow::anyhow!("Transaction hash not found in response"))?
            .to_string();

        Ok(tx_hash)
    }

    /// Wait for transaction to be confirmed and return the result
    async fn wait_for_transaction(&self, tx_hash: &str, epoch: u64) -> Result<SubmissionResult> {
        let max_wait_attempts = 10;
        let poll_interval = Duration::from_secs(2);

        for attempt in 1..=max_wait_attempts {
            let request = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: 1,
                method: "getTransaction".to_string(),
                params: json!({
                    "hash": tx_hash
                }),
            };

            let response = self
                .client
                .post(&self.config.rpc_url)
                .json(&request)
                .send()
                .await
                .context("Failed to get transaction status")?;

            let body: JsonRpcResponse<serde_json::Value> = response
                .json()
                .await
                .context("Failed to parse transaction status response")?;

            if let Some(error) = body.error {
                // Transaction not found yet is expected while pending
                if error.code == -32602 || error.message.contains("not found") {
                    debug!("Transaction not confirmed yet (attempt {})", attempt);
                    tokio::time::sleep(poll_interval).await;
                    continue;
                }
                return Err(anyhow::anyhow!(
                    "Failed to get transaction status: {}",
                    error.message
                ));
            }

            if let Some(result) = body.result {
                let status = result
                    .get("status")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Transaction status not found"))?;

                match status {
                    "SUCCESS" => {
                        let ledger = result
                            .get("ledger")
                            .and_then(|l| l.as_u64())
                            .ok_or_else(|| anyhow::anyhow!("Ledger number not found"))?;

                        // Get timestamp from contract return value
                        let timestamp = result
                            .get("returnValue")
                            .and_then(|rv| rv.as_u64())
                            .unwrap_or(0);

                        return Ok(SubmissionResult {
                            transaction_hash: tx_hash.to_string(),
                            epoch,
                            ledger,
                            timestamp,
                        });
                    }
                    "FAILED" => {
                        let error_msg = result
                            .get("resultXdr")
                            .and_then(|x| x.as_str())
                            .unwrap_or("Unknown error");
                        return Err(anyhow::anyhow!("Transaction failed: {}", error_msg));
                    }
                    "PENDING" | "NOT_FOUND" => {
                        debug!("Transaction still pending (attempt {})", attempt);
                        tokio::time::sleep(poll_interval).await;
                        continue;
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Unknown transaction status: {}", status));
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Transaction confirmation timeout after {} attempts",
            max_wait_attempts
        ))
    }

    /// Health check for the RPC endpoint
    pub async fn health_check(&self) -> Result<bool> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getHealth".to_string(),
            params: json!({}),
        };

        let response = self
            .client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send health check request")?;

        let body: JsonRpcResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse health check response")?;

        Ok(body.result.is_some() && body.error.is_none())
    }

    /// Verify that a snapshot exists on-chain for the given hash and epoch
    pub async fn verify_snapshot_exists(&self, hash: &str, epoch: u64) -> Result<bool> {
        debug!("Verifying snapshot exists for epoch {} with hash {}", epoch, hash);

        // Convert hex hash back to bytes for contract call
        let hash_bytes = hex::decode(hash)
            .context("Invalid hash format")?;
        
        if hash_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Hash must be exactly 32 bytes"));
        }

        let mut hash_array = [0u8; 32];
        hash_array.copy_from_slice(&hash_bytes);

        // Call the contract's verify_snapshot function
        let verify_args = json!({
            "contractId": self.config.contract_id,
            "function": "verify_snapshot",
            "args": [
                {
                    "type": "bytes",
                    "value": hash
                }
            ]
        });

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "simulateTransaction".to_string(),
            params: json!({
                "transaction": verify_args
            }),
        };

        let response = self
            .client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send verification request")?;

        let body: JsonRpcResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse verification response")?;

        if let Some(error) = body.error {
            warn!("Verification request failed: {}", error.message);
            return Ok(false);
        }

        if let Some(result) = body.result {
            // Extract the return value from the simulation
            let return_value = result
                .get("returnValue")
                .and_then(|rv| rv.as_bool())
                .unwrap_or(false);

            debug!("Verification result for epoch {}: {}", epoch, return_value);
            Ok(return_value)
        } else {
            Ok(false)
        }
    }

    /// Get snapshot data for a specific epoch from the contract
    pub async fn get_snapshot_by_epoch(&self, epoch: u64) -> Result<Option<String>> {
        debug!("Getting snapshot for epoch {}", epoch);

        let get_args = json!({
            "contractId": self.config.contract_id,
            "function": "get_snapshot",
            "args": [
                {
                    "type": "u64",
                    "value": epoch.to_string()
                }
            ]
        });

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "simulateTransaction".to_string(),
            params: json!({
                "transaction": get_args
            }),
        };

        let response = self
            .client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send get snapshot request")?;

        let body: JsonRpcResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse get snapshot response")?;

        if let Some(error) = body.error {
            if error.message.contains("not found") {
                return Ok(None);
            }
            return Err(anyhow::anyhow!("Get snapshot failed: {}", error.message));
        }

        if let Some(result) = body.result {
            let hash_hex = result
                .get("returnValue")
                .and_then(|rv| rv.as_str())
                .map(|s| s.to_string());

            Ok(hash_hex)
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_invoke_args() {
        let config = ContractConfig {
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            contract_id: "CBGTG4JJFEQE3SPBGQFP3X5HM46N47LXZPXQACVKB7QA6X2XB2IG5CTA".to_string(),
            network_passphrase: "Test SDF Network ; September 2015".to_string(),
            source_secret_key: "S...".to_string(),
        };

        let service = ContractService::new(config).unwrap();
        let hash = [0u8; 32];
        let epoch = 123;

        let args = service.build_invoke_args(hash, epoch).unwrap();
        
        assert_eq!(args["contractId"], "CBGTG4JJFEQE3SPBGQFP3X5HM46N47LXZPXQACVKB7QA6X2XB2IG5CTA");
        assert_eq!(args["function"], "submit_snapshot");
        assert!(args["args"].is_array());
    }

    #[tokio::test]
    async fn test_health_check_with_mock() {
        // This would require a mock server setup
        // Placeholder for integration testing
    }
}
