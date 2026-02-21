use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tracing::info;

use crate::rpc::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::rpc::error::RpcError;
use crate::rpc::metrics;
use crate::rpc::retry;

/// Retry and circuit breaker configuration for the RPC client.
#[derive(Clone)]
pub struct RpcClientConfig {
    pub max_retries: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub circuit_breaker: CircuitBreakerConfig,
}

impl Default for RpcClientConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(5),
            circuit_breaker: CircuitBreakerConfig::default(),
        }
    }
}

impl RpcClientConfig {
    /// Load configuration from environment variables with defaults.
    pub fn from_env() -> Self {
        let max_retries = std::env::var("RPC_MAX_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);
        let initial_backoff = std::env::var("RPC_INITIAL_BACKOFF_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(100));
        let max_backoff = std::env::var("RPC_MAX_BACKOFF_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(5));
        let failure_threshold = std::env::var("RPC_CIRCUIT_BREAKER_FAILURE_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);
        let success_threshold = std::env::var("RPC_CIRCUIT_BREAKER_SUCCESS_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2);
        let timeout_secs = std::env::var("RPC_CIRCUIT_BREAKER_TIMEOUT_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        Self {
            max_retries,
            initial_backoff,
            max_backoff,
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold,
                success_threshold,
                timeout_duration: Duration::from_secs(timeout_secs),
                half_open_max_calls: 3,
            },
        }
    }
}

const RPC_ENDPOINT: &str = "stellar";

/// Stellar RPC Client for interacting with Stellar network via RPC and Horizon API
#[derive(Clone)]
pub struct StellarRpcClient {
    client: Client,
    rpc_url: String,
    horizon_url: String,
    mock_mode: bool,
    circuit_breaker: CircuitBreaker,
    config: RpcClientConfig,
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
    pub destination: String,
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    pub amount: String,
    pub created_at: String,
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
        Self::new_with_config(rpc_url, horizon_url, mock_mode, RpcClientConfig::from_env())
    }

    /// Create a new client with explicit config (e.g. for tests).
    pub fn new_with_config(
        rpc_url: String,
        horizon_url: String,
        mock_mode: bool,
        config: RpcClientConfig,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        let circuit_breaker =
            CircuitBreaker::new(config.circuit_breaker.clone(), RPC_ENDPOINT.to_string());

        Self {
            client,
            rpc_url,
            horizon_url,
            mock_mode,
            circuit_breaker,
            config,
        }
    }

    /// Create a new client with default OnFinality RPC and Horizon URLs
    pub fn new_with_defaults(mock_mode: bool) -> Self {
        Self::new(
            "https://stellar.api.onfinality.io/public".to_string(),
            "https://horizon.stellar.org".to_string(),
            mock_mode,
        )
    }

    /// Convert a failed HTTP response or reqwest error into RpcError.
    fn response_to_error(
        status: reqwest::StatusCode,
        body: String,
    ) -> RpcError {
        let status_code = status.as_u16();
        if status_code == 429 {
            let retry_after = None; // Could parse Retry-After header if present
            return RpcError::RateLimitError {
                retry_after,
            };
        }
        if (500..=599).contains(&status_code) {
            return RpcError::ServerError {
                status: status_code,
                message: body,
            };
        }
        RpcError::ServerError {
            status: status_code,
            message: body,
        }
    }

    /// Execute one HTTP GET and return response or RpcError.
    async fn get_once(&self, url: &str) -> Result<reqwest::Response, RpcError> {
        let response = self.client.get(url).send().await.map_err(|e| {
            if e.is_timeout() {
                RpcError::TimeoutError(Duration::from_secs(30))
            } else {
                RpcError::NetworkError(e)
            }
        })?;
        if response.status().is_success() {
            Ok(response)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(Self::response_to_error(status, body))
        }
    }

    /// Execute one HTTP POST and return response or RpcError.
    async fn post_once(&self, url: &str, payload: &serde_json::Value) -> Result<reqwest::Response, RpcError> {
        let response = self.client.post(url).json(payload).send().await.map_err(|e| {
            if e.is_timeout() {
                RpcError::TimeoutError(Duration::from_secs(30))
            } else {
                RpcError::NetworkError(e)
            }
        })?;
        if response.status().is_success() {
            Ok(response)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(Self::response_to_error(status, body))
        }
    }

    /// Check the health of the RPC endpoint
    pub async fn check_health(&self) -> Result<HealthResponse, RpcError> {
        if self.mock_mode {
            return Ok(Self::mock_health_response());
        }
        info!("Checking RPC health at {}", self.rpc_url);
        let result = self
            .circuit_breaker
            .call(async {
                let client = self;
                retry::retry_with_backoff(
                    || Box::pin(client.check_health_internal()),
                    client.config.max_retries,
                    client.config.initial_backoff,
                    client.config.max_backoff,
                )
                .await
            })
            .await;
        if let Err(ref e) = result {
            metrics::record_rpc_error(RPC_ENDPOINT, e);
            tracing::error!(error_type = %e.error_type(), "RPC health check failed: {}", e);
        }
        result
    }

    async fn check_health_internal(&self) -> Result<HealthResponse, RpcError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "getHealth",
            "id": 1
        });
        let resp = self.post_once(&self.rpc_url, &payload).await?;
        let json_response: JsonRpcResponse<HealthResponse> = resp
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        if let Some(error) = json_response.error {
            return Err(RpcError::JsonRpcError {
                code: error.code,
                message: error.message,
            });
        }
        json_response
            .result
            .ok_or_else(|| RpcError::ParseError("No result in health response".to_string()))
    }

