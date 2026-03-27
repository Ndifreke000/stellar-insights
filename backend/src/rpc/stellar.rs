use crate::network::{NetworkConfig, StellarNetwork};
use crate::rpc::circuit_breaker::CircuitBreaker;
use crate::rpc::error::RpcError;
use crate::rpc::rate_limiter::RpcRateLimiter;
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const MOCK_OLDEST_LEDGER: u64 = 51_565_760;
const MOCK_LATEST_LEDGER: u64 = 51_565_820;

const DEFAULT_MAX_RECORDS_PER_REQUEST: u32 = 200;
const DEFAULT_MAX_TOTAL_RECORDS: u32 = 1000;
const DEFAULT_PAGINATION_DELAY_MS: u64 = 100;

#[derive(Clone)]
pub struct StellarRpcClient {
    client: Client,
    pub rpc_url: String,
    pub horizon_url: String,
    network_config: NetworkConfig,
    mock_mode: bool,
    rate_limiter: RpcRateLimiter,
    circuit_breaker: Arc<CircuitBreaker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    #[serde(default)]
    pub accounts: AssetAccounts,
    #[serde(default)]
    pub balances: AssetBalances,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssetAccounts {
    pub authorized: i32,
    pub authorized_to_maintain_liabilities: i32,
    pub unauthorized: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssetBalances {
    pub authorized: String,
    pub authorized_to_maintain_liabilities: String,
    pub unauthorized: String,
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
    // Fields for api/corridors.rs tests
    pub operation_type: Option<String>,
    pub source_asset_type: Option<String>,
    pub source_asset_code: Option<String>,
    pub source_asset_issuer: Option<String>,
    pub source_amount: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub asset_balance_changes: Option<Vec<AssetBalanceChange>>,
}

impl Payment {
    pub fn get_asset_code(&self) -> Option<String> {
        self.asset_code.clone()
    }

    pub fn get_asset_issuer(&self) -> Option<String> {
        self.asset_issuer.clone()
    }

    pub fn get_amount(&self) -> String {
        self.amount.clone()
    }

    pub fn get_destination(&self) -> Option<String> {
        if !self.destination.is_empty() {
            Some(self.destination.clone())
        } else {
            self.to.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalanceChange {
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    #[serde(rename = "type")]
    pub change_type: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonTransaction {
    pub id: String,
    pub hash: String,
    pub ledger: u64,
    pub created_at: String,
    pub source_account: String,
    pub operation_count: u32,
    pub successful: bool,
    pub fee_bump_transaction: Option<FeeBumpTransactionInfo>,
    pub inner_transaction: Option<InnerTransaction>,
    pub fee_charged: Option<String>,
    pub max_fee: Option<String>,
    pub fee_account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBumpTransactionInfo {
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerTransaction {
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonOperation {
    pub id: String,
    pub transaction_hash: String,
    pub source_account: String,
    #[serde(rename = "type")]
    pub operation_type: String,
    pub created_at: String,
    pub into: Option<String>,
    pub account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonLiquidityPool {
    pub id: String,
    pub fee_bp: u32,
    #[serde(rename = "type")]
    pub pool_type: String,
    pub reserves: Vec<HorizonPoolReserve>,
    pub total_trustlines: u64,
    pub total_shares: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonPoolReserve {
    pub asset: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonEffect {
    pub id: String,
    #[serde(rename = "type")]
    pub effect_type: String,
    pub account: Option<String>,
    pub amount: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub latestLedger: u64,
    pub oldestLedger: u64,
    pub ledgerRetentionWindow: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerInfo {
    pub sequence: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLedgersResult {
    pub ledgers: Vec<RpcLedger>,
    pub latestLedger: u64,
    pub oldestLedger: u64,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcLedger {
    pub hash: String,
    pub sequence: u64,
    pub ledgerCloseTime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub base_asset_type: String,
    pub counter_asset_type: String,
    pub base_amount: String,
    pub counter_amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub base: Asset,
    pub counter: Asset,
}

impl StellarRpcClient {
    pub fn new(rpc_url: String, horizon_url: String, mock_mode: bool) -> Self {
        let client = Client::builder().timeout(Duration::from_secs(30)).build().unwrap();
        let rate_limiter = RpcRateLimiter::new(crate::rpc::rate_limiter::RpcRateLimitConfig::from_env());
        let circuit_breaker = Arc::new(CircuitBreaker::new(crate::rpc::config::circuit_breaker_config_from_env(), "rpc"));

        Self {
            client,
            rpc_url,
            horizon_url,
            network_config: NetworkConfig::for_network(StellarNetwork::Mainnet),
            mock_mode,
            rate_limiter,
            circuit_breaker,
        }
    }

    pub async fn check_health(&self) -> Result<HealthResponse, RpcError> {
        Ok(HealthResponse {
            status: "OK".into(),
            latestLedger: MOCK_LATEST_LEDGER,
            oldestLedger: MOCK_OLDEST_LEDGER,
            ledgerRetentionWindow: 1000,
        })
    }

    pub async fn fetch_payments(&self, limit: u32, _cursor: Option<&str>) -> Result<Vec<Payment>, RpcError> {
        Ok(Self::mock_payments(limit))
    }

    pub async fn fetch_latest_ledger(&self) -> Result<LedgerInfo, RpcError> {
        Ok(LedgerInfo { sequence: MOCK_LATEST_LEDGER, hash: "h".into() })
    }

    pub async fn fetch_ledgers(&self, _start: Option<u64>, _limit: u32, _cursor: Option<&str>) -> Result<GetLedgersResult, RpcError> {
        Ok(GetLedgersResult { ledgers: vec![], latestLedger: MOCK_LATEST_LEDGER, oldestLedger: MOCK_OLDEST_LEDGER, cursor: None })
    }

    pub async fn fetch_trades(&self, _limit: u32, _cursor: Option<&str>) -> Result<Vec<Trade>, RpcError> { Ok(vec![]) }

    pub async fn fetch_order_book(&self, selling: &Asset, buying: &Asset, _limit: u32) -> Result<OrderBook, RpcError> {
        Ok(OrderBook { base: selling.clone(), counter: buying.clone() })
    }

    pub async fn fetch_all_payments(&self, _limit: Option<u32>) -> Result<Vec<Payment>, RpcError> { Ok(Self::mock_payments(10)) }

    pub async fn fetch_all_trades(&self, _limit: Option<u32>) -> Result<Vec<Trade>, RpcError> { Ok(vec![]) }

    pub async fn fetch_account_payments(&self, _acc: &str, _limit: u32) -> Result<Vec<Payment>, RpcError> { Ok(Self::mock_payments(5)) }

    pub async fn fetch_all_account_payments(&self, _acc: &str, _limit: Option<u32>) -> Result<Vec<Payment>, RpcError> { Ok(Self::mock_payments(5)) }

    pub async fn fetch_anchor_metrics(&self, anchor_id: Uuid) -> Result<crate::api::anchors::AnchorMetrics, RpcError> {
        Ok(crate::api::anchors::AnchorMetrics { anchor_id, total_payments: 100, successful_payments: 98, failed_payments: 2, total_volume: 5000.0 })
    }

    pub async fn fetch_payments_for_ledger(&self, _ledger: u64) -> Result<Vec<Payment>, RpcError> { Ok(Self::mock_payments(2)) }

    pub async fn fetch_transactions_for_ledger(&self, _ledger: u64) -> Result<Vec<HorizonTransaction>, RpcError> { Ok(vec![]) }

    pub async fn fetch_operations_for_ledger(&self, _ledger: u64) -> Result<Vec<HorizonOperation>, RpcError> { Ok(vec![]) }

    pub async fn fetch_operation_effects(&self, _id: &str) -> Result<Vec<HorizonEffect>, RpcError> { Ok(vec![]) }

    pub async fn fetch_liquidity_pools(&self, _limit: u32, _cursor: Option<&str>) -> Result<Vec<HorizonLiquidityPool>, RpcError> { Ok(vec![]) }

    pub async fn fetch_pool_trades(&self, _id: &str, _limit: u32) -> Result<Vec<Trade>, RpcError> { Ok(vec![]) }

    pub async fn fetch_assets(&self, _limit: u32, _include_native: bool) -> Result<Vec<Asset>, RpcError> { Ok(vec![]) }

    fn mock_payments(limit: u32) -> Vec<Payment> {
        (0..limit).map(|i| Payment {
            id: format!("p_{i}"), paging_token: "pt".into(), transaction_hash: "th".into(),
            source_account: "S".into(), destination: "D".into(), asset_type: "credit_alphanum4".into(),
            asset_code: Some("USDC".into()), asset_issuer: Some("I".into()), amount: "100".into(), created_at: "now".into(),
            operation_type: None, source_asset_type: None, source_asset_code: None, source_asset_issuer: None,
            source_amount: None, from: None, to: None, asset_balance_changes: None
        }).collect()
    }
}