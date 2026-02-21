use crate::network::{NetworkConfig, StellarNetwork};
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 100;
const BACKOFF_MULTIPLIER: u64 = 2;
const MOCK_OLDEST_LEDGER: u64 = 51_565_760;
const MOCK_LATEST_LEDGER: u64 = 51_565_820;

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
    /// Maximum records per single request (default: 200)
    max_records_per_request: u32,
    /// Maximum total records across all paginated requests (default: 10000)
    max_total_records: u32,
    /// Delay between pagination requests in milliseconds (default: 100)
    pagination_delay_ms: u64,
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
}

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
    pub fee_charged: Option<String>, // Can be number or string, Horizon usually string
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
    pub price_r: Price,
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

// I'm adding structs for getLedgers RPC method as required by issue #2
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
// Implementation
// ============================================================================

impl StellarRpcClient {
    /// Create a new Stellar RPC client
    ///
    /// # Arguments
    /// * `rpc_url` - The Stellar RPC endpoint URL (e.g., OnFinality)
    /// * `horizon_url` - The Horizon API endpoint URL
    /// * `mock_mode` - If true, returns mock data instead of making real API calls
    pub fn new(rpc_url: String, horizon_url: String, mock_mode: bool) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        // Determine network based on URLs
        let network = if horizon_url.contains("testnet") {
            StellarNetwork::Testnet
        } else {
            StellarNetwork::Mainnet
        };

        let network_config = NetworkConfig::for_network(network);

        // Load pagination config from environment or use defaults
        let max_records_per_request = std::env::var("RPC_MAX_RECORDS_PER_REQUEST")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(200);

        let max_total_records = std::env::var("RPC_MAX_TOTAL_RECORDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10000);

        let pagination_delay_ms = std::env::var("RPC_PAGINATION_DELAY_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        Self {
            client,
            rpc_url,
            horizon_url,
            network_config,
            mock_mode,
            max_records_per_request,
            max_total_records,
            pagination_delay_ms,
        }
    }