    /// Run an operation through circuit breaker and retry; record metrics on error.
    async fn with_circuit_and_retry<F, Fut, T>(&self, mut f: F) -> Result<T, RpcError>
    where
        F: FnMut() -> std::pin::Pin<Box<Fut>>,
        Fut: std::future::Future<Output = Result<T, RpcError>>,
    {
        let result = self
            .circuit_breaker
            .call(async move {
                let client = self;
                retry::retry_with_backoff(
                    || f(),
                    client.config.max_retries,
                    client.config.initial_backoff,
                    client.config.max_backoff,
                )
                .await
            })
            .await;
        if let Err(ref e) = result {
            metrics::record_rpc_error(RPC_ENDPOINT, e);
            tracing::error!(error_type = %e.error_type(), "RPC request failed: {}", e);
        }
        result
    }

    /// Fetch latest ledger information
    pub async fn fetch_latest_ledger(&self) -> Result<LedgerInfo, RpcError> {
        if self.mock_mode {
            return Ok(Self::mock_ledger_info());
        }
        info!("Fetching latest ledger from Horizon API");
        self.with_circuit_and_retry(|| Box::pin(self.fetch_latest_ledger_internal()))
            .await
    }

    async fn fetch_latest_ledger_internal(&self) -> Result<LedgerInfo, RpcError> {
        let url = format!("{}/ledgers?order=desc&limit=1", self.horizon_url);
        let resp = self.get_once(&url).await?;
        let horizon_response: HorizonResponse<LedgerInfo> = resp
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        horizon_response
            .embedded
            .and_then(|e| e.records.into_iter().next())
            .ok_or_else(|| RpcError::ParseError("No ledger data found".to_string()))
    }

