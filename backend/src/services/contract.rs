use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Duration;
use stellar_xdr::curr::{
    DecoratedSignature, Limits, ReadXdr, Signature, SignatureHint, TransactionEnvelope,
    TransactionSignaturePayload, TransactionSignaturePayloadTaggedTransaction, WriteXdr,
};
use tracing::{debug, error, info, warn};

/// Minimal Stellar keypair sufficient for signing transaction hashes.
///
/// `stellar_sdk` 0.1 does not export usable `KeyPair`/`Network` types (see the
/// now-resolved FIXME below), so signing is implemented directly on top of
/// `stellar-strkey` (StrKey encode/decode) and `ed25519-dalek` (the actual
/// signature scheme Stellar accounts use), which are both already exact,
/// minimal, well-maintained building blocks for this.
struct StellarKeyPair {
    signing_key: SigningKey,
}

impl StellarKeyPair {
    /// Decodes a StrKey secret seed ("S...") into a signing key.
    fn from_secret_seed(seed: &str) -> Result<Self> {
        let raw = stellar_strkey::ed25519::PrivateKey::from_string(seed)
            .map_err(|e| anyhow::anyhow!("Invalid source secret key: {e}"))?;
        Ok(Self {
            signing_key: SigningKey::from_bytes(&raw.0),
        })
    }

    fn sign(&self, data: &[u8]) -> [u8; 64] {
        self.signing_key.sign(data).to_bytes()
    }

    /// The last 4 bytes of the public key, used as the `DecoratedSignature` hint.
    fn signature_hint(&self) -> SignatureHint {
        let public = self.signing_key.verifying_key().to_bytes();
        let mut hint = [0u8; 4];
        hint.copy_from_slice(&public[28..32]);
        SignatureHint(hint)
    }
}

/// Computes the Stellar `NETWORK_ID` for a given network passphrase, per the
/// Stellar protocol's transaction signature base definition
/// (`NETWORK_ID = SHA256(network_passphrase)`).
fn network_id(passphrase: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    hasher.finalize().into()
}

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
/// Note: All fields required for JSON deserialization from Stellar RPC
#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    #[allow(dead_code)] // Required for JSON deserialization
    jsonrpc: String,
    #[allow(dead_code)] // Required for JSON deserialization
    id: u64,
    #[serde(default)]
    result: Option<T>,
    #[serde(default)]
    error: Option<RpcError>,
}

/// RPC error details
/// Note: All fields required for JSON deserialization from Stellar RPC
#[derive(Debug, Deserialize, Clone)]
struct RpcError {
    #[allow(dead_code)] // Required for JSON deserialization
    code: i32,
    message: String,
    #[serde(default)]
    #[allow(dead_code)] // Required for JSON deserialization
    data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResult {
    pub hash: String,
    pub transaction_hash: String,
    pub ledger: u64,
    pub timestamp: u64,
}

impl ContractService {
    #[must_use]
    pub fn new(config: ContractConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .expect("Failed to build HTTP client");
        Self { client, config }
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

        Ok(Self::new(config))
    }

    pub async fn submit_snapshot(&self, hash: [u8; 32], epoch: u64) -> Result<SubmissionResult> {
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
                            "Failed to submit snapshot after {MAX_RETRIES} retries"
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
    async fn try_submit_snapshot(&self, hash: [u8; 32], epoch: u64) -> Result<SubmissionResult> {
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
    async fn simulate_transaction(
        &self,
        invoke_args: &serde_json::Value,
    ) -> Result<serde_json::Value> {
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
            .ok_or_else(|| anyhow::anyhow!("No simulation result returned (status: {status})"))
    }

    /// Prepare and sign the transaction using the Soroban RPC simulation result.
    ///
    /// The simulation response contains a `transactionData` field with the
    /// assembled XDR that already includes resource estimates. This decodes
    /// that envelope, computes the transaction signature base
    /// (`SHA256(NETWORK_ID ++ XDR(TransactionSignaturePayload))`), signs it
    /// with the configured source account's ed25519 key, attaches the
    /// resulting `DecoratedSignature`, and re-encodes the envelope.
    fn prepare_and_sign_transaction(&self, simulated: &serde_json::Value) -> Result<String> {
        let transaction_xdr = simulated
            .get("transactionData")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow::anyhow!("Simulation did not return transaction data"))?;

        if transaction_xdr.is_empty() {
            return Err(anyhow::anyhow!("Simulation returned empty transactionData"));
        }

        let keypair = StellarKeyPair::from_secret_seed(&self.config.source_secret_key)
            .context("Failed to load source signing key")?;

        // Decode the transaction envelope returned by simulation.
        let xdr_bytes = general_purpose::STANDARD
            .decode(transaction_xdr)
            .context("Failed to decode simulation XDR")?;

        let mut envelope = TransactionEnvelope::from_xdr(&xdr_bytes, Limits::none())
            .map_err(|e| anyhow::anyhow!("Failed to parse transaction XDR: {}", e))?;

        let tx = match &envelope {
            TransactionEnvelope::Tx(v1) => v1.tx.clone(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported transaction envelope version"
                ))
            }
        };