    /// Create a new client with network configuration
    pub fn new_with_network(network: StellarNetwork, mock_mode: bool) -> Self {
        let network_config = NetworkConfig::for_network(network);

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        // Load pagination config from environment or use defaults
        let max_records_per_request = std::env::var("RPC_MAX_RECORDS_PER_REQUEST")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(200);

        let max_total_records = std::env::var("RPC_MAX_TOTAL_RECORDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10000);

        let pagination_delay_ms = std::env::var("RPC_PAGINATION_DELAY_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        Self {
            client,
            rpc_url: network_config.rpc_url.clone(),
            horizon_url: network_config.horizon_url.clone(),
            network_config,
            mock_mode,
            max_records_per_request,
            max_total_records,
            pagination_delay_ms,
        }
    }

    /// Create a new client with default OnFinality RPC and Horizon URLs (mainnet)
    pub fn new_with_defaults(mock_mode: bool) -> Self {
        Self::new_with_network(StellarNetwork::Mainnet, mock_mode)
    }

    /// Get the current network configuration
    pub fn network_config(&self) -> &NetworkConfig {
        &self.network_config
    }

    /// Get the current network
    pub fn network(&self) -> StellarNetwork {
        self.network_config.network
    }

    /// Check if this client is connected to mainnet
    pub fn is_mainnet(&self) -> bool {
        self.network_config.is_mainnet()
    }

    /// Check if this client is connected to testnet
    pub fn is_testnet(&self) -> bool {
        self.network_config.is_testnet()
    }

    /// Check the health of the RPC endpoint
    pub async fn check_health(&self) -> Result<HealthResponse> {
        if self.mock_mode {
            return Ok(Self::mock_health_response());
        }

        info!("Checking RPC health at {}", self.rpc_url);

        let payload = json!({
            "jsonrpc": "2.0",
            "method": "getHealth",
            "id": 1
        });

        let response = self
            .retry_request(|| async { self.client.post(&self.rpc_url).json(&payload).send().await })
            .await
            .context("Failed to check RPC health")?;

        let json_response: JsonRpcResponse<HealthResponse> = response
            .json()
            .await
            .context("Failed to parse health response")?;

        if let Some(error) = json_response.error {
            anyhow::bail!("RPC error: {} (code: {})", error.message, error.code);
        }

        json_response.result.context("No result in health response")
    }

    /// Fetch latest ledger information
    pub async fn fetch_latest_ledger(&self) -> Result<LedgerInfo> {
        if self.mock_mode {
            return Ok(Self::mock_ledger_info());
        }

        info!("Fetching latest ledger from Horizon API");

        let url = format!("{}/ledgers?order=desc&limit=1", self.horizon_url);

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch latest ledger")?;

        let horizon_response: HorizonResponse<LedgerInfo> = response
            .json()
            .await
            .context("Failed to parse ledger response")?;

        let ledger = horizon_response
            .embedded
            .and_then(|e| e.records.into_iter().next())
            .context("No ledger data found")?;

        Ok(ledger)
    }

    /// I'm fetching ledgers via RPC getLedgers for sequential ingestion (issue #2)
    pub async fn fetch_ledgers(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<GetLedgersResult> {
        if self.mock_mode {
            let start = if let Some(c) = cursor {
                c.parse::<u64>()
                    .ok()
                    .map(|v| v.saturating_add(1))
                    .unwrap_or_else(|| start_ledger.unwrap_or(MOCK_OLDEST_LEDGER))
            } else {
                start_ledger.unwrap_or(MOCK_OLDEST_LEDGER)
            };
            return Ok(Self::mock_get_ledgers(start, limit));
        }

        info!("Fetching ledgers via RPC getLedgers");

        let mut params = serde_json::Map::new();
        params.insert("pagination".to_string(), json!({ "limit": limit }));

        // I must use either startLedger or cursor, not both
        if let Some(c) = cursor {
            params
                .get_mut("pagination")
                .unwrap()
                .as_object_mut()
                .unwrap()
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

        let response = self
            .retry_request(|| async { self.client.post(&self.rpc_url).json(&payload).send().await })
            .await
            .context("Failed to fetch ledgers")?;

        let json_response: JsonRpcResponse<GetLedgersResult> = response
            .json()
            .await
            .context("Failed to parse getLedgers response")?;

        if let Some(error) = json_response.error {
            anyhow::bail!("RPC error: {} (code: {})", error.message, error.code);
        }

        json_response
            .result
            .context("No result in getLedgers response")
    }

    /// Fetch recent payments
    pub async fn fetch_payments(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Payment>> {
        if self.mock_mode {
            return Ok(Self::mock_payments(limit));
        }

        info!("Fetching {} payments from Horizon API", limit);

        let mut url = format!("{}/payments?order=desc&limit={}", self.horizon_url, limit);

        if let Some(cursor) = cursor {
            url.push_str(&format!("&cursor={}", cursor));
        }

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch payments")?;

        let horizon_response: HorizonResponse<Payment> = response
            .json()
            .await
            .context("Failed to parse payments response")?;

        let payments = horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default();

        Ok(payments)
    }

    /// Fetch recent trades
    pub async fn fetch_trades(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Trade>> {
        if self.mock_mode {
            return Ok(Self::mock_trades(limit));
        }

        info!("Fetching {} trades from Horizon API", limit);

        let mut url = format!("{}/trades?order=desc&limit={}", self.horizon_url, limit);

        if let Some(cursor) = cursor {
            url.push_str(&format!("&cursor={}", cursor));
        }

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch trades")?;

        let horizon_response: HorizonResponse<Trade> = response
            .json()
            .await
            .context("Failed to parse trades response")?;

        let trades = horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default();

        Ok(trades)
    }

    /// Fetch order book for a trading pair
    pub async fn fetch_order_book(
        &self,
        selling_asset: &Asset,
        buying_asset: &Asset,
        limit: u32,
    ) -> Result<OrderBook> {
        if self.mock_mode {
            return Ok(Self::mock_order_book(selling_asset, buying_asset));
        }

        info!("Fetching order book from Horizon API");

        let selling_params = Self::asset_to_query_params("selling", selling_asset);
        let buying_params = Self::asset_to_query_params("buying", buying_asset);

        let url = format!(
            "{}/order_book?{}&{}&limit={}",
            self.horizon_url, selling_params, buying_params, limit
        );

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch order book")?;

        let order_book: OrderBook = response
            .json()
            .await
            .context("Failed to parse order book response")?;

        Ok(order_book)
    }

    pub async fn fetch_payments_for_ledger(&self, sequence: u64) -> Result<Vec<Payment>> {
        if self.mock_mode {
            return Ok(Self::mock_payments(5));
        }

        let url = format!(
            "{}/ledgers/{}/payments?limit=200",
            self.horizon_url, sequence
        );

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch ledger payments")?;

        let horizon_response: HorizonResponse<Payment> = response
            .json()
            .await
            .context("Failed to parse ledger payments response")?;

        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch transactions for a specific ledger
    pub async fn fetch_transactions_for_ledger(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonTransaction>> {
        if self.mock_mode {
            return Ok(Self::mock_transactions(5, sequence));
        }

        let url = format!(
            "{}/ledgers/{}/transactions?limit=200&include_failed=true",
            self.horizon_url, sequence
        );

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch ledger transactions")?;

        let horizon_response: HorizonResponse<HorizonTransaction> = response
            .json()
            .await
            .context("Failed to parse ledger transactions response")?;

        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch operations for a specific ledger
    pub async fn fetch_operations_for_ledger(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonOperation>> {
        if self.mock_mode {
            return Ok(Self::mock_operations_for_ledger(sequence));
        }

        let url = format!(
            "{}/ledgers/{}/operations?limit=200",
            self.horizon_url, sequence
        );

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch ledger operations")?;

        let horizon_response: HorizonResponse<HorizonOperation> = response
            .json()
            .await
            .context("Failed to parse ledger operations response")?;

        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch effects for a specific operation
    pub async fn fetch_operation_effects(&self, operation_id: &str) -> Result<Vec<HorizonEffect>> {
        if self.mock_mode {
            return Ok(Self::mock_effects_for_operation(operation_id));
        }

        let url = format!(
            "{}/operations/{}/effects?limit=200",
            self.horizon_url, operation_id
        );

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch operation effects")?;

        let horizon_response: HorizonResponse<HorizonEffect> = response
            .json()
            .await
            .context("Failed to parse operation effects response")?;

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
    ) -> Result<Vec<Payment>> {
        if self.mock_mode {
            return Ok(Self::mock_payments(limit));
        }

        info!(
            "Fetching {} payments for account {} from Horizon API",
            limit, account_id
        );

        let url = format!(
            "{}/accounts/{}/payments?order=desc&limit={}",
            self.horizon_url, account_id, limit
        );

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch account payments")?;

        let horizon_response: HorizonResponse<Payment> = response
            .json()
            .await
            .context("Failed to parse payments response")?;

        let payments = horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default();

        Ok(payments)
    }

    // ============================================================================
    // Paginated Fetch Methods
    // ============================================================================

    /// Fetch all payments with automatic pagination up to max_total_records
    ///
    /// # Arguments
    /// * `max_records` - Optional maximum number of records to fetch (uses config default if None)
    ///
    /// # Returns
    /// Vector of all fetched payments up to the limit
    pub async fn fetch_all_payments(&self, max_records: Option<u32>) -> Result<Vec<Payment>> {
        if self.mock_mode {
            let limit = max_records.unwrap_or(self.max_total_records);
            return Ok(Self::mock_payments(limit));
        }

        let max_records = max_records.unwrap_or(self.max_total_records);
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
                .fetch_payments(limit, cursor.as_deref())
                .await
                .context("Failed to fetch payments page")?;

            if payments.is_empty() {
                info!("No more payments available, stopping pagination");
                break;
            }

            fetched += payments.len() as u32;

            // Extract cursor from last payment for next page
            if let Some(last_payment) = payments.last() {
                cursor = Some(last_payment.paging_token.clone());
            }

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

    /// Fetch all trades with automatic pagination up to max_total_records
    ///
    /// # Arguments
    /// * `max_records` - Optional maximum number of records to fetch (uses config default if None)
    ///
    /// # Returns
    /// Vector of all fetched trades up to the limit
    pub async fn fetch_all_trades(&self, max_records: Option<u32>) -> Result<Vec<Trade>> {
        if self.mock_mode {
            let limit = max_records.unwrap_or(self.max_total_records);
            return Ok(Self::mock_trades(limit));
        }

        let max_records = max_records.unwrap_or(self.max_total_records);
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
            let limit = max_records.unwrap_or(self.max_total_records);
            return Ok(Self::mock_payments(limit));
        }

        let max_records = max_records.unwrap_or(self.max_total_records);
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
                url.push_str(&format!("&cursor={}", cursor_val));
            }

            let response = self
                .retry_request(|| async { self.client.get(&url).send().await })
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
    fn asset_to_query_params(prefix: &str, asset: &Asset) -> String {
        if asset.asset_type == "native" {
            format!("{}_asset_type=native", prefix)
        } else {
            format!(
                "{}_asset_type={}&{}_asset_code={}&{}_asset_issuer={}",
                prefix,
                asset.asset_type,
                prefix,
                asset.asset_code.as_ref().unwrap(),
                prefix,
                asset.asset_issuer.as_ref().unwrap()
            )
        }
    }

    /// Retry a request with exponential backoff
    async fn retry_request<F, Fut>(&self, request_fn: F) -> Result<reqwest::Response>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    {
        let mut attempt = 0;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        loop {
            let start_time = Instant::now();

            match request_fn().await {
                Ok(response) => {
                    let elapsed = start_time.elapsed().as_millis();

                    if response.status().is_success() {
                        debug!("Request succeeded in {} ms", elapsed);
                        return Ok(response);
                    } else {
                        let status = response.status();
                        let error_text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());

                        warn!(
                            "Request failed with status {} in {} ms: {}",
                            status, elapsed, error_text
                        );

                        if attempt >= MAX_RETRIES {
                            anyhow::bail!(
                                "Request failed after {} retries. Status: {}, Error: {}",
                                MAX_RETRIES,
                                status,
                                error_text
                            );
                        }
                    }
                }
                Err(err) => {
                    let elapsed = start_time.elapsed().as_millis();
                    warn!(
                        "Request error after {} ms (attempt {}/{}): {}",
                        elapsed,
                        attempt + 1,
                        MAX_RETRIES + 1,
                        err
                    );

                    if attempt >= MAX_RETRIES {
                        return Err(err)
                            .context(format!("Request failed after {} retries", MAX_RETRIES));
                    }
                }
            }

            attempt += 1;

            info!(
                "Retrying request in {} ms (attempt {}/{})",
                backoff_ms,
                attempt + 1,
                MAX_RETRIES + 1
            );

            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            backoff_ms *= BACKOFF_MULTIPLIER;
        }
    }

    // ============================================================================
    // Mock Data Methods
    // ============================================================================

    fn mock_health_response() -> HealthResponse {
        HealthResponse {
            status: "healthy".to_string(),
            latest_ledger: MOCK_LATEST_LEDGER,
            oldest_ledger: MOCK_OLDEST_LEDGER,
            ledger_retention_window: 60,
        }
    }

    fn mock_ledger_info() -> LedgerInfo {
        LedgerInfo {
            sequence: 51583040,
            hash: "abc123def456".to_string(),
            previous_hash: "xyz789uvw012".to_string(),
            transaction_count: 245,
            operation_count: 1203,
            closed_at: "2026-01-22T10:30:00Z".to_string(),
            total_coins: "105443902087.3472865".to_string(),
            fee_pool: "3145678.9012345".to_string(),
            base_fee: 100,
            base_reserve: "0.5".to_string(),
        }
    }

    // I'm mocking getLedgers response for testing
    fn mock_get_ledgers(start: u64, limit: u32) -> GetLedgersResult {
        if start > MOCK_LATEST_LEDGER {
            return GetLedgersResult {
                ledgers: Vec::new(),
                latest_ledger: MOCK_LATEST_LEDGER,
                oldest_ledger: MOCK_OLDEST_LEDGER,
                cursor: Some(MOCK_LATEST_LEDGER.to_string()),
            };
        }

        let end = (start.saturating_add(limit as u64).saturating_sub(1)).min(MOCK_LATEST_LEDGER);
        let ledgers = (start..=end)
            .enumerate()
            .map(|(i, seq)| RpcLedger {
                hash: format!("hash_{}", seq),
                sequence: seq,
                ledger_close_time: format!("{}", 1734032457 + i as u64 * 5),
                header_xdr: Some("mock_header".to_string()),
                metadata_xdr: Some("mock_metadata".to_string()),
            })
            .collect();

        GetLedgersResult {
            ledgers,
            latest_ledger: MOCK_LATEST_LEDGER,
            oldest_ledger: MOCK_OLDEST_LEDGER,
            cursor: Some(end.to_string()),
        }
    }

    fn mock_payments(limit: u32) -> Vec<Payment> {
        (0..limit)
            .map(|i| {
                let is_path_payment = i % 5 == 0;
                let is_native_source = i % 3 == 0;
                let is_native_dest = i % 4 == 0;

                Payment {
                    id: format!("payment_{}", i),
                    paging_token: format!("paging_{}", i),
                    transaction_hash: format!("txhash_{}", i),
                    source_account: format!(
                        "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX{:03}",
                        i
                    ),
                    destination: format!(
                        "GDYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY{:03}",
                        i
                    ),
                    // For regular payments, asset_type/code/issuer represent the transferred asset
                    // For path payments, they represent the destination asset
                    asset_type: if is_native_dest {
                        "native".to_string()
                    } else if i % 2 == 0 {
                        "credit_alphanum4".to_string()
                    } else {
                        "credit_alphanum12".to_string()
                    },
                    asset_code: if is_native_dest {
                        None
                    } else if i % 2 == 0 {
                        Some(["USDC", "EURT", "BRL", "NGNT"][i as usize % 4].to_string())
                    } else {
                        Some("LONGASSETCODE".to_string())
                    },
                    asset_issuer: if is_native_dest {
                        None
                    } else {
                        Some(format!(
                            "GISSUER{:02}XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
                            i % 10
                        ))
                    },
                    amount: format!("{}.0000000", 100 + i * 10),
                    created_at: format!("2026-01-22T10:{:02}:00Z", i % 60),
                    operation_type: if is_path_payment {
                        Some(if i % 2 == 0 {
                            "path_payment_strict_send".to_string()
                        } else {
                            "path_payment_strict_receive".to_string()
                        })
                    } else {
                        Some("payment".to_string())
                    },
                    // Source asset for path payments
                    source_asset_type: if is_path_payment {
                        Some(if is_native_source {
                            "native".to_string()
                        } else {
                            "credit_alphanum4".to_string()
                        })
                    } else {
                        None
                    },
                    source_asset_code: if is_path_payment && !is_native_source {
                        Some(["USD", "EUR", "GBP", "JPY"][i as usize % 4].to_string())
                    } else {
                        None
                    },
                    source_asset_issuer: if is_path_payment && !is_native_source {
                        Some(format!(
                            "GSRCISSUER{:02}XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
                            i % 10
                        ))
                    } else {
                        None
                    },
                    source_amount: if is_path_payment {
                        Some(format!("{}.0000000", 90 + i * 10))
                    } else {
                        None
                    },
                    from: Some(format!(
                        "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX{:03}",
                        i
                    )),
                    to: Some(format!(
                        "GDYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY{:03}",
                        i
                    )),
                }
            })
            .collect()
    }

    fn mock_trades(limit: u32) -> Vec<Trade> {
        (0..limit)
            .map(|i| Trade {
                id: format!("trade_{}", i),
                ledger_close_time: format!("2026-01-22T10:{:02}:00Z", i % 60),
                base_account: format!("GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX{:03}", i),
                base_amount: format!("{}.0000000", 1000 + i * 100),
                base_asset_type: "native".to_string(),
                base_asset_code: None,
                base_asset_issuer: None,
                counter_account: format!(
                    "GDYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY{:03}",
                    i
                ),
                counter_amount: format!("{}.0000000", 500 + i * 50),
                counter_asset_type: "credit_alphanum4".to_string(),
                counter_asset_code: Some("USDC".to_string()),
                counter_asset_issuer: Some(
                    "GBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
                ),
                price: Price {
                    n: 2 + i as i64,
                    d: 1,
                },
                trade_type: "orderbook".to_string(),
            })
            .collect()
    }

    fn mock_order_book(selling_asset: &Asset, buying_asset: &Asset) -> OrderBook {
        let bids = vec![
            OrderBookEntry {
                price: "0.9950".to_string(),
                amount: "1000.0000000".to_string(),
                price_r: Price { n: 199, d: 200 },
            },
            OrderBookEntry {
                price: "0.9900".to_string(),
                amount: "2500.0000000".to_string(),
                price_r: Price { n: 99, d: 100 },
            },
            OrderBookEntry {
                price: "0.9850".to_string(),
                amount: "5000.0000000".to_string(),
                price_r: Price { n: 197, d: 200 },
            },
        ];

        let asks = vec![
            OrderBookEntry {
                price: "1.0050".to_string(),
                amount: "1200.0000000".to_string(),
                price_r: Price { n: 201, d: 200 },
            },
            OrderBookEntry {
                price: "1.0100".to_string(),
                amount: "3000.0000000".to_string(),
                price_r: Price { n: 101, d: 100 },
            },
            OrderBookEntry {
                price: "1.0150".to_string(),
                amount: "4500.0000000".to_string(),
                price_r: Price { n: 203, d: 200 },
            },
        ];

        OrderBook {
            bids,
            asks,
            base: selling_asset.clone(),
            counter: buying_asset.clone(),
        }
    }

    fn mock_transactions(limit: u32, ledger_sequence: u64) -> Vec<HorizonTransaction> {
        (0..limit)
            .map(|i| {
                let is_fee_bump = i % 2 == 0;
                HorizonTransaction {
                    id: format!("tx_{}", i),
                    hash: format!("txhash_{}", i),
                    ledger: ledger_sequence,
                    created_at: "2026-01-22T10:30:00Z".to_string(),
                    source_account: "GXX".to_string(),
                    fee_account: Some("GXX".to_string()),
                    fee_charged: Some("100".to_string()),
                    max_fee: Some("1000".to_string()),
                    operation_count: 1,
                    successful: true,
                    paging_token: format!("pt_{}", i),
                    fee_bump_transaction: if is_fee_bump {
                        Some(FeeBumpTransactionInfo {
                            hash: format!("fb_hash_{}", i),
                            signatures: vec!["sig1".to_string()],
                        })
                    } else {
                        None
                    },
                    inner_transaction: if is_fee_bump {
                        Some(InnerTransaction {
                            hash: format!("inner_hash_{}", i),
                            max_fee: Some("500".to_string()),
                            signatures: vec!["sig1".to_string()],
                        })
                    } else {
                        None
                    },
                }
            })
            .collect()
    }

    fn mock_operations_for_ledger(sequence: u64) -> Vec<HorizonOperation> {
        let source_a = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string();
        let source_b = "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string();
        let dest_a = "GDESTAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string();
        let dest_b = "GDESTBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string();

        vec![
            HorizonOperation {
                id: format!("op_{}_0", sequence),
                paging_token: format!("pt_{}_0", sequence),
                transaction_hash: format!("txhash_{}_0", sequence),
                source_account: source_a.clone(),
                operation_type: "account_merge".to_string(),
                created_at: "2026-01-22T10:30:00Z".to_string(),
                account: Some(source_a),
                into: Some(dest_a),
                amount: None,
            },
            HorizonOperation {
                id: format!("op_{}_1", sequence),
                paging_token: format!("pt_{}_1", sequence),
                transaction_hash: format!("txhash_{}_1", sequence),
                source_account: "GCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC"
                    .to_string(),
                operation_type: "payment".to_string(),
                created_at: "2026-01-22T10:31:00Z".to_string(),
                account: None,
                into: None,
                amount: Some("25.0000000".to_string()),
            },
            HorizonOperation {
                id: format!("op_{}_2", sequence),
                paging_token: format!("pt_{}_2", sequence),
                transaction_hash: format!("txhash_{}_2", sequence),
                source_account: source_b.clone(),
                operation_type: "account_merge".to_string(),
                created_at: "2026-01-22T10:32:00Z".to_string(),
                account: Some(source_b),
                into: Some(dest_b),
                amount: None,
            },
        ]
    }

    fn mock_effects_for_operation(operation_id: &str) -> Vec<HorizonEffect> {
        if operation_id.ends_with("_0") {
            return vec![HorizonEffect {
                id: format!("effect_{}_0", operation_id),
                effect_type: "account_credited".to_string(),
                account: Some(
                    "GDESTAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
                ),
                amount: Some("125.5000000".to_string()),
                asset_type: Some("native".to_string()),
            }];
        }

        if operation_id.ends_with("_2") {
            return vec![
                HorizonEffect {
                    id: format!("effect_{}_0", operation_id),
                    effect_type: "account_credited".to_string(),
                    account: Some(
                        "GDESTBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string(),
                    ),
                    amount: Some("10.0000000".to_string()),
                    asset_type: Some("native".to_string()),
                },
                HorizonEffect {
                    id: format!("effect_{}_1", operation_id),
                    effect_type: "account_credited".to_string(),
                    account: Some(
                        "GDESTBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string(),
                    ),
                    amount: Some("0.5000000".to_string()),
                    asset_type: Some("native".to_string()),
                },
            ];
        }

        Vec::new()
    }

    // ============================================================================
    // Liquidity Pool Methods
    // ============================================================================

    /// Fetch liquidity pools from Horizon API
    pub async fn fetch_liquidity_pools(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<HorizonLiquidityPool>> {
        if self.mock_mode {
            return Ok(Self::mock_liquidity_pools(limit));
        }

        info!("Fetching {} liquidity pools from Horizon API", limit);

        let mut url = format!(
            "{}/liquidity_pools?order=desc&limit={}",
            self.horizon_url, limit
        );

        if let Some(cursor) = cursor {
            url.push_str(&format!("&cursor={}", cursor));
        }

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch liquidity pools")?;

        let horizon_response: HorizonResponse<HorizonLiquidityPool> = response
            .json()
            .await
            .context("Failed to parse liquidity pools response")?;

        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch a single liquidity pool by ID
    pub async fn fetch_liquidity_pool(&self, pool_id: &str) -> Result<HorizonLiquidityPool> {
        if self.mock_mode {
            let pools = Self::mock_liquidity_pools(1);
            let mut pool = pools.into_iter().next().unwrap();
            pool.id = pool_id.to_string();
            return Ok(pool);
        }

        info!("Fetching liquidity pool {} from Horizon API", pool_id);

        let url = format!("{}/liquidity_pools/{}", self.horizon_url, pool_id);

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch liquidity pool")?;

        let pool: HorizonLiquidityPool = response
            .json()
            .await
            .context("Failed to parse liquidity pool response")?;

        Ok(pool)
    }

    /// Fetch trades for a specific liquidity pool
    pub async fn fetch_pool_trades(&self, pool_id: &str, limit: u32) -> Result<Vec<Trade>> {
        if self.mock_mode {
            return Ok(Self::mock_trades(limit));
        }

        info!(
            "Fetching {} trades for pool {} from Horizon API",
            limit, pool_id
        );

        let url = format!(
            "{}/liquidity_pools/{}/trades?order=desc&limit={}",
            self.horizon_url, pool_id, limit
        );

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch pool trades")?;

        let horizon_response: HorizonResponse<Trade> = response
            .json()
            .await
            .context("Failed to parse pool trades response")?;

        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch assets from Horizon API, sorted by rating
    pub async fn fetch_assets(&self, limit: u32, rating_sort: bool) -> Result<Vec<HorizonAsset>> {
        if self.mock_mode {
            return Ok(Self::mock_assets(limit));
        }

        info!("Fetching {} assets from Horizon API", limit);
        let mut url = format!("{}/assets?limit={}", self.horizon_url, limit);
        if rating_sort {
            url.push_str("&order=desc&sort=rating");
        } else {
            url.push_str("&order=desc");
        }

        let response = self
            .retry_request(|| async { self.client.get(&url).send().await })
            .await
            .context("Failed to fetch assets")?;

        let horizon_response: HorizonResponse<HorizonAsset> = response
            .json()
            .await
            .context("Failed to parse assets response")?;

        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    // ============================================================================
    // Liquidity Pool Mock Data
    // ============================================================================

    fn mock_liquidity_pools(limit: u32) -> Vec<HorizonLiquidityPool> {
        let pool_configs = vec![
            (
                "USDC",
                "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
                "XLM",
                "",
                "500000.0",
                "1200000.0",
                "850000.0",
            ),
            (
                "USDC",
                "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
                "EURC",
                "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y36DAVIZA67CE7BKBHP4V2OA",
                "320000.0",
                "295000.0",
                "610000.0",
            ),
            (
                "XLM",
                "",
                "BTC",
                "GDPJALI4AZKUU2W426U5WKMAT6CN3AJRPIIRYR2YM54TL2GDEMNQERFT",
                "450000.0",
                "12.5",
                "750000.0",
            ),
            (
                "USDC",
                "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
                "yUSDC",
                "GDGTVWSM4MGS2T7Z7GVZE5SAEVLSWM5SGY5Q2EMUQWRMEV2RNYY3YFG6",
                "180000.0",
                "179500.0",
                "360000.0",
            ),
            (
                "XLM",
                "",
                "AQUA",
                "GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
                "800000.0",
                "5000000.0",
                "420000.0",
            ),
        ];

        pool_configs
            .iter()
            .take(limit as usize)
            .enumerate()
            .map(
                |(i, (code_a, issuer_a, code_b, issuer_b, amt_a, amt_b, shares))| {
                    let asset_a = if issuer_a.is_empty() {
                        "native".to_string()
                    } else {
                        format!("{}:{}", code_a, issuer_a)
                    };
                    let asset_b = if issuer_b.is_empty() {
                        "native".to_string()
                    } else {
                        format!("{}:{}", code_b, issuer_b)
                    };

                    HorizonLiquidityPool {
                        id: format!("pool_{:064x}", i + 1),
                        fee_bp: 30,
                        pool_type: "constant_product".to_string(),
                        total_trustlines: 100 + (i as u64 * 50),
                        total_shares: shares.to_string(),
                        reserves: vec![
                            HorizonPoolReserve {
                                asset: asset_a,
                                amount: amt_a.to_string(),
                            },
                            HorizonPoolReserve {
                                asset: asset_b,
                                amount: amt_b.to_string(),
                            },
                        ],
                        paging_token: Some(format!("pt_pool_{}", i)),
                    }
                },
            )
            .collect()
    }

    fn mock_assets(limit: u32) -> Vec<HorizonAsset> {
        let mut assets = Vec::new();
        let issues = vec![
            (
                "USDC",
                "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            ),
            (
                "AQUA",
                "GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
            ),
            (
                "yXLM",
                "GARDNV3Q7YGT4AKSDF25A9NTVAMQUD8UAKGHXONL6R2FMBXVGFZDFZEM",
            ),
            (
                "BTC",
                "GDPJALI4AZKUU2W426U5WKMAT6CN3AJRPIIRYR2YM54TL2GDEMNQERFT",
            ),
        ];

        for (i, (code, issuer)) in issues.iter().take(limit as usize).enumerate() {
            let base_trustlines = 10000 - (i as i32 * 2000);
            assets.push(HorizonAsset {
                asset_type: "credit_alphanum4".to_string(),
                asset_code: code.to_string(),
                asset_issuer: issuer.to_string(),
                num_claimable_balances: 0,
                num_liquidity_pools: 0,
                num_contracts: 0,
                accounts: AssetAccounts {
                    authorized: base_trustlines,
                    authorized_to_maintain_liabilities: 0,
                    unauthorized: base_trustlines / 20,
                },
                claimable_balances_amount: "0.0".to_string(),
                liquidity_pools_amount: "0.0".to_string(),
                contracts_amount: "0.0".to_string(),
                balances: AssetBalances {
                    authorized: format!("{}.0000000", base_trustlines * 1000),
                    authorized_to_maintain_liabilities: "0.0".to_string(),
                    unauthorized: "0.0".to_string(),
                },
                flags: AssetFlags {
                    auth_required: false,
                    auth_revocable: false,
                    auth_immutable: false,
                    auth_clawback_enabled: false,
                },
            })
        }
        assets
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
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
            .fetch_ledgers(Some(MOCK_LATEST_LEDGER.saturating_add(1)), 5, None)
            .await
            .unwrap();

        assert!(result.ledgers.is_empty());
        assert_eq!(result.latest_ledger, MOCK_LATEST_LEDGER);
    }

    #[tokio::test]
    async fn test_pagination_config_defaults() {
        let client = StellarRpcClient::new_with_defaults(true);

        // Verify default pagination config is loaded
        assert_eq!(client.max_records_per_request, 200);
        assert_eq!(client.max_total_records, 10000);
        assert_eq!(client.pagination_delay_ms, 100);
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
}