    /// Fetch ledgers via RPC getLedgers for sequential ingestion
    pub async fn fetch_ledgers(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError> {
        if self.mock_mode {
            return Ok(Self::mock_get_ledgers(start_ledger.unwrap_or(1000), limit));
        }
        info!("Fetching ledgers via RPC getLedgers");
        let start_ledger = start_ledger;
        let cursor_owned = cursor.map(|s| s.to_string());
        self.with_circuit_and_retry(|| {
            Box::pin(self.fetch_ledgers_internal(start_ledger, limit, cursor_owned.as_deref()))
        })
        .await
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
        let resp = self.post_once(&self.rpc_url, &payload).await?;
        let json_response: JsonRpcResponse<GetLedgersResult> = resp
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        if let Some(error) = json_response.error {
            return Err(RpcError::JsonRpcError {
                code: error.code,
                message: error.message,
            });
        }
        json_response
            .result
            .ok_or_else(|| RpcError::ParseError("No result in getLedgers response".to_string()))
    }

    /// Fetch recent payments
    pub async fn fetch_payments(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Payment>, RpcError> {
        if self.mock_mode {
            return Ok(Self::mock_payments(limit));
        }
        info!("Fetching {} payments from Horizon API", limit);
        let cursor_owned = cursor.map(|s| s.to_string());
        self.with_circuit_and_retry(|| {
            Box::pin(self.fetch_payments_internal(limit, cursor_owned.as_deref()))
        })
        .await
    }

    async fn fetch_payments_internal(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<Payment>, RpcError> {
        let mut url = format!("{}/payments?order=desc&limit={}", self.horizon_url, limit);
        if let Some(c) = cursor {
            url.push_str(&format!("&cursor={}", c));
        }
        let resp = self.get_once(&url).await?;
        let horizon_response: HorizonResponse<Payment> = resp
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch recent trades
    pub async fn fetch_trades(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Trade>, RpcError> {
        if self.mock_mode {
            return Ok(Self::mock_trades(limit));
        }
        info!("Fetching {} trades from Horizon API", limit);
        let cursor_owned = cursor.map(|s| s.to_string());
        self.with_circuit_and_retry(|| {
            Box::pin(self.fetch_trades_internal(limit, cursor_owned.as_deref()))
        })
        .await
    }

    async fn fetch_trades_internal(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<Trade>, RpcError> {
        let mut url = format!("{}/trades?order=desc&limit={}", self.horizon_url, limit);
        if let Some(c) = cursor {
            url.push_str(&format!("&cursor={}", c));
        }
        let resp = self.get_once(&url).await?;
        let horizon_response: HorizonResponse<Trade> = resp
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
            return Ok(Self::mock_order_book(selling_asset, buying_asset));
        }
        info!("Fetching order book from Horizon API");
        let selling_asset = selling_asset.clone();
        let buying_asset = buying_asset.clone();
        self.with_circuit_and_retry(|| {
            Box::pin(self.fetch_order_book_internal(&selling_asset, &buying_asset, limit))
        })
        .await
    }

    async fn fetch_order_book_internal(
        &self,
        selling_asset: &Asset,
        buying_asset: &Asset,
        limit: u32,
    ) -> Result<OrderBook, RpcError> {
        let selling_params = Self::asset_to_query_params("selling", selling_asset);
        let buying_params = Self::asset_to_query_params("buying", buying_asset);
        let url = format!(
            "{}/order_book?{}&{}&limit={}",
            self.horizon_url, selling_params, buying_params, limit
        );
        let resp = self.get_once(&url).await?;
        resp.json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))
    }

    pub async fn fetch_payments_for_ledger(&self, sequence: u64) -> Result<Vec<Payment>, RpcError> {
        if self.mock_mode {
            return Ok(Self::mock_payments(5));
        }
        self.with_circuit_and_retry(|| Box::pin(self.fetch_payments_for_ledger_internal(sequence)))
            .await
    }

    async fn fetch_payments_for_ledger_internal(&self, sequence: u64) -> Result<Vec<Payment>, RpcError> {
        let url = format!("{}/ledgers/{}/payments?limit=200", self.horizon_url, sequence);
        let resp = self.get_once(&url).await?;
        let horizon_response: HorizonResponse<Payment> = resp
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch transactions for a specific ledger
    pub async fn fetch_transactions_for_ledger(&self, sequence: u64) -> Result<Vec<HorizonTransaction>, RpcError> {
        if self.mock_mode {
            return Ok(Self::mock_transactions(5));
        }
        self.with_circuit_and_retry(|| {
            Box::pin(self.fetch_transactions_for_ledger_internal(sequence))
        })
        .await
    }

    async fn fetch_transactions_for_ledger_internal(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError> {
        let url = format!(
            "{}/ledgers/{}/transactions?limit=200&include_failed=true",
            self.horizon_url, sequence
        );
        let resp = self.get_once(&url).await?;
        let horizon_response: HorizonResponse<HorizonTransaction> = resp
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
            return Ok(Self::mock_payments(limit));
        }
        info!(
            "Fetching {} payments for account {} from Horizon API",
            limit, account_id
        );
        let account_id = account_id.to_string();
        self.with_circuit_and_retry(|| {
            Box::pin(self.fetch_account_payments_internal(&account_id, limit))
        })
        .await
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
        let resp = self.get_once(&url).await?;
        let horizon_response: HorizonResponse<Payment> = resp
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
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

    // ============================================================================
    // Mock Data Methods
    // ============================================================================

    fn mock_health_response() -> HealthResponse {
        HealthResponse {
            status: "healthy".to_string(),
            latest_ledger: 51583040,
            oldest_ledger: 51565760,
            ledger_retention_window: 17280,
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
        let ledgers = (0..limit)
            .map(|i| RpcLedger {
                hash: format!("hash_{}", start + i as u64),
                sequence: start + i as u64,
                ledger_close_time: format!("{}", 1734032457 + i as u64 * 5),
                header_xdr: Some("mock_header".to_string()),
                metadata_xdr: Some("mock_metadata".to_string()),
            })
            .collect();
        GetLedgersResult {
            ledgers,
            latest_ledger: start + limit as u64 + 100,
            oldest_ledger: start.saturating_sub(1000),
            cursor: Some(format!("{}", start + limit as u64 - 1)),
        }
    }

    fn mock_payments(limit: u32) -> Vec<Payment> {
        (0..limit)
            .map(|i| Payment {
                id: format!("payment_{}", i),
                paging_token: format!("paging_{}", i),
                transaction_hash: format!("txhash_{}", i),
                source_account: format!(
                    "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX{:03}",
                    i
                ),
                destination: format!("GDYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY{:03}", i),
                asset_type: if i % 3 == 0 {
                    "native".to_string()
                } else {
                    "credit_alphanum4".to_string()
                },
                asset_code: if i % 3 == 0 {
                    None
                } else {
                    Some("USDC".to_string())
                },
                asset_issuer: if i % 3 == 0 {
                    None
                } else {
                    Some("GBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string())
                },
                amount: format!("{}.0000000", 100 + i * 10),
                created_at: format!("2026-01-22T10:{:02}:00Z", i % 60),
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

    fn mock_transactions(limit: u32) -> Vec<HorizonTransaction> {
        (0..limit)
            .map(|i| {
                let is_fee_bump = i % 2 == 0;
                HorizonTransaction {
                    id: format!("tx_{}", i),
                    hash: format!("txhash_{}", i),
                    ledger: 51583040,
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
}
