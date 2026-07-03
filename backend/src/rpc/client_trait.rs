//! RPC client trait abstraction for testability
//!
//! This module provides a trait-based abstraction for the Stellar RPC client,
//! allowing easy swapping between real and mock implementations for testing.

use crate::network::NetworkConfig;
use crate::rpc::stellar::{
    GetLedgersResult, HealthResponse, HorizonOperation, HorizonTransaction, LedgerInfo, OrderBook,
    Payment, RpcLedger, StellarRpcClient, Trade,
};

use super::error::RpcError;

/// Trait defining the Stellar RPC client interface
///
/// This trait abstracts over the real RPC client and mock implementations,
/// enabling dependency injection for testing.
#[async_trait::async_trait]
pub trait StellarRpcClientTrait: Send + Sync {
    /// Get the network configuration
    fn network_config(&self) -> &NetworkConfig;

    /// Check the health of the RPC endpoint
    async fn check_health(&self) -> Result<HealthResponse, RpcError>;

    /// Fetch latest ledger information
    async fn fetch_latest_ledger(&self) -> Result<LedgerInfo, RpcError>;

    /// Fetch ledger by sequence number
    async fn fetch_ledger_by_sequence(&self, sequence: u64) -> Result<LedgerInfo, RpcError>;

    /// Fetch multiple ledgers with pagination
    async fn fetch_ledgers(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError>;

    /// Fetch transactions for a specific ledger
    async fn fetch_transactions_for_ledger(
        &self,
        ledger_sequence: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError>;

    /// Fetch payments for a specific ledger
    async fn fetch_payments_for_ledger(
        &self,
        ledger_sequence: u64,
    ) -> Result<Vec<Payment>, RpcError>;

    /// Fetch payments for an account
    async fn fetch_account_payments(
        &self,
        account_id: &str,
        limit: u32,
    ) -> Result<Vec<Payment>, RpcError>;

    /// Fetch all account payments (with optional limit)
    async fn fetch_all_account_payments(
        &self,
        account_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Payment>, RpcError>;

    /// Fetch operations for a ledger
    async fn fetch_operations_for_ledger(
        &self,
        ledger_sequence: u64,
    ) -> Result<Vec<HorizonOperation>, RpcError>;

    /// Fetch trades from the order book
    async fn fetch_trades(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Trade>, RpcError>;

    /// Fetch order book for a trading pair
    async fn fetch_order_book(
        &self,
        selling_asset: &crate::rpc::stellar::Asset,
        buying_asset: &crate::rpc::stellar::Asset,
        limit: u32,
    ) -> Result<OrderBook, RpcError>;

    /// Fetch liquidity pools
    async fn fetch_liquidity_pools(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<crate::rpc::stellar::HorizonLiquidityPool>, RpcError>;

    /// Fetch pool trades
    async fn fetch_pool_trades(&self, pool_id: &str, limit: u32) -> Result<Vec<Trade>, RpcError>;

    /// Fetch assets
    async fn fetch_assets(
        &self,
        limit: u32,
        sponsored: bool,
    ) -> Result<Vec<crate::rpc::stellar::HorizonAsset>, RpcError>;
}

/// Wrapper that implements the trait for the existing StellarRpcClient
#[async_trait::async_trait]
impl StellarRpcClientTrait for StellarRpcClient {
    fn network_config(&self) -> &NetworkConfig {
        StellarRpcClient::network_config(self)
    }

    async fn check_health(&self) -> Result<HealthResponse, RpcError> {
        StellarRpcClient::check_health(self).await
    }

    async fn fetch_latest_ledger(&self) -> Result<LedgerInfo, RpcError> {
        StellarRpcClient::fetch_latest_ledger(self).await
    }

    async fn fetch_ledger_by_sequence(&self, sequence: u64) -> Result<LedgerInfo, RpcError> {
        StellarRpcClient::fetch_ledger_by_sequence(self, sequence).await
    }

    async fn fetch_ledgers(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError> {
        StellarRpcClient::fetch_ledgers(self, start_ledger, limit, cursor).await
    }

    async fn fetch_transactions_for_ledger(
        &self,
        ledger_sequence: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError> {
        StellarRpcClient::fetch_transactions_for_ledger(self, ledger_sequence).await
    }

    async fn fetch_payments_for_ledger(
        &self,
        ledger_sequence: u64,
    ) -> Result<Vec<Payment>, RpcError> {
        StellarRpcClient::fetch_payments_for_ledger(self, ledger_sequence).await
    }

    async fn fetch_account_payments(
        &self,
        account_id: &str,
        limit: u32,
    ) -> Result<Vec<Payment>, RpcError> {
        StellarRpcClient::fetch_account_payments(self, account_id, limit).await
    }

    async fn fetch_all_account_payments(
        &self,
        account_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Payment>, RpcError> {
        StellarRpcClient::fetch_all_account_payments(self, account_id, limit)
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))
    }

    async fn fetch_operations_for_ledger(
        &self,
        ledger_sequence: u64,
    ) -> Result<Vec<HorizonOperation>, RpcError> {
        StellarRpcClient::fetch_operations_for_ledger(self, ledger_sequence).await
    }

    async fn fetch_trades(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Trade>, RpcError> {
        StellarRpcClient::fetch_trades(self, limit, cursor).await
    }

    async fn fetch_order_book(
        &self,
        selling_asset: &crate::rpc::stellar::Asset,
        buying_asset: &crate::rpc::stellar::Asset,
        limit: u32,
    ) -> Result<OrderBook, RpcError> {
        StellarRpcClient::fetch_order_book(self, selling_asset, buying_asset, limit).await
    }

    async fn fetch_liquidity_pools(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<crate::rpc::stellar::HorizonLiquidityPool>, RpcError> {
        StellarRpcClient::fetch_liquidity_pools(self, limit, cursor).await
    }

    async fn fetch_pool_trades(&self, pool_id: &str, limit: u32) -> Result<Vec<Trade>, RpcError> {
        StellarRpcClient::fetch_pool_trades(self, pool_id, limit).await
    }

    async fn fetch_assets(
        &self,
        limit: u32,
        sponsored: bool,
    ) -> Result<Vec<crate::rpc::stellar::HorizonAsset>, RpcError> {
        StellarRpcClient::fetch_assets(self, limit, sponsored).await
    }
}

/// Mock RPC client for testing
pub struct MockStellarRpcClient {
    network_config: NetworkConfig,
}

impl MockStellarRpcClient {
    /// Create a new mock RPC client
    pub fn new(network_config: NetworkConfig) -> Self {
        Self { network_config }
    }

    /// Create a new mock RPC client for testnet
    pub fn testnet() -> Self {
        Self {
            network_config: NetworkConfig::for_network(crate::network::StellarNetwork::Testnet),
        }
    }

    /// Create a new mock RPC client for mainnet
    pub fn mainnet() -> Self {
        Self {
            network_config: NetworkConfig::for_network(crate::network::StellarNetwork::Mainnet),
        }
    }
}

#[async_trait::async_trait]
impl StellarRpcClientTrait for MockStellarRpcClient {
    fn network_config(&self) -> &NetworkConfig {
        &self.network_config
    }

    async fn check_health(&self) -> Result<HealthResponse, RpcError> {
        Ok(HealthResponse {
            status: "healthy".to_string(),
            latest_ledger: 1000,
            oldest_ledger: 1,
            ledger_retention_window: 1000,
        })
    }

    async fn fetch_latest_ledger(&self) -> Result<LedgerInfo, RpcError> {
        Ok(LedgerInfo {
            sequence: 1000,
            hash: "mock-ledger-hash".to_string(),
            previous_hash: "mock-prev".to_string(),
            transaction_count: 0,
            operation_count: 0,
            closed_at: "2024-01-01T00:00:00Z".to_string(),
            total_coins: "0".to_string(),
            fee_pool: "0".to_string(),
            base_fee: 100,
            base_reserve: "0".to_string(),
            protocol_version: 21,
        })
    }

    async fn fetch_ledger_by_sequence(&self, sequence: u64) -> Result<LedgerInfo, RpcError> {
        Ok(LedgerInfo {
            sequence,
            hash: format!("mock-ledger-hash-{sequence}"),
            previous_hash: "mock-prev".to_string(),
            transaction_count: 0,
            operation_count: 0,
            closed_at: "2024-01-01T00:00:00Z".to_string(),
            total_coins: "0".to_string(),
            fee_pool: "0".to_string(),
            base_fee: 100,
            base_reserve: "0".to_string(),
            protocol_version: 21,
        })
    }

    async fn fetch_ledgers(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        _cursor: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError> {
        let start = start_ledger.unwrap_or(1000);
        let ledgers: Vec<RpcLedger> = (0..limit)
            .map(|i| {
                let seq = start + u64::from(i);
                RpcLedger {
                    hash: format!("mock-ledger-hash-{seq}"),
                    sequence: seq,
                    ledger_close_time: "2024-01-01T00:00:00Z".to_string(),
                    header_xdr: None,
                    metadata_xdr: None,
                }
            })
            .collect();
        let latest = start.saturating_add(u64::from(limit).saturating_sub(1));
        Ok(GetLedgersResult {
            ledgers,
            latest_ledger: latest,
            oldest_ledger: start.saturating_sub(100),
            cursor: None,
        })
    }

    async fn fetch_transactions_for_ledger(
        &self,
        _ledger_sequence: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError> {
        Ok(vec![])
    }

    async fn fetch_payments_for_ledger(
        &self,
        _ledger_sequence: u64,
    ) -> Result<Vec<Payment>, RpcError> {
        Ok(vec![])
    }

    async fn fetch_account_payments(
        &self,
        _account_id: &str,
        _limit: u32,
    ) -> Result<Vec<Payment>, RpcError> {
        Ok(vec![])
    }

    async fn fetch_all_account_payments(
        &self,
        _account_id: &str,
        _limit: Option<u32>,
    ) -> Result<Vec<Payment>, RpcError> {
        Ok(vec![])
    }

    async fn fetch_operations_for_ledger(
        &self,
        _ledger_sequence: u64,
    ) -> Result<Vec<HorizonOperation>, RpcError> {
        Ok(vec![])
    }

    async fn fetch_trades(
        &self,
        _limit: u32,
        _cursor: Option<&str>,
    ) -> Result<Vec<Trade>, RpcError> {
        Ok(vec![])
    }

    async fn fetch_order_book(
        &self,
        _selling_asset: &crate::rpc::stellar::Asset,
        _buying_asset: &crate::rpc::stellar::Asset,
        _limit: u32,
    ) -> Result<OrderBook, RpcError> {
        Ok(OrderBook {
            bids: vec![],
            asks: vec![],
            base: crate::rpc::stellar::Asset {
                asset_type: "native".to_string(),
                asset_code: None,
                asset_issuer: None,
            },
            counter: crate::rpc::stellar::Asset {
                asset_type: "native".to_string(),
                asset_code: None,
                asset_issuer: None,
            },
        })
    }

    async fn fetch_liquidity_pools(
        &self,
        _limit: u32,
        _cursor: Option<&str>,
    ) -> Result<Vec<crate::rpc::stellar::HorizonLiquidityPool>, RpcError> {
        Ok(vec![])
    }

    async fn fetch_pool_trades(&self, _pool_id: &str, _limit: u32) -> Result<Vec<Trade>, RpcError> {
        Ok(vec![])
    }

    async fn fetch_assets(
        &self,
        _limit: u32,
        _sponsored: bool,
    ) -> Result<Vec<crate::rpc::stellar::HorizonAsset>, RpcError> {
        Ok(vec![])
    }
}
