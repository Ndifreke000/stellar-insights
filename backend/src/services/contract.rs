use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, Instant};
use stellar_xdr::curr::{
    ContractDataDurability, ContractId, DecoratedSignature, Hash, LedgerEntryData, LedgerKey,
    LedgerKeyContractData, Limits, ReadXdr, ScAddress, ScSymbol, ScVal, ScVec, Signature,
    SignatureHint, TransactionEnvelope, TransactionSignaturePayload,
    TransactionSignaturePayloadTaggedTransaction, WriteXdr,
};

use crate::services::envelope_policy;
use crate::services::signature_audit::{SignatureAuditor, SignatureStatus};
use crate::services::transaction_signer::{
    self, signature_hint_from_public_key, verify_ed25519_signature, LocalSeedSigner,
    TransactionSigner,
};
use crate::snapshot::verify::OnChainRecord;
use tracing::{debug, error, info, warn};

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
    /// Transaction signing backend. `None` on read-only clients; submitting
    /// without one fails with an explicit error.
    signer: Option<Arc<dyn TransactionSigner>>,
    /// Audit trail for every signature attempt (tracing + optional DB).
    auditor: SignatureAuditor,
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

        // Backwards-compatible construction: a non-empty secret seed becomes
        // a local signer. `from_env` is the production entry point and picks
        // the backend via TX_SIGNER_BACKEND instead.
        let signer = if config.source_secret_key.trim().is_empty() {
            None
        } else {
            LocalSeedSigner::from_secret_seed(&config.source_secret_key)
                .ok()
                .map(|s| Arc::new(s) as Arc<dyn TransactionSigner>)
        };

        Self {
            client,
            config,
            signer,
            auditor: SignatureAuditor::default(),
        }
    }

    /// Attach a specific signer (tests, or callers that built one themselves).
    #[must_use]
    pub fn with_signer(mut self, signer: Arc<dyn TransactionSigner>) -> Self {
        self.signer = Some(signer);
        self
    }

    /// Persist signature audit records to the given database pool, in
    /// addition to the always-on tracing events.
    #[must_use]
    pub fn with_audit_pool(mut self, pool: sqlx::SqlitePool) -> Self {
        self.auditor = SignatureAuditor::new(Some(pool));
        self
    }

    /// Create from environment variables.
    ///
    /// The signing backend is selected by `TX_SIGNER_BACKEND`
    /// (`local` default, `gcp-kms`, `aws-kms`); see
    /// `services::transaction_signer` for the full variable list. With a KMS
    /// backend no secret seed is loaded into process memory.
    pub async fn from_env() -> Result<Self> {
        let config = ContractConfig {
            rpc_url: std::env::var("SOROBAN_RPC_URL")
                .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string()),
            contract_id: std::env::var("SNAPSHOT_CONTRACT_ID")
                .context("SNAPSHOT_CONTRACT_ID environment variable not set")?,
            network_passphrase: std::env::var("STELLAR_NETWORK_PASSPHRASE")
                .unwrap_or_else(|_| "Test SDF Network ; September 2015".to_string()),
            // Only used by the local signer backend; KMS backends ignore it.
            source_secret_key: std::env::var("STELLAR_SOURCE_SECRET_KEY").unwrap_or_default(),
        };

        let signer = transaction_signer::signer_from_env().await?;
        Ok(Self::new(config).with_signer(signer))
    }

    /// Create a read-only client from environment variables.
    ///
    /// Reads go through `getLedgerEntries`, which needs no source account and
    /// no signer, so only `SNAPSHOT_CONTRACT_ID` is required. Calling the
    /// submit path on a read-only client fails with an explicit "no signer"
    /// error.
    pub fn read_only_from_env() -> Result<Self> {
        let config = ContractConfig {
            rpc_url: std::env::var("SOROBAN_RPC_URL")
                .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string()),
            contract_id: std::env::var("SNAPSHOT_CONTRACT_ID")
                .context("SNAPSHOT_CONTRACT_ID environment variable not set")?,
            network_passphrase: std::env::var("STELLAR_NETWORK_PASSPHRASE")
                .unwrap_or_else(|_| "Test SDF Network ; September 2015".to_string()),
            source_secret_key: String::new(),
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
        let signed_xdr = self.prepare_and_sign_transaction(&simulated, hash, epoch)?;

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
    /// assembled XDR that already includes resource estimates. Before any
    /// signature is produced, the decoded envelope is checked against the
    /// envelope policy (`services::envelope_policy`): it must be exactly the
    /// `submit_snapshot(hash, epoch)` call on the configured contract.
    /// Only then is the transaction signature base
    /// (`SHA256(NETWORK_ID ++ XDR(TransactionSignaturePayload))`) handed to
    /// the configured [`TransactionSigner`], which never sees anything but
    /// the 32-byte hash.
    async fn prepare_and_sign_transaction(
        &self,
        simulated: &serde_json::Value,
        hash: [u8; 32],
        epoch: u64,
    ) -> Result<String> {
        let transaction_xdr = simulated
            .get("transactionData")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow::anyhow!("Simulation did not return transaction data"))?;

        if transaction_xdr.is_empty() {
            return Err(anyhow::anyhow!("Simulation returned empty transactionData"));
        }

        // Decode the transaction envelope returned by simulation.
        let xdr_bytes = general_purpose::STANDARD
            .decode(transaction_xdr)
            .context("Failed to decode simulation XDR")?;

        let mut envelope = TransactionEnvelope::from_xdr(&xdr_bytes, Limits::none())
            .map_err(|e| anyhow::anyhow!("Failed to parse transaction XDR: {}", e))?;

        let tx = match &envelope {
            TransactionEnvelope::Tx(v1) => v1.tx.clone(),
            _ => return Err(anyhow::anyhow!("Unsupported transaction envelope version")),
        };

        // Envelope policy gate: refuse to sign anything that is not exactly
        // the submit_snapshot(hash, epoch) call we asked the RPC to simulate.
        if let Err(e) =
            envelope_policy::validate_submit_snapshot_envelope(&tx, &self.config.contract_id, &hash, epoch)
        {
            self.audit(SignatureStatus::PolicyRejected, "", epoch, None, Some(e.to_string()))
                .await;
            return Err(e.context("Envelope policy rejected the simulated transaction"));
        }

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
        let tx_hash_hex = hex::encode(tx_hash);

        let signer = self.signer.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "No transaction signer configured; cannot submit snapshots \
                 (read-only client or missing TX_SIGNER_BACKEND credentials)"
            )
        })?;

        // The signer only ever sees the 32-byte signature base.
        let started = Instant::now();
        let signature = match signer.sign(&tx_hash).await {
            Ok(sig) => sig,
            Err(e) => {
                self.audit(
                    SignatureStatus::SignError,
                    &tx_hash_hex,
                    epoch,
                    None,
                    Some(format!("{}: {e}", signer.backend())),
                )
                .await;
                return Err(e.context("Transaction signing failed"));
            }
        };
        let latency_ms = started.elapsed().as_millis() as u64;

        if signature.len() != 64 {
            self.audit(
                SignatureStatus::SignError,
                &tx_hash_hex,
                epoch,
                None,
                Some(format!("{}: bad signature length {}", signer.backend(), signature.len())),
            )
            .await;
            return Err(anyhow::anyhow!(
                "Signer returned {} bytes; expected a 64-byte Ed25519 signature",
                signature.len()
            ));
        }

        // Defense in depth: verify the signature against the signer's public
        // key before attaching it, so a misbehaving KMS cannot produce a
        // transaction that will fail on-chain.
        let public_key = signer
            .public_key()
            .await
            .context("Failed to fetch signer public key")?;
        if let Err(e) = verify_ed25519_signature(&public_key, &tx_hash, &signature) {
            self.audit(
                SignatureStatus::SignError,
                &tx_hash_hex,
                epoch,
                None,
                Some(format!("{}: signature self-check failed: {e}", signer.backend())),
            )
            .await;
            return Err(e.context("Signer produced an invalid signature"));
        }

        let hint = signature_hint_from_public_key(&public_key);
        let decorated_sig = DecoratedSignature {
            hint: SignatureHint(hint),
            signature: Signature(
                signature
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Unexpected signature length"))?,
            ),
        };

        if let TransactionEnvelope::Tx(ref mut v1) = envelope {
            // `VecM` exposes no push of its own; the bounded conversion back
            // from `Vec` is what enforces the 20-signature limit.
            let mut signatures = v1.signatures.to_vec();
            signatures.push(decorated_sig);
            v1.signatures = signatures
                .try_into()
                .map_err(|e| anyhow::anyhow!("Failed to attach signature: {e}"))?;
        }

        self.audit(
            SignatureStatus::Signed,
            &tx_hash_hex,
            epoch,
            Some(latency_ms),
            None,
        )
        .await;

        let signed_xdr = envelope
            .to_xdr_base64(Limits::none())
            .context("Failed to re-encode signed transaction XDR")?;

        debug!("Signed transaction XDR ({} chars)", signed_xdr.len());

        Ok(signed_xdr)
    }

    /// Emit a signature audit event for the current signer.
    async fn audit(
        &self,
        status: SignatureStatus,
        tx_hash: &str,
        epoch: u64,
        latency_ms: Option<u64>,
        error: Option<String>,
    ) {
        let (backend, key_id) = self
            .signer
            .as_ref()
            .map(|s| (s.backend().to_string(), s.key_id().to_string()))
            .unwrap_or_else(|| ("none".to_string(), "none".to_string()));
        self.auditor
            .record(&crate::services::signature_audit::SignatureAuditEvent {
                signer_backend: backend,
                signer_key_id: key_id,
                tx_hash: tx_hash.to_string(),
                contract_id: self.config.contract_id.clone(),
                function_name: envelope_policy::SUBMIT_SNAPSHOT_FUNCTION.to_string(),
                epoch: Some(epoch),
                status,
                error,
                latency_ms: latency_ms.unwrap_or(0),
            })
            .await;
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

    /// Check that `hash` is the hash anchored on-chain for `epoch`.
    pub async fn verify_snapshot_exists(&self, hash: &str, epoch: u64) -> Result<bool> {
        Ok(self
            .get_snapshot_record(epoch)
            .await?
            .is_some_and(|record| {
                record
                    .hash
                    .eq_ignore_ascii_case(hash.trim_start_matches("0x"))
            }))
    }

    /// Hex hash anchored on-chain for `epoch`, if any.
    pub async fn get_snapshot_by_epoch(&self, epoch: u64) -> Result<Option<String>> {
        Ok(self
            .get_snapshot_record(epoch)
            .await?
            .map(|record| record.hash))
    }

    /// Read the snapshot record for `epoch` straight from contract storage.
    ///
    /// The `payraider` contract keeps every snapshot in one persistent entry,
    /// `DataKey::Snapshots -> Map<u64, Snapshot>`, so this fetches that entry
    /// with `getLedgerEntries` and picks out the epoch. Unlike simulating
    /// `get_snapshot`, this needs no source account and no fee.
    pub async fn get_snapshot_record(&self, epoch: u64) -> Result<Option<OnChainRecord>> {
        let key = snapshots_ledger_key(&self.config.contract_id)?;
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getLedgerEntries".to_string(),
            params: json!({ "keys": [key] }),
        };

        let body: JsonRpcResponse<serde_json::Value> = self
            .client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send getLedgerEntries RPC request")?
            .json()
            .await
            .context("Failed to parse getLedgerEntries RPC response")?;

        if let Some(error) = body.error {
            return Err(anyhow::anyhow!(
                "getLedgerEntries failed: {} (code: {})",
                error.message,
                error.code
            ));
        }

        let Some(entry) = body
            .result
            .as_ref()
            .and_then(|r| r.get("entries"))
            .and_then(|e| e.as_array())
            .and_then(|e| e.first())
        else {
            // No entry at all: nothing has ever been submitted (or it was archived).
            return Ok(None);
        };

        let entry_xdr = entry
            .get("xdr")
            .and_then(|x| x.as_str())
            .context("getLedgerEntries entry missing xdr")?;
        let data = LedgerEntryData::from_xdr_base64(entry_xdr, Limits::none())
            .map_err(|e| anyhow::anyhow!("Failed to decode ledger entry XDR: {e}"))?;
        let LedgerEntryData::ContractData(contract_data) = data else {
            return Err(anyhow::anyhow!("Ledger entry is not contract data"));
        };

        let as_u32 = |field: &str| {
            entry
                .get(field)
                .and_then(serde_json::Value::as_u64)
                .and_then(|v| u32::try_from(v).ok())
        };

        Ok(
            find_snapshot_in_map(&contract_data.val, epoch)?.map(|(hash, timestamp)| {
                OnChainRecord {
                    contract_id: self.config.contract_id.clone(),
                    epoch,
                    hash: hex::encode(hash),
                    timestamp,
                    last_modified_ledger: as_u32("lastModifiedLedgerSeq"),
                    live_until_ledger: as_u32("liveUntilLedgerSeq"),
                }
            }),
        )
    }
}

