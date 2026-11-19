//! Horizon REST API client.
//!
//! [`HorizonApi`] is the narrow interface for everything served by Horizon
//! (ledgers, payments, trades, order books, pools, assets). Both the real
//! [`crate::rpc::StellarRpcClient`] and [`crate::rpc::MockStellarRpcClient`]
//! implement it through the blanket impl over
//! [`crate::rpc::StellarRpcClientTrait`], so call sites can depend on
//! `Arc<dyn HorizonApi>` and tests can swap in the mock.

pub mod client;
pub mod models;
pub mod pagination;
pub mod pools;

use crate::rpc::client_trait::StellarRpcClientTrait;
use crate::rpc::error::RpcError;
pub use models::*;

#[async_trait::async_trait]
pub trait HorizonApi: Send + Sync {
    async fn latest_ledger(&self) -> Result<crate::rpc::soroban::LedgerInfo, RpcError>;
    async fn ledger_by_sequence(
        &self,
        sequence: u64,
    ) -> Result<crate::rpc::soroban::LedgerInfo, RpcError>;
    async fn payments_for_ledger(&self, sequence: u64) -> Result<Vec<Payment>, RpcError>;
    async fn transactions_for_ledger(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError>;
    async fn operations_for_ledger(&self, sequence: u64)
        -> Result<Vec<HorizonOperation>, RpcError>;
    async fn account_payments(
        &self,
        account_id: &str,
        limit: u32,
    ) -> Result<Vec<Payment>, RpcError>;
    async fn trades(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Trade>, RpcError>;
    async fn order_book(
        &self,
        selling: &Asset,
        buying: &Asset,
        limit: u32,
    ) -> Result<OrderBook, RpcError>;
    async fn liquidity_pools(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<HorizonLiquidityPool>, RpcError>;
    async fn pool_trades(&self, pool_id: &str, limit: u32) -> Result<Vec<Trade>, RpcError>;
    async fn assets(&self, limit: u32, sponsored: bool) -> Result<Vec<HorizonAsset>, RpcError>;
}

#[async_trait::async_trait]
impl<T: StellarRpcClientTrait> HorizonApi for T {
    async fn latest_ledger(&self) -> Result<crate::rpc::soroban::LedgerInfo, RpcError> {
        self.fetch_latest_ledger().await
    }
    async fn ledger_by_sequence(
        &self,
        sequence: u64,
    ) -> Result<crate::rpc::soroban::LedgerInfo, RpcError> {
        self.fetch_ledger_by_sequence(sequence).await
    }
    async fn payments_for_ledger(&self, sequence: u64) -> Result<Vec<Payment>, RpcError> {
        self.fetch_payments_for_ledger(sequence).await
    }
    async fn transactions_for_ledger(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError> {
        self.fetch_transactions_for_ledger(sequence).await
    }
    async fn operations_for_ledger(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonOperation>, RpcError> {
        self.fetch_operations_for_ledger(sequence).await
    }
    async fn account_payments(
        &self,
        account_id: &str,
        limit: u32,
    ) -> Result<Vec<Payment>, RpcError> {
        self.fetch_account_payments(account_id, limit).await
    }
    async fn trades(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Trade>, RpcError> {
        StellarRpcClientTrait::fetch_trades(self, limit, cursor).await
    }
    async fn order_book(
        &self,
        selling: &Asset,
        buying: &Asset,
        limit: u32,
    ) -> Result<OrderBook, RpcError> {
        StellarRpcClientTrait::fetch_order_book(self, selling, buying, limit).await
    }
    async fn liquidity_pools(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<HorizonLiquidityPool>, RpcError> {
        StellarRpcClientTrait::fetch_liquidity_pools(self, limit, cursor).await
    }
    async fn pool_trades(&self, pool_id: &str, limit: u32) -> Result<Vec<Trade>, RpcError> {
        StellarRpcClientTrait::fetch_pool_trades(self, pool_id, limit).await
    }
    async fn assets(&self, limit: u32, sponsored: bool) -> Result<Vec<HorizonAsset>, RpcError> {
        StellarRpcClientTrait::fetch_assets(self, limit, sponsored).await
    }
}

#[cfg(test)]
mod contract_tests {
    //! Contract tests: the same suite runs against every `HorizonApi`
    //! implementation. Set `STELLAR_CONTRACT_TESTS_TESTNET=1` to also run it
    //! against the public testnet.
    use super::*;
    use crate::rpc::{MockStellarRpcClient, StellarRpcClient};
    use std::sync::Arc;

    async fn run_suite(api: Arc<dyn HorizonApi>) {
        let latest = api.latest_ledger().await.expect("latest ledger");
        assert!(latest.sequence > 0);
        assert!(!latest.hash.is_empty());
        let trades = api.trades(5, None).await.expect("trades");
        assert!(trades.len() <= 5);
        let pools = api.liquidity_pools(3, None).await.expect("pools");
        assert!(pools.len() <= 3);
    }

    #[tokio::test]
    async fn mock_trait_impl_satisfies_contract() {
        run_suite(Arc::new(MockStellarRpcClient::testnet())).await;
    }

    #[tokio::test]
    async fn mock_mode_client_satisfies_contract() {
        run_suite(Arc::new(StellarRpcClient::new_with_defaults(true))).await;
    }

    #[tokio::test]
    async fn testnet_satisfies_contract_when_enabled() {
        if std::env::var("STELLAR_CONTRACT_TESTS_TESTNET").as_deref() != Ok("1") {
            return;
        }
        run_suite(Arc::new(StellarRpcClient::new_with_network(
            crate::network::StellarNetwork::Testnet,
            false,
        )))
        .await;
    }
}
