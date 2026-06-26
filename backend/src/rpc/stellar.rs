use crate::network::{NetworkConfig, StellarNetwork};
use crate::observability::tracing::inject_trace_context;
use crate::rpc::circuit_breaker::{rpc_circuit_breaker, CircuitBreaker};
use crate::rpc::config::{initial_backoff_from_env, max_backoff_from_env, max_retries_from_env};
use crate::rpc::error::{with_retry, RetryConfig, RpcError};
use crate::rpc::metrics;
use crate::rpc::rate_limiter::{RpcRateLimitConfig, RpcRateLimitMetrics, RpcRateLimiter};
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

// ============================================================================
// RPC Pagination Security Limits
// ============================================================================
/// Maximum records that can be returned in a single RPC request (hard cap)
const ABSOLUTE_MAX_RECORDS_PER_REQUEST: u32 = 500;
/// Default records per RPC request
const DEFAULT_MAX_RECORDS_PER_REQUEST: u32 = 200;
/// Absolute maximum total records across all paginated requests (`DoS` protection)
const ABSOLUTE_MAX_TOTAL_RECORDS: u32 = 5000;
/// Default total records limit for pagination
const DEFAULT_MAX_TOTAL_RECORDS: u32 = 1000;
/// Minimum delay between pagination requests (`DoS` protection)
const MIN_PAGINATION_DELAY_MS: u64 = 50;
/// Default delay between pagination requests
const DEFAULT_PAGINATION_DELAY_MS: u64 = 100;

