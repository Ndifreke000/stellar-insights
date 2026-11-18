//! Soroban / Stellar JSON-RPC client.
//!
//! [`SorobanRpc`] is the narrow interface for the JSON-RPC endpoint
//! (`getHealth`, `getLedgers`). Like [`crate::rpc::horizon::HorizonApi`] it is
//! implemented for every [`crate::rpc::StellarRpcClientTrait`] implementor.

pub mod client;
pub mod models;

use crate::rpc::client_trait::StellarRpcClientTrait;
use crate::rpc::error::RpcError;
pub use models::*;

#[async_trait::async_trait]
pub trait SorobanRpc: Send + Sync {
    async fn health(&self) -> Result<HealthResponse, RpcError>;
    async fn ledgers(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError>;
}

#[async_trait::async_trait]
impl<T: StellarRpcClientTrait> SorobanRpc for T {
    async fn health(&self) -> Result<HealthResponse, RpcError> {
        StellarRpcClientTrait::check_health(self).await
    }
    async fn ledgers(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError> {
        StellarRpcClientTrait::fetch_ledgers(self, start_ledger, limit, cursor).await
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::rpc::{MockStellarRpcClient, StellarRpcClient};
    use std::sync::Arc;

    async fn run_suite(rpc: Arc<dyn SorobanRpc>) {
        let health = rpc.health().await.expect("health");
        assert!(health.latest_ledger >= health.oldest_ledger);
        let page = rpc.ledgers(None, 3, None).await.expect("ledgers");
        assert!(page.ledgers.len() <= 3);
    }

    #[tokio::test]
    async fn mock_trait_impl_satisfies_contract() {
        run_suite(Arc::new(MockStellarRpcClient::testnet())).await;
    }

    #[tokio::test]
    async fn mock_mode_client_satisfies_contract() {
        run_suite(Arc::new(StellarRpcClient::new_with_defaults(true))).await;
    }
}