        // Build the transaction signature base per the Stellar protocol:
        // SHA256(NETWORK_ID ++ XDR(TransactionSignaturePayload)).
        let payload = TransactionSignaturePayload {
            network_id: stellar_xdr::curr::Hash(network_id(&self.config.network_passphrase)),
            tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(tx),
        };
        let payload_bytes = payload
            .to_xdr(Limits::none())
            .context("Failed to encode transaction signature payload")?;
        let mut hasher = Sha256::new();
        hasher.update(&payload_bytes);
        let tx_hash: [u8; 32] = hasher.finalize().into();

        let signature = keypair.sign(&tx_hash);

        let decorated_sig = DecoratedSignature {
            hint: keypair.signature_hint(),
            signature: Signature(
                signature
                    .to_vec()
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Unexpected signature length"))?,
            ),
        };

        if let TransactionEnvelope::Tx(ref mut v1) = envelope {
            v1.signatures
                .try_push(decorated_sig)
                .map_err(|e| anyhow::anyhow!("Failed to attach signature: {e}"))?;
        }

        let signed_xdr = envelope
            .to_xdr_base64(Limits::none())
            .context("Failed to re-encode signed transaction XDR")?;

        debug!(
            "Signed transaction XDR ({} chars)",
            signed_xdr.len()
        );

        Ok(signed_xdr)
    }

    async fn send_transaction(&self, signed_xdr: &str) -> Result<String> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "sendTransaction".to_string(),
            params: json!({ "transaction": signed_xdr }),
        };

        let response = self
            .client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send sendTransaction RPC request")?;

        let body: JsonRpcResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse sendTransaction RPC response")?;

        if let Some(error) = body.error {
            return Err(anyhow::anyhow!(
                "sendTransaction failed: {} (code: {})",
                error.message,
                error.code
            ));
        }

        let result = body
            .result
            .ok_or_else(|| anyhow::anyhow!("sendTransaction returned empty result"))?;

        result
            .get("hash")
            .or_else(|| result.get("transactionHash"))
            .and_then(|h| h.as_str())
            .map(std::string::ToString::to_string)
            .context("sendTransaction result missing transaction hash")
    }

    async fn wait_for_transaction(&self, tx_hash: &str, epoch: u64) -> Result<SubmissionResult> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getTransaction".to_string(),
            params: json!({ "hash": tx_hash }),
        };

        for _ in 0..60 {
            let response = self
                .client
                .post(&self.config.rpc_url)
                .json(&request)
                .send()
                .await
                .context("Failed to send getTransaction RPC request")?;

            let body: JsonRpcResponse<serde_json::Value> = response
                .json()
                .await
                .context("Failed to parse getTransaction RPC response")?;

            if let Some(error) = &body.error {
                let transient = error.message.to_ascii_lowercase().contains("not found");
                if !transient {
                    return Err(anyhow::anyhow!(
                        "getTransaction failed: {} (code: {})",
                        error.message,
                        error.code
                    ));
                }
            } else if let Some(result) = body.result {
                let status = result.get("status").and_then(|s| s.as_str()).unwrap_or("");
                if status.eq_ignore_ascii_case("success") || status.eq_ignore_ascii_case("failed") {
                    let ledger = result
                        .get("ledger")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let timestamp = result
                        .get("createdAt")
                        .and_then(|s| s.as_str())
                        .and_then(|s| {
                            chrono::DateTime::parse_from_rfc3339(s)
                                .ok()
                                .map(|d| d.timestamp() as u64)
                        })
                        .unwrap_or(0);

                    return Ok(SubmissionResult {
                        hash: tx_hash.to_string(),
                        transaction_hash: tx_hash.to_string(),
                        ledger,
                        timestamp,
                    });
                }
            }

            tokio::time::sleep(Duration::from_millis(250)).await;
        }

        Err(anyhow::anyhow!(
            "Timed out waiting for transaction {tx_hash} (epoch {epoch})"
        ))
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