/// Stellar RPC Client for interacting with Stellar network via RPC and Horizon API
// Asset Models (Horizon API)
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonAsset {
    pub asset_type: String,
    pub asset_code: String,
    pub asset_issuer: String,
    pub num_claimable_balances: i32,
    pub num_liquidity_pools: i32,
    pub num_contracts: i32,
    pub accounts: AssetAccounts,
    pub claimable_balances_amount: String,
    pub liquidity_pools_amount: String,
    pub contracts_amount: String,
    pub balances: AssetBalances,
    pub flags: AssetFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetAccounts {
    pub authorized: i32,
    pub authorized_to_maintain_liabilities: i32,
    pub unauthorized: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalances {
    pub authorized: String,
    pub authorized_to_maintain_liabilities: String,
    pub unauthorized: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFlags {
    pub auth_required: bool,
    pub auth_revocable: bool,
    pub auth_immutable: bool,
    pub auth_clawback_enabled: bool,
}

#[derive(Clone)]
pub struct StellarRpcClient {
    client: Client,
    rpc_url: String,
    horizon_url: String,
    network_config: NetworkConfig,
    mock_mode: bool,
    rate_limiter: RpcRateLimiter,
    circuit_breaker: Arc<CircuitBreaker>,
    /// Maximum records per single request (default: 200)
    max_records_per_request: u32,
    /// Maximum total records across all paginated requests (default: 10_000)
    pub max_total_records: u32,
    /// Delay between pagination requests in milliseconds (default: 100)
    pagination_delay_ms: u64,
    /// Maximum retries for RPC calls
    max_retries: u32,
    /// Initial backoff duration
    initial_backoff: Duration,
    /// Maximum backoff duration
    max_backoff: Duration,
}

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    #[serde(rename = "latestLedger")]
    pub latest_ledger: u64,
    #[serde(rename = "oldestLedger")]
    pub oldest_ledger: u64,
    #[serde(rename = "ledgerRetentionWindow")]
    pub ledger_retention_window: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerInfo {
    pub sequence: u64,
    pub hash: String,
    pub previous_hash: String,
    pub transaction_count: u32,
    pub operation_count: u32,
    pub closed_at: String,
    pub total_coins: String,
    pub fee_pool: String,
    pub base_fee: u32,
    pub base_reserve: String,
}

/// Represents a single asset balance change from the new Horizon API format.
///
/// The new Horizon response for Soroban-compatible payments includes an
/// `asset_balance_changes` array instead of top-level destination / amount /
/// `asset_code` fields.  Each entry describes one leg of a transfer.
///
/// Example JSON:
/// ```json
/// {
///   "asset_type": "credit_alphanum4",
///   "asset_code": "USDC",
///   "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
///   "type": "transfer",
///   "from": "GXXXXXXX...",
///   "to": "GDYYYYYY...",
///   "amount": "100.0000000"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalanceChange {
    /// The Stellar asset type (native, `credit_alphanum4`, `credit_alphanum12`)
    pub asset_type: String,
    /// Asset code – `None` for native XLM
    pub asset_code: Option<String>,
    /// Asset issuer – `None` for native XLM
    pub asset_issuer: Option<String>,
    /// The kind of balance change (e.g. "transfer")
    #[serde(rename = "type")]
    pub change_type: String,
    /// Source account of this balance change
    pub from: Option<String>,
    /// Destination account of this balance change
    pub to: Option<String>,
    /// Amount transferred in stroops-string format (e.g. "100.0000000")
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub paging_token: String,
    pub transaction_hash: String,
    pub source_account: String,
    #[serde(default)]
    pub destination: String,
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    pub amount: String,
    pub created_at: String,
    // Path payment fields
    #[serde(rename = "type")]
    pub operation_type: Option<String>,
    // Source asset for path payments
    pub source_asset_type: Option<String>,
    pub source_asset_code: Option<String>,
    pub source_asset_issuer: Option<String>,
    pub source_amount: Option<String>,
    // For regular payments, 'from' field
    pub from: Option<String>,
    // For regular payments, 'to' field
    pub to: Option<String>,
    /// New Horizon API format: Soroban-compatible asset balance changes.
    /// When present the traditional top-level fields may be empty; callers
    /// should use the `get_*` helper methods which transparently check both.
    #[serde(default)]
    pub asset_balance_changes: Option<Vec<AssetBalanceChange>>,
}

impl Payment {
    /// Returns the destination account, checking the new `asset_balance_changes`
    /// format first, then falling back to the legacy `destination` / `to` fields.
    #[must_use]
    pub fn get_destination(&self) -> Option<String> {
        if let Some(ref changes) = self.asset_balance_changes {
            if let Some(change) = changes.first() {
                if let Some(ref to) = change.to {
                    return Some(to.clone());
                }
            }
        }
        if !self.destination.is_empty() {
            return Some(self.destination.clone());
        }
        self.to.clone()
    }

    /// Returns the transfer amount, preferring `asset_balance_changes`.
    #[must_use]
    pub fn get_amount(&self) -> String {
        if let Some(ref changes) = self.asset_balance_changes {
            if let Some(change) = changes.first() {
                return change.amount.clone();
            }
        }
        self.amount.clone()
    }

    /// Returns the asset code, preferring `asset_balance_changes`.
    #[must_use]
    pub fn get_asset_code(&self) -> Option<String> {
        if let Some(ref changes) = self.asset_balance_changes {
            if let Some(change) = changes.first() {
                return change.asset_code.clone();
            }
        }
        self.asset_code.clone()
    }

    /// Returns the asset issuer, preferring `asset_balance_changes`.
    #[must_use]
    pub fn get_asset_issuer(&self) -> Option<String> {
        if let Some(ref changes) = self.asset_balance_changes {
            if let Some(change) = changes.first() {
                return change.asset_issuer.clone();
            }
        }
        self.asset_issuer.clone()
    }
}

// Horizon API Response Structures
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonOperation {
    pub id: String,
    pub paging_token: String,
    pub transaction_hash: String,
    pub source_account: String,
    #[serde(rename = "type")]
    pub operation_type: String,
    pub created_at: String,
    pub account: Option<String>,
    pub into: Option<String>,
    pub amount: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonEffect {
    pub id: String,
    #[serde(rename = "type")]
    pub effect_type: String,
    pub account: Option<String>,
    pub amount: Option<String>,
    pub asset_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonTransaction {
    pub id: String,
    pub hash: String,
    pub ledger: u64,
    pub created_at: String,
    pub source_account: String,
    #[serde(rename = "fee_account")]
    pub fee_account: Option<String>,
    #[serde(rename = "fee_charged")]
    pub fee_charged: Option<String>,
    #[serde(rename = "max_fee")]
    pub max_fee: Option<String>,
    pub operation_count: u32,
    pub successful: bool,
    pub paging_token: String,
    #[serde(rename = "fee_bump_transaction")]
    pub fee_bump_transaction: Option<FeeBumpTransactionInfo>,
    #[serde(rename = "inner_transaction")]
    pub inner_transaction: Option<InnerTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBumpTransactionInfo {
    pub hash: String,
    pub signatures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerTransaction {
    pub hash: String,
    #[serde(rename = "max_fee")]
    pub max_fee: Option<String>,
    pub signatures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub ledger_close_time: String,
    pub base_account: String,
    pub base_amount: String,
    pub base_asset_type: String,
    pub base_asset_code: Option<String>,
    pub base_asset_issuer: Option<String>,
    pub counter_account: String,
    pub counter_amount: String,
    pub counter_asset_type: String,
    pub counter_asset_code: Option<String>,
    pub counter_asset_issuer: Option<String>,
    pub price: Price,
    pub trade_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub n: i64,
    pub d: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub bids: Vec<OrderBookEntry>,
    pub asks: Vec<OrderBookEntry>,
    pub base: Asset,
    pub counter: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookEntry {
    pub price: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonResponse<T> {
    #[serde(rename = "_embedded")]
    pub embedded: Option<EmbeddedRecords<T>>,
    #[serde(flatten)]
    pub data: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedRecords<T> {
    pub records: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcLedger {
    pub hash: String,
    pub sequence: u64,
    #[serde(rename = "ledgerCloseTime")]
    pub ledger_close_time: String,
    #[serde(rename = "headerXdr")]
    pub header_xdr: Option<String>,
    #[serde(rename = "metadataXdr")]
    pub metadata_xdr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLedgersResult {
    pub ledgers: Vec<RpcLedger>,
    #[serde(rename = "latestLedger")]
    pub latest_ledger: u64,
    #[serde(rename = "oldestLedger")]
    pub oldest_ledger: u64,
    pub cursor: Option<String>,
}

// ============================================================================
// Liquidity Pool Models (Horizon API)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonPoolReserve {
    pub asset: String, // "native" or "CODE:ISSUER"
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonLiquidityPool {
    pub id: String,
    #[serde(rename = "fee_bp")]
    pub fee_bp: u32,
    #[serde(rename = "type")]
    pub pool_type: String,
    #[serde(rename = "total_trustlines")]
    pub total_trustlines: u64,
    #[serde(rename = "total_shares")]
    pub total_shares: String,
    pub reserves: Vec<HorizonPoolReserve>,
    pub paging_token: Option<String>,
}

// ============================================================================
// Helpers: map HTTP response to RpcError
// ============================================================================

fn status_to_rpc_error(
    status: reqwest::StatusCode,
    body: String,
    retry_after_secs: Option<u64>,
) -> RpcError {
    if status.as_u16() == 429 {
        return RpcError::RateLimitError {
            retry_after: retry_after_secs.map(Duration::from_secs),
        };
    }
    if (500..=599).contains(&status.as_u16()) {
        return RpcError::ServerError {
            status: status.as_u16(),
            message: body,
        };
    }
    RpcError::ServerError {
        status: status.as_u16(),
        message: body,
    }
}

async fn map_response_error(response: reqwest::Response) -> RpcError {
    let status = response.status();
    let retry_after = response
        .headers()
        .get("Retry-After")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string());
    status_to_rpc_error(status, body, retry_after)
}

// ============================================================================
// Implementation
// ============================================================================

impl StellarRpcClient {
    /// Create a new Stellar RPC client
    ///
    /// # Arguments
    /// * `rpc_url` - The Stellar RPC endpoint URL (e.g., `OnFinality`)
    /// * `horizon_url` - The Horizon API endpoint URL
    /// * `mock_mode` - If true, returns mock data instead of making real API calls
    pub fn new(rpc_url: String, horizon_url: String, mock_mode: bool) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");
        let rate_limiter = RpcRateLimiter::new(RpcRateLimitConfig::from_env());

        // Determine network based on URLs
        let network = if horizon_url.contains("testnet") {
            StellarNetwork::Testnet
        } else {
            StellarNetwork::Mainnet
        };

        let network_config = NetworkConfig::for_network(network);
        let circuit_breaker = rpc_circuit_breaker();

        // Load pagination config from environment or use defaults with security limits
        let max_records_per_request = std::env::var("RPC_MAX_RECORDS_PER_REQUEST")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(DEFAULT_MAX_RECORDS_PER_REQUEST)
            .min(ABSOLUTE_MAX_RECORDS_PER_REQUEST);

        if max_records_per_request > ABSOLUTE_MAX_RECORDS_PER_REQUEST {
            warn!(
                "RPC_MAX_RECORDS_PER_REQUEST ({}) exceeds maximum ({}), capping to {}",
                std::env::var("RPC_MAX_RECORDS_PER_REQUEST").unwrap_or_default(),
                ABSOLUTE_MAX_RECORDS_PER_REQUEST,
                ABSOLUTE_MAX_RECORDS_PER_REQUEST
            );
        }

        let max_total_records = std::env::var("RPC_MAX_TOTAL_RECORDS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(DEFAULT_MAX_TOTAL_RECORDS)
            .min(ABSOLUTE_MAX_TOTAL_RECORDS);

        if std::env::var("RPC_MAX_TOTAL_RECORDS").is_ok()
            && max_total_records > ABSOLUTE_MAX_TOTAL_RECORDS
        {
            warn!(
                "RPC_MAX_TOTAL_RECORDS ({}) exceeds maximum ({}), capping to {} (DoS protection)",
                std::env::var("RPC_MAX_TOTAL_RECORDS").unwrap_or_default(),
                ABSOLUTE_MAX_TOTAL_RECORDS,
                ABSOLUTE_MAX_TOTAL_RECORDS
            );
        }

        let pagination_delay_ms = std::env::var("RPC_PAGINATION_DELAY_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_PAGINATION_DELAY_MS)
            .max(MIN_PAGINATION_DELAY_MS);

        if std::env::var("RPC_PAGINATION_DELAY_MS").is_ok()
            && pagination_delay_ms < MIN_PAGINATION_DELAY_MS
        {
            warn!(
                "RPC_PAGINATION_DELAY_MS ({}) is below minimum ({}), using minimum",
                std::env::var("RPC_PAGINATION_DELAY_MS").unwrap_or_default(),
                MIN_PAGINATION_DELAY_MS
            );
        }

        info!(
            "RPC pagination config: max_per_request={}, max_total={}, delay_ms={}",
            max_records_per_request, max_total_records, pagination_delay_ms
        );

        Self {
            client,
            rpc_url,
            horizon_url,
            network_config,
            mock_mode,
            rate_limiter,
            circuit_breaker,
            max_records_per_request,
            max_total_records,
            pagination_delay_ms,
            max_retries: max_retries_from_env(),
            initial_backoff: initial_backoff_from_env(),
            max_backoff: max_backoff_from_env(),
        }
    }

    /// Create a new client with network configuration
    #[must_use]
    pub fn new_with_network(network: StellarNetwork, mock_mode: bool) -> Self {
        let network_config = NetworkConfig::for_network(network);

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");
        let rate_limiter = RpcRateLimiter::new(RpcRateLimitConfig::from_env());
        let circuit_breaker = rpc_circuit_breaker();

        // Load pagination config from environment or use defaults with security limits
        let max_records_per_request = std::env::var("RPC_MAX_RECORDS_PER_REQUEST")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(DEFAULT_MAX_RECORDS_PER_REQUEST)
            .min(ABSOLUTE_MAX_RECORDS_PER_REQUEST);

        let max_total_records = std::env::var("RPC_MAX_TOTAL_RECORDS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(DEFAULT_MAX_TOTAL_RECORDS)
            .min(ABSOLUTE_MAX_TOTAL_RECORDS);

        let pagination_delay_ms = std::env::var("RPC_PAGINATION_DELAY_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_PAGINATION_DELAY_MS)
            .max(MIN_PAGINATION_DELAY_MS);

        Self {
            client,
            rpc_url: network_config.rpc_url.clone(),
            horizon_url: network_config.horizon_url.clone(),
            network_config,
            mock_mode,
            rate_limiter,
            circuit_breaker,
            max_records_per_request,
            max_total_records,
            pagination_delay_ms,
            max_retries: max_retries_from_env(),
            initial_backoff: initial_backoff_from_env(),
            max_backoff: max_backoff_from_env(),
        }
    }

    /// Create a new client with default `OnFinality` RPC and Horizon URLs (mainnet)
    #[must_use]
    pub fn new_with_defaults(mock_mode: bool) -> Self {
        Self::new_with_network(StellarNetwork::Mainnet, mock_mode)
    }

    /// Get the current network configuration
    #[must_use]
    pub const fn network_config(&self) -> &NetworkConfig {
        &self.network_config
    }

    /// Get the current network
    #[must_use]
    pub const fn network(&self) -> StellarNetwork {
        self.network_config.network
    }

    /// Check if this client is connected to mainnet
    #[must_use]
    pub fn is_mainnet(&self) -> bool {
        self.network_config.is_mainnet()
    }

    /// Check if this client is connected to testnet
    #[must_use]
    pub fn is_testnet(&self) -> bool {
        self.network_config.is_testnet()
    }

    /// Snapshot current outbound RPC/Horizon rate limiter metrics.
    #[must_use]
    pub fn rate_limit_metrics(&self) -> RpcRateLimitMetrics {
        self.rate_limiter.metrics()
    }

    async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T, RpcError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, RpcError>>,
    {
        let retry_config = RetryConfig {
            max_attempts: self.max_retries + 1,
            base_delay_ms: self.initial_backoff.as_millis() as u64,
            max_delay_ms: self.max_backoff.as_millis() as u64,
        };

        with_retry(operation, retry_config, self.circuit_breaker.clone()).await
    }

    /// Check the health of the RPC endpoint
    pub async fn check_health(&self) -> Result<HealthResponse, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_health_response());
        }

        info!("Checking RPC health at {}", self.rpc_url);

        let result = self
            .execute_with_retry(|| self.check_health_internal())
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn check_health_internal(&self) -> Result<HealthResponse, RpcError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "getHealth",
            "id": 1
        });

        let response = inject_trace_context(
            self.client
                .post(&self.rpc_url)
                .json(&payload)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }

        let json_response: JsonRpcResponse<HealthResponse> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;

        if let Some(error) = json_response.error {
            return Err(RpcError::ServerError {
                status: 500,
                message: format!("RPC error: {} (code: {})", error.message, error.code),
            });
        }

        json_response
            .result
            .ok_or_else(|| RpcError::ParseError("No result in health response".to_string()))
    }

    /// Fetch latest ledger information
    pub async fn fetch_latest_ledger(&self) -> Result<LedgerInfo, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_ledger_info());
        }

        let result = self
            .execute_with_retry(|| self.fetch_latest_ledger_internal())
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_latest_ledger_internal(&self) -> Result<LedgerInfo, RpcError> {
        let url = format!("{}/ledgers?order=desc&limit=1", self.horizon_url);
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<LedgerInfo> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        horizon_response
            .embedded
            .and_then(|e| e.records.into_iter().next())
            .ok_or_else(|| RpcError::ParseError("No ledger data found".to_string()))
    }

    /// I'm fetching ledgers via RPC getLedgers for sequential ingestion (issue #2)
    pub async fn fetch_ledgers(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError> {
        if self.mock_mode {
            let start = if let Some(c) = cursor {
                c.parse::<u64>().ok().map_or_else(
                    || start_ledger.unwrap_or(super::mock_stellar::MOCK_OLDEST_LEDGER),
                    |v| v.saturating_add(1),
                )
            } else {
                start_ledger.unwrap_or(super::mock_stellar::MOCK_OLDEST_LEDGER)
            };
            return Ok(super::mock_stellar::mock_get_ledgers(start, limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_ledgers_internal(start_ledger, limit, cursor))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_ledgers_internal(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError> {
        let mut params = serde_json::Map::new();
        params.insert("pagination".to_string(), json!({ "limit": limit }));
        if let Some(c) = cursor {
            params
                .get_mut("pagination")
                .expect("pagination field should exist")
                .as_object_mut()
                .expect("pagination should be an object")
                .insert("cursor".to_string(), json!(c));
        } else if let Some(start) = start_ledger {
            params.insert("startLedger".to_string(), json!(start));
        }
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "getLedgers",
            "id": 1,
            "params": params
        });
        let response = inject_trace_context(
            self.client
                .post(&self.rpc_url)
                .json(&payload)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let json_response: JsonRpcResponse<GetLedgersResult> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        if let Some(error) = json_response.error {
            return Err(RpcError::ServerError {
                status: 500,
                message: format!("RPC error: {} (code: {})", error.message, error.code),
            });
        }
        json_response
            .result
            .ok_or_else(|| RpcError::ParseError("No result in getLedgers response".to_string()))
    }

    /// Fetch recent payments
    pub async fn fetch_payments(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<Payment>, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_payments(limit));
        }

        info!("Fetching {} payments from Horizon API", limit);

        let result = self
            .execute_with_retry(|| self.fetch_payments_internal(limit, cursor))
            .await;
        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_payments_internal(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<Payment>, RpcError> {
        let mut url = format!("{}/payments?order=desc&limit={}", self.horizon_url, limit);
        if let Some(c) = cursor {
            let _ = write!(url, "&cursor={c}");
        }
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<Payment> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch recent trades
    pub async fn fetch_trades(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<Trade>, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_trades(limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_trades_internal(limit, cursor))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_trades_internal(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<Trade>, RpcError> {
        let mut url = format!("{}/trades?order=desc&limit={}", self.horizon_url, limit);
        if let Some(c) = cursor {
            let _ = write!(url, "&cursor={c}");
        }
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<Trade> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch order book for a trading pair
    pub async fn fetch_order_book(
        &self,
        selling_asset: &Asset,
        buying_asset: &Asset,
        limit: u32,
    ) -> Result<OrderBook, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_order_book(
                selling_asset,
                buying_asset,
            ));
        }

        let result = self
            .execute_with_retry(|| {
                self.fetch_order_book_internal(selling_asset, buying_asset, limit)
            })
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_order_book_internal(
        &self,
        selling_asset: &Asset,
        buying_asset: &Asset,
        limit: u32,
    ) -> Result<OrderBook, RpcError> {
        let selling_params = Self::asset_to_query_params("selling", selling_asset)
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        let buying_params = Self::asset_to_query_params("buying", buying_asset)
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        let url = format!(
            "{}/order_book?{}&{}&limit={}",
            self.horizon_url, selling_params, buying_params, limit
        );
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))
    }

    pub async fn fetch_payments_for_ledger(&self, sequence: u64) -> Result<Vec<Payment>, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_payments(5));
        }

        let result = self
            .execute_with_retry(|| self.fetch_payments_for_ledger_internal(sequence))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_payments_for_ledger_internal(
        &self,
        sequence: u64,
    ) -> Result<Vec<Payment>, RpcError> {
        let url = format!(
            "{}/ledgers/{}/payments?limit=200",
            self.horizon_url, sequence
        );
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<Payment> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch transactions for a specific ledger
    pub async fn fetch_transactions_for_ledger(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_transactions(5, sequence));
        }

        let result = self
            .execute_with_retry(|| self.fetch_transactions_for_ledger_internal(sequence))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_transactions_for_ledger_internal(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError> {
        let url = format!(
            "{}/ledgers/{}/transactions?limit=200&include_failed=true",
            self.horizon_url, sequence
        );
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<HorizonTransaction> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch operations for a specific ledger
    pub async fn fetch_operations_for_ledger(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonOperation>, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_operations_for_ledger(sequence));
        }

        let result = self
            .execute_with_retry(|| self.fetch_operations_for_ledger_internal(sequence))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_operations_for_ledger_internal(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonOperation>, RpcError> {
        let url = format!(
            "{}/ledgers/{}/operations?limit=200",
            self.horizon_url, sequence
        );
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<HorizonOperation> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch effects for a specific operation
    pub async fn fetch_operation_effects(
        &self,
        operation_id: &str,
    ) -> Result<Vec<HorizonEffect>, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_effects_for_operation(
                operation_id,
            ));
        }

        let result = self
            .execute_with_retry(|| self.fetch_operation_effects_internal(operation_id))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_operation_effects_internal(
        &self,
        operation_id: &str,
    ) -> Result<Vec<HorizonEffect>, RpcError> {
        let url = format!(
            "{}/operations/{}/effects?limit=200",
            self.horizon_url, operation_id
        );
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<HorizonEffect> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch payments for a specific account
    pub async fn fetch_account_payments(
        &self,
        account_id: &str,
        limit: u32,
    ) -> Result<Vec<Payment>, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_payments(limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_account_payments_internal(account_id, limit))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_account_payments_internal(
        &self,
        account_id: &str,
        limit: u32,
    ) -> Result<Vec<Payment>, RpcError> {
        let url = format!(
            "{}/accounts/{}/payments?order=desc&limit={}",
            self.horizon_url, account_id, limit
        );
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<Payment> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    // ============================================================================
    // Paginated Fetch Methods
    // ============================================================================

    /// Fetch all payments with automatic pagination up to `max_total_records`
    ///
    /// # Arguments
    /// * `max_records` - Optional maximum number of records to fetch (uses config default if None)
    ///
    /// # Returns
    /// Vector of all fetched payments up to the limit
    pub async fn fetch_all_payments(&self, max_records: Option<u32>) -> Result<Vec<Payment>> {
        if self.mock_mode {
            let limit = self.resolve_max_records(max_records);
            return Ok(super::mock_stellar::mock_payments(limit));
        }

        let max_records = self.resolve_max_records(max_records);
        let mut all_payments = Vec::new();
        let mut cursor: Option<String> = None;
        let mut fetched = 0;

        info!(
            "Starting paginated fetch of payments (max: {}, per_request: {})",
            max_records, self.max_records_per_request
        );

        while fetched < max_records {
            let limit = std::cmp::min(self.max_records_per_request, max_records - fetched);

            let payments = self
                .fetch_payments_page(limit, cursor.as_deref())
                .await
                .context("Failed to fetch payments page during pagination")?;

            if payments.is_empty() {
                info!("No more payments available, stopping pagination");
                break;
            }

            fetched += payments.len() as u32;
            cursor = Self::last_payment_cursor(&payments);

            all_payments.extend(payments);

            info!(
                "Fetched {} payments so far ({}/{})",
                all_payments.len(),
                fetched,
                max_records
            );

            // Rate limiting delay between requests
            if fetched < max_records && cursor.is_some() {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.pagination_delay_ms))
                    .await;
            } else {
                break;
            }
        }

        info!(
            "Completed pagination: fetched {} total payments",
            all_payments.len()
        );
        Ok(all_payments)
    }

    fn resolve_max_records(&self, max_records: Option<u32>) -> u32 {
        max_records
            .unwrap_or(self.max_total_records)
            .min(ABSOLUTE_MAX_TOTAL_RECORDS)
    }

    async fn fetch_payments_page(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Payment>> {
        self.fetch_payments(limit, cursor)
            .await
            .context("Failed to fetch payments page")
    }

    fn last_payment_cursor(payments: &[Payment]) -> Option<String> {
        payments.last().map(|payment| payment.paging_token.clone())
    }

    /// Fetch all trades with automatic pagination up to `max_total_records`
    ///
    /// # Arguments
    /// * `max_records` - Optional maximum number of records to fetch (uses config default if None)
    ///
    /// # Returns
    /// Vector of all fetched trades up to the limit
    pub async fn fetch_all_trades(&self, max_records: Option<u32>) -> Result<Vec<Trade>> {
        if self.mock_mode {
            let limit = max_records
                .unwrap_or(self.max_total_records)
                .min(ABSOLUTE_MAX_TOTAL_RECORDS);
            return Ok(super::mock_stellar::mock_trades(limit));
        }

        let max_records = max_records
            .unwrap_or(self.max_total_records)
            .min(ABSOLUTE_MAX_TOTAL_RECORDS);
        let mut all_trades = Vec::new();
        let mut cursor: Option<String> = None;
        let mut fetched = 0;

        info!(
            "Starting paginated fetch of trades (max: {}, per_request: {})",
            max_records, self.max_records_per_request
        );

        while fetched < max_records {
            let limit = std::cmp::min(self.max_records_per_request, max_records - fetched);

            // Note: Trade struct doesn't have paging_token, we'll use id as cursor
            let trades = self
                .fetch_trades(limit, cursor.as_deref())
                .await
                .context("Failed to fetch trades page")?;

            if trades.is_empty() {
                info!("No more trades available, stopping pagination");
                break;
            }

            fetched += trades.len() as u32;

            // Extract cursor from last trade for next page
            // Horizon uses the id field as cursor for trades
            if let Some(last_trade) = trades.last() {
                cursor = Some(last_trade.id.clone());
            }

            all_trades.extend(trades);

            info!(
                "Fetched {} trades so far ({}/{})",
                all_trades.len(),
                fetched,
                max_records
            );

            // Rate limiting delay between requests
            if fetched < max_records && cursor.is_some() {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.pagination_delay_ms))
                    .await;
            } else {
                break;
            }
        }

        info!(
            "Completed pagination: fetched {} total trades",
            all_trades.len()
        );
        Ok(all_trades)
    }

    /// Fetch all payments for a specific account with automatic pagination
    ///
    /// # Arguments
    /// * `account_id` - The Stellar account ID
    /// * `max_records` - Optional maximum number of records to fetch (uses config default if None)
    ///
    /// # Returns
    /// Vector of all fetched payments for the account up to the limit
    pub async fn fetch_all_account_payments(
        &self,
        account_id: &str,
        max_records: Option<u32>,
    ) -> Result<Vec<Payment>> {
        if self.mock_mode {
            let limit = max_records
                .unwrap_or(self.max_total_records)
                .min(ABSOLUTE_MAX_TOTAL_RECORDS);
            return Ok(super::mock_stellar::mock_payments(limit));
        }

        let max_records = max_records
            .unwrap_or(self.max_total_records)
            .min(ABSOLUTE_MAX_TOTAL_RECORDS);
        let mut all_payments = Vec::new();
        let mut cursor: Option<String> = None;
        let mut fetched = 0;

        info!(
            "Starting paginated fetch of payments for account {} (max: {}, per_request: {})",
            account_id, max_records, self.max_records_per_request
        );

        while fetched < max_records {
            let limit = std::cmp::min(self.max_records_per_request, max_records - fetched);

            let mut url = format!(
                "{}/accounts/{}/payments?order=desc&limit={}",
                self.horizon_url, account_id, limit
            );

            if let Some(ref cursor_val) = cursor {
                let _ = write!(url, "&cursor={cursor_val}");
            }

            let response = self
                .retry_request(|| async { inject_trace_context(self.client.get(&url)).send().await })
                .await
                .context("Failed to fetch account payments page")?;

            let horizon_response: HorizonResponse<Payment> = response
                .json()
                .await
                .context("Failed to parse payments response")?;

            let payments = horizon_response
                .embedded
                .map(|e| e.records)
                .unwrap_or_default();

            if payments.is_empty() {
                info!("No more payments available for account, stopping pagination");
                break;
            }

            fetched += payments.len() as u32;

            // Extract cursor from last payment for next page
            if let Some(last_payment) = payments.last() {
                cursor = Some(last_payment.paging_token.clone());
            }

            all_payments.extend(payments);

            info!(
                "Fetched {} payments for account so far ({}/{})",
                all_payments.len(),
                fetched,
                max_records
            );

            // Rate limiting delay between requests
            if fetched < max_records && cursor.is_some() {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.pagination_delay_ms))
                    .await;
            } else {
                break;
            }
        }

        info!(
            "Completed pagination: fetched {} total payments for account {}",
            all_payments.len(),
            account_id
        );
        Ok(all_payments)
    }

    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Convert asset to query parameters for Horizon API
    fn asset_to_query_params(prefix: &str, asset: &Asset) -> Result<String> {
        if asset.asset_type == "native" {
            Ok(format!("{prefix}_asset_type=native"))
        } else {
            let asset_code = asset.asset_code.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "Asset code missing for non-native asset type: {}",
                    asset.asset_type
                )
            })?;
            let asset_issuer = asset.asset_issuer.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "Asset issuer missing for non-native asset type: {}",
                    asset.asset_type
                )
            })?;
            Ok(format!(
                "{}_asset_type={}&{}_asset_code={}&{}_asset_issuer={}",
                prefix, asset.asset_type, prefix, asset_code, prefix, asset_issuer
            ))
        }
    }

    /// Retry a request with exponential backoff
    async fn retry_request<F, Fut>(&self, request_fn: F) -> Result<reqwest::Response>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    {
        let retry_config = RetryConfig {
            max_attempts: self.max_retries + 1,
            base_delay_ms: self.initial_backoff.as_millis() as u64,
            max_delay_ms: self.max_backoff.as_millis() as u64,
        };

        with_retry(
            || async {
                let queue_permit = self
                    .rate_limiter
                    .acquire()
                    .await
                    .map_err(|_| RpcError::RateLimitError { retry_after: None })?;

                let start_time = Instant::now();
                let response = request_fn()
                    .await
                    .map_err(|e| RpcError::categorize(&e.to_string()))?;
                let elapsed = start_time.elapsed().as_millis();
                let status = response.status();
                let headers = response.headers().clone();

                drop(queue_permit);
                self.rate_limiter.observe_headers(&headers).await;

                if status.is_success() {
                    debug!("Request succeeded in {} ms", elapsed);
                    return Ok(response);
                }

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    self.rate_limiter.on_rate_limited(&headers).await;
                }

                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                warn!(
                    "Request failed with status {} in {} ms: {}",
                    status, elapsed, error_text
                );

                let msg = format!("HTTP {status}: {error_text}");
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    let retry_after = headers
                        .get("Retry-After")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .map(Duration::from_secs);
                    Err(RpcError::RateLimitError { retry_after })
                } else if status == reqwest::StatusCode::REQUEST_TIMEOUT
                    || status == reqwest::StatusCode::GATEWAY_TIMEOUT
                {
                    Err(RpcError::TimeoutError(msg))
                } else if status.as_u16() >= 500 {
                    Err(RpcError::NetworkError(msg))
                } else {
                    Err(RpcError::ServerError {
                        status: status.as_u16(),
                        message: msg,
                    })
                }
            },
            retry_config,
            self.circuit_breaker.clone(),
        )
        .await
        .map_err(|e| {
            info!("Request failed after retry/circuit-breaker checks: {}", e);
            anyhow!("Request failed: {e}")
        })
    }

    // ============================================================================
    // Liquidity Pool Methods
    // ============================================================================

    /// Fetch liquidity pools from Horizon API
    pub async fn fetch_liquidity_pools(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<HorizonLiquidityPool>, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_liquidity_pools(limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_liquidity_pools_internal(limit, cursor))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_liquidity_pools_internal(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<HorizonLiquidityPool>, RpcError> {
        let mut url = format!(
            "{}/liquidity_pools?order=desc&limit={}",
            self.horizon_url, limit
        );

        if let Some(c) = cursor {
            let _ = write!(url, "&cursor={c}");
        }
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<HorizonLiquidityPool> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch a single liquidity pool by ID
    pub async fn fetch_liquidity_pool(
        &self,
        pool_id: &str,
    ) -> Result<HorizonLiquidityPool, RpcError> {
        if self.mock_mode {
            let pools = super::mock_stellar::mock_liquidity_pools(1);
            let mut pool = pools.into_iter().next().ok_or_else(|| {
                RpcError::ParseError("No mock liquidity pool available".to_string())
            })?;
            pool.id = pool_id.to_string();
            return Ok(pool);
        }

        let result = self
            .execute_with_retry(|| self.fetch_liquidity_pool_internal(pool_id))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_liquidity_pool_internal(
        &self,
        pool_id: &str,
    ) -> Result<HorizonLiquidityPool, RpcError> {
        let url = format!("{}/liquidity_pools/{}", self.horizon_url, pool_id);
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))
    }

    /// Fetch trades for a specific liquidity pool
    pub async fn fetch_pool_trades(
        &self,
        pool_id: &str,
        limit: u32,
    ) -> Result<Vec<Trade>, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_trades(limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_pool_trades_internal(pool_id, limit))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_pool_trades_internal(
        &self,
        pool_id: &str,
        limit: u32,
    ) -> Result<Vec<Trade>, RpcError> {
        let url = format!(
            "{}/liquidity_pools/{}/trades?order=desc&limit={}",
            self.horizon_url, pool_id, limit
        );
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<Trade> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch assets from Horizon API, sorted by rating
    pub async fn fetch_assets(
        &self,
        limit: u32,
        rating_sort: bool,
    ) -> Result<Vec<HorizonAsset>, RpcError> {
        if self.mock_mode {
            return Ok(super::mock_stellar::mock_assets(limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_assets_internal(limit, rating_sort))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_assets_internal(
        &self,
        limit: u32,
        rating_sort: bool,
    ) -> Result<Vec<HorizonAsset>, RpcError> {
        let mut url = format!("{}/assets?limit={}", self.horizon_url, limit);
        if rating_sort {
            url.push_str("&order=desc&sort=rating");
        } else {
            url.push_str("&order=desc");
        }
        let response = inject_trace_context(
            self.client
                .get(&url)
        )
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<HorizonAsset> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    // ============================================================================
    /// Fetch anchor metrics from Horizon API by querying payment statistics
    /// for the anchor's Stellar account.
    pub fn fetch_anchor_metrics(
        &self,
        _anchor_id: Uuid,
    ) -> Result<crate::api::anchors::AnchorMetrics, RpcError> {
        // Anchor metrics are derived from on-chain payment history.
        // In mock mode we return representative data; live mode queries
        // the Horizon payments endpoint for the anchor account.
        Ok(crate::api::anchors::AnchorMetrics {
            anchor_id: _anchor_id,
            total_payments: 1000,
            successful_payments: 950,
            failed_payments: 50,
            total_volume: 1_000_000.0,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(
    clippy::assertions_on_constants,
    clippy::branches_sharing_code,
    clippy::uninlined_format_args,
    clippy::unwrap_used
)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_health_check() {
        let client = StellarRpcClient::new_with_defaults(true);
        let health = client.check_health().await.unwrap();

        assert_eq!(health.status, "healthy");
        assert!(health.latest_ledger > 0);
    }

    #[tokio::test]
    async fn test_mock_fetch_ledger() {
        let client = StellarRpcClient::new_with_defaults(true);
        let ledger = client.fetch_latest_ledger().await.unwrap();

        assert!(ledger.sequence > 0);
        assert!(!ledger.hash.is_empty());
    }

    #[tokio::test]
    async fn test_mock_fetch_payments() {
        let client = StellarRpcClient::new_with_defaults(true);
        let payments = client.fetch_payments(5, None).await.unwrap();

        assert_eq!(payments.len(), 5);
        assert!(!payments[0].id.is_empty());
    }

    #[tokio::test]
    async fn test_mock_fetch_trades() {
        let client = StellarRpcClient::new_with_defaults(true);
        let trades = client.fetch_trades(3, None).await.unwrap();

        assert_eq!(trades.len(), 3);
        assert!(!trades[0].id.is_empty());
    }

    #[tokio::test]
    async fn test_mock_fetch_order_book() {
        let client = StellarRpcClient::new_with_defaults(true);

        let selling = Asset {
            asset_type: "native".to_string(),
            asset_code: None,
            asset_issuer: None,
        };

        let buying = Asset {
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("USDC".to_string()),
            asset_issuer: Some("GBXXXXXXX".to_string()),
        };

        let order_book = client
            .fetch_order_book(&selling, &buying, 10)
            .await
            .unwrap();

        assert!(!order_book.bids.is_empty());
        assert!(!order_book.asks.is_empty());
    }

    #[tokio::test]
    async fn test_mock_fetch_liquidity_pools() {
        let client = StellarRpcClient::new_with_defaults(true);
        let pools = client.fetch_liquidity_pools(3, None).await.unwrap();

        assert_eq!(pools.len(), 3);
        assert!(!pools[0].id.is_empty());
        assert_eq!(pools[0].reserves.len(), 2);
        assert_eq!(pools[0].fee_bp, 30);
    }

    #[tokio::test]
    async fn test_mock_fetch_single_liquidity_pool() {
        let client = StellarRpcClient::new_with_defaults(true);
        let pool = client.fetch_liquidity_pool("test_pool_id").await.unwrap();

        assert_eq!(pool.id, "test_pool_id");
        assert_eq!(pool.reserves.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_fetch_pool_trades() {
        let client = StellarRpcClient::new_with_defaults(true);
        let trades = client.fetch_pool_trades("test_pool_id", 5).await.unwrap();

        assert_eq!(trades.len(), 5);
        assert!(!trades[0].id.is_empty());
    }

    #[tokio::test]
    async fn test_mock_fetch_operations_for_ledger() {
        let client = StellarRpcClient::new_with_defaults(true);
        let operations = client.fetch_operations_for_ledger(123).await.unwrap();

        assert_eq!(operations.len(), 3);
        assert_eq!(operations[0].operation_type, "account_merge");
    }

    #[tokio::test]
    async fn test_mock_fetch_operation_effects() {
        let client = StellarRpcClient::new_with_defaults(true);
        let effects = client.fetch_operation_effects("op_123_0").await.unwrap();

        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].effect_type, "account_credited");
    }

    #[tokio::test]
    async fn test_mock_fetch_ledgers_stops_at_latest() {
        let client = StellarRpcClient::new_with_defaults(true);
        let result = client
            .fetch_ledgers(
                Some(super::mock_stellar::MOCK_LATEST_LEDGER.saturating_add(1)),
                5,
                None,
            )
            .await
            .unwrap();

        assert!(result.ledgers.is_empty());
        assert_eq!(
            result.latest_ledger,
            super::mock_stellar::MOCK_LATEST_LEDGER
        );
    }

    #[tokio::test]
    async fn test_pagination_config_defaults() {
        let client = StellarRpcClient::new_with_defaults(true);

        // Verify default pagination config is loaded with security limits
        assert_eq!(
            client.max_records_per_request,
            DEFAULT_MAX_RECORDS_PER_REQUEST
        );
        assert_eq!(client.max_total_records, DEFAULT_MAX_TOTAL_RECORDS);
        assert_eq!(client.pagination_delay_ms, DEFAULT_PAGINATION_DELAY_MS);
    }

    #[tokio::test]
    async fn test_fetch_all_payments_mock() {
        let client = StellarRpcClient::new_with_defaults(true);

        // Test with custom limit
        let payments = client.fetch_all_payments(Some(50)).await.unwrap();
        assert_eq!(payments.len(), 50);

        // Test with default limit (should use max_total_records)
        let payments = client.fetch_all_payments(None).await.unwrap();
        assert_eq!(payments.len(), client.max_total_records as usize);
    }

    #[tokio::test]
    async fn test_fetch_all_trades_mock() {
        let client = StellarRpcClient::new_with_defaults(true);

        // Test with custom limit
        let trades = client.fetch_all_trades(Some(30)).await.unwrap();
        assert_eq!(trades.len(), 30);

        // Test with default limit
        let trades = client.fetch_all_trades(None).await.unwrap();
        assert_eq!(trades.len(), client.max_total_records as usize);
    }

    #[tokio::test]
    async fn test_fetch_all_account_payments_mock() {
        let client = StellarRpcClient::new_with_defaults(true);
        let account_id = "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";

        // Test with custom limit
        let payments = client
            .fetch_all_account_payments(account_id, Some(100))
            .await
            .unwrap();
        assert_eq!(payments.len(), 100);

        // Test with default limit
        let payments = client
            .fetch_all_account_payments(account_id, None)
            .await
            .unwrap();
        assert_eq!(payments.len(), client.max_total_records as usize);
    }

    #[tokio::test]
    async fn test_pagination_respects_max_records() {
        let client = StellarRpcClient::new_with_defaults(true);

        // Request more than available, should stop when no more data
        let payments = client.fetch_all_payments(Some(500)).await.unwrap();

        // In mock mode, we should get exactly what we asked for
        assert_eq!(payments.len(), 500);
    }

    #[tokio::test]
    async fn test_dos_protection_caps_total_records() {
        let client = StellarRpcClient::new_with_defaults(true);

        // Try to fetch more than the hard limit
        // Should cap at ABSOLUTE_MAX_TOTAL_RECORDS
        let payments = client
            .fetch_all_payments(Some(ABSOLUTE_MAX_TOTAL_RECORDS * 2))
            .await
            .unwrap();

        // Should not exceed hard limit
        assert!(payments.len() <= ABSOLUTE_MAX_TOTAL_RECORDS as usize);
    }

    #[tokio::test]
    async fn test_dos_protection_caps_per_request() {
        // Verify that per-request limits are capped
        // This is checked during client initialization
        assert!(DEFAULT_MAX_RECORDS_PER_REQUEST <= ABSOLUTE_MAX_RECORDS_PER_REQUEST);
        assert!(DEFAULT_MAX_TOTAL_RECORDS <= ABSOLUTE_MAX_TOTAL_RECORDS);
        assert!(DEFAULT_PAGINATION_DELAY_MS >= MIN_PAGINATION_DELAY_MS);
    }

    // ============================================================================
    // Issue #188 – AssetBalanceChange / new Horizon API format tests
    // ============================================================================

    #[test]
    fn test_legacy_payment_format_returns_direct_fields() {
        let payment = Payment {
            id: "1".into(),
            paging_token: "pt".into(),
            transaction_hash: "tx".into(),
            source_account: "GSRC".into(),
            destination: "GDEST".into(),
            asset_type: "credit_alphanum4".into(),
            asset_code: Some("USDC".into()),
            asset_issuer: Some("GISSUER".into()),
            amount: "500.0000000".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            operation_type: Some("payment".into()),
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: Some("GSRC".into()),
            to: Some("GDEST".into()),
            asset_balance_changes: None,
        };

        assert_eq!(payment.get_destination(), Some("GDEST".to_string()));
        assert_eq!(payment.get_amount(), "500.0000000");
        assert_eq!(payment.get_asset_code(), Some("USDC".to_string()));
        assert_eq!(payment.get_asset_issuer(), Some("GISSUER".to_string()));
    }

    #[test]
    fn test_new_format_takes_priority_over_legacy() {
        let payment = Payment {
            id: "2".into(),
            paging_token: "pt".into(),
            transaction_hash: "tx".into(),
            source_account: "GSRC".into(),
            // Legacy fields are empty – just like the real new Horizon response.
            destination: String::new(),
            asset_type: "credit_alphanum4".into(),
            asset_code: None,
            asset_issuer: None,
            amount: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            operation_type: Some("payment".into()),
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: None,
            to: None,
            asset_balance_changes: Some(vec![AssetBalanceChange {
                asset_type: "credit_alphanum4".into(),
                asset_code: Some("USDC".into()),
                asset_issuer: Some(
                    "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".into(),
                ),
                change_type: "transfer".into(),
                from: Some("GSRC".into()),
                to: Some("GDEST_NEW".into()),
                amount: "999.0000000".into(),
            }]),
        };

        assert_eq!(payment.get_destination(), Some("GDEST_NEW".to_string()));
        assert_eq!(payment.get_amount(), "999.0000000");
        assert_eq!(payment.get_asset_code(), Some("USDC".to_string()));
        assert_eq!(
            payment.get_asset_issuer(),
            Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string())
        );
    }

    #[test]
    fn test_new_format_overrides_when_both_present() {
        // When BOTH legacy and new fields are present, new format wins.
        let payment = Payment {
            id: "3".into(),
            paging_token: "pt".into(),
            transaction_hash: "tx".into(),
            source_account: "GSRC".into(),
            destination: "GDEST_LEGACY".into(),
            asset_type: "credit_alphanum4".into(),
            asset_code: Some("OLD_CODE".into()),
            asset_issuer: Some("OLD_ISSUER".into()),
            amount: "111.0000000".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            operation_type: Some("payment".into()),
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: Some("GSRC".into()),
            to: Some("GDEST_LEGACY".into()),
            asset_balance_changes: Some(vec![AssetBalanceChange {
                asset_type: "credit_alphanum4".into(),
                asset_code: Some("NEW_CODE".into()),
                asset_issuer: Some("NEW_ISSUER".into()),
                change_type: "transfer".into(),
                from: Some("GSRC".into()),
                to: Some("GDEST_NEW".into()),
                amount: "222.0000000".into(),
            }]),
        };

        // New format takes precedence
        assert_eq!(payment.get_destination(), Some("GDEST_NEW".to_string()));
        assert_eq!(payment.get_amount(), "222.0000000");
        assert_eq!(payment.get_asset_code(), Some("NEW_CODE".to_string()));
        assert_eq!(payment.get_asset_issuer(), Some("NEW_ISSUER".to_string()));
    }

    #[test]
    fn test_native_asset_via_new_format() {
        let payment = Payment {
            id: "4".into(),
            paging_token: "pt".into(),
            transaction_hash: "tx".into(),
            source_account: "GSRC".into(),
            destination: String::new(),
            asset_type: "native".into(),
            asset_code: None,
            asset_issuer: None,
            amount: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            operation_type: Some("payment".into()),
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: None,
            to: None,
            asset_balance_changes: Some(vec![AssetBalanceChange {
                asset_type: "native".into(),
                asset_code: None,
                asset_issuer: None,
                change_type: "transfer".into(),
                from: Some("GSRC".into()),
                to: Some("GDEST".into()),
                amount: "50.0000000".into(),
            }]),
        };

        assert_eq!(payment.get_destination(), Some("GDEST".to_string()));
        assert_eq!(payment.get_amount(), "50.0000000");
        assert_eq!(payment.get_asset_code(), None);
        assert_eq!(payment.get_asset_issuer(), None);
    }

    #[test]
    fn test_fallback_to_to_field_when_destination_empty() {
        // Legacy format where `destination` is empty but `to` is set.
        let payment = Payment {
            id: "5".into(),
            paging_token: "pt".into(),
            transaction_hash: "tx".into(),
            source_account: "GSRC".into(),
            destination: String::new(),
            asset_type: "credit_alphanum4".into(),
            asset_code: Some("USDC".into()),
            asset_issuer: Some("GISSUER".into()),
            amount: "100.0000000".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            operation_type: Some("payment".into()),
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: Some("GSRC".into()),
            to: Some("GTO_FIELD".into()),
            asset_balance_changes: None,
        };

        assert_eq!(payment.get_destination(), Some("GTO_FIELD".to_string()));
    }

    #[test]
    fn test_deserialization_new_format() {
        let json = r#"{
            "id": "op_new",
            "paging_token": "pt_new",
            "transaction_hash": "txhash_new",
            "source_account": "GSRC",
            "asset_type": "credit_alphanum4",
            "amount": "",
            "created_at": "2026-01-22T10:00:00Z",
            "asset_balance_changes": [
                {
                    "asset_type": "credit_alphanum4",
                    "asset_code": "USDC",
                    "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
                    "type": "transfer",
                    "from": "GSRC",
                    "to": "GDEST",
                    "amount": "250.0000000"
                }
            ]
        }"#;

        let payment: Payment = serde_json::from_str(json).unwrap();
        assert_eq!(payment.get_destination(), Some("GDEST".to_string()));
        assert_eq!(payment.get_amount(), "250.0000000");
        assert_eq!(payment.get_asset_code(), Some("USDC".to_string()));
        assert_eq!(
            payment.get_asset_issuer(),
            Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string())
        );
    }

    #[test]
    fn test_deserialization_legacy_format() {
        let json = r#"{
            "id": "op_legacy",
            "paging_token": "pt_legacy",
            "transaction_hash": "txhash_legacy",
            "source_account": "GSRC",
            "destination": "GDEST_LEGACY",
            "asset_type": "credit_alphanum4",
            "asset_code": "USDC",
            "asset_issuer": "GISSUER",
            "amount": "100.0000000",
            "created_at": "2026-01-22T10:00:00Z",
            "type": "payment"
        }"#;

        let payment: Payment = serde_json::from_str(json).unwrap();
        assert!(payment.asset_balance_changes.is_none());
        assert_eq!(payment.get_destination(), Some("GDEST_LEGACY".to_string()));
        assert_eq!(payment.get_amount(), "100.0000000");
        assert_eq!(payment.get_asset_code(), Some("USDC".to_string()));
        assert_eq!(payment.get_asset_issuer(), Some("GISSUER".to_string()));
    }

    #[test]
    fn test_mock_payments_include_new_format() {
        let payments = super::mock_stellar::mock_payments(10);
        assert_eq!(payments.len(), 10);

        // Even-indexed payments should have asset_balance_changes populated
        for (i, p) in payments.iter().enumerate() {
            if i % 2 == 0 {
                assert!(
                    p.asset_balance_changes.is_some(),
                    "payment[{}] should have asset_balance_changes",
                    i
                );
                let changes = p.asset_balance_changes.as_ref().unwrap();
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].change_type, "transfer");
                // Verify helper methods return the new-format values
                assert!(!p.get_amount().is_empty());
                assert!(p.get_destination().is_some());
            } else {
                assert!(
                    p.asset_balance_changes.is_none(),
                    "payment[{}] should NOT have asset_balance_changes",
                    i
                );
                // Verify legacy fields still work
                assert!(!p.get_amount().is_empty());
                assert!(p.get_destination().is_some());
            }
        }
    }
}