/// Base64 `LedgerKey` for the contract's `DataKey::Snapshots` persistent entry.
///
/// A `#[contracttype]` unit enum variant is encoded as `ScVec[Symbol(name)]`.
fn snapshots_ledger_key(contract_id: &str) -> Result<String> {
    let contract = stellar_strkey::Contract::from_string(contract_id)
        .map_err(|e| anyhow::anyhow!("Invalid SNAPSHOT_CONTRACT_ID: {e}"))?;
    let symbol = ScSymbol(
        "Snapshots"
            .try_into()
            .map_err(|e| anyhow::anyhow!("Invalid symbol: {e}"))?,
    );
    let key_vec = ScVec(
        vec![ScVal::Symbol(symbol)]
            .try_into()
            .map_err(|e| anyhow::anyhow!("Invalid key vec: {e}"))?,
    );

    LedgerKey::ContractData(LedgerKeyContractData {
        contract: ScAddress::Contract(ContractId(Hash(contract.0))),
        key: ScVal::Vec(Some(key_vec)),
        durability: ContractDataDurability::Persistent,
    })
    .to_xdr_base64(Limits::none())
    .context("Failed to encode ledger key")
}

/// Extract `(hash, timestamp)` for `epoch` from the stored `Map<u64, Snapshot>`.
///
/// `Snapshot` is a `#[contracttype]` struct, so it arrives as an `ScMap` keyed
/// by field-name symbols (`epoch`, `hash`, `timestamp`).
fn find_snapshot_in_map(val: &ScVal, epoch: u64) -> Result<Option<([u8; 32], u64)>> {
    let ScVal::Map(Some(snapshots)) = val else {
        return Err(anyhow::anyhow!("Snapshots entry is not a map"));
    };

    let Some(entry) = snapshots
        .0
        .iter()
        .find(|e| matches!(e.key, ScVal::U64(k) if k == epoch))
    else {
        return Ok(None);
    };

    let ScVal::Map(Some(fields)) = &entry.val else {
        return Err(anyhow::anyhow!(
            "Snapshot for epoch {epoch} is not a struct"
        ));
    };

    let field = |name: &str| {
        fields.0.iter().find_map(|f| match &f.key {
            ScVal::Symbol(sym) if sym.0.to_utf8_string_lossy() == name => Some(&f.val),
            _ => None,
        })
    };

    let hash = match field("hash") {
        Some(ScVal::Bytes(bytes)) => <[u8; 32]>::try_from(bytes.0.as_vec().as_slice())
            .map_err(|_| anyhow::anyhow!("Snapshot hash is not 32 bytes"))?,
        _ => {
            return Err(anyhow::anyhow!(
                "Snapshot for epoch {epoch} has no hash field"
            ))
        }
    };
    let timestamp = match field("timestamp") {
        Some(ScVal::U64(ts)) => *ts,
        _ => 0,
    };

    Ok(Some((hash, timestamp)))
}
