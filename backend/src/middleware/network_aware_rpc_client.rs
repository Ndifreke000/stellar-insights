use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::network_aware_rpc_client::NetworkAwareRpcConfig;
use crate::models::network_context_middleware::NetworkContext;

#[derive(Clone)]
pub struct NetworkAwareRpcClient {
    config: NetworkAwareRpcConfig,
    state: Arc<RwLock<bool>>,
}

impl NetworkAwareRpcClient {
    pub fn new(config: NetworkAwareRpcConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn process(&self, context: &NetworkContext) -> Result<()> {
        match context.network {
            crate::models::network_context_middleware::Network::Testnet => {
                tracing::info!("RPC client using testnet endpoint");
            }
            crate::models::network_context_middleware::Network::Mainnet => {
                tracing::info!("RPC client using mainnet endpoint");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_functionality() {
        let cfg = NetworkAwareRpcConfig::default();
        let instance = NetworkAwareRpcClient::new(cfg);
        let ctx = crate::models::network_context_middleware::NetworkContext::testnet();
        let result = instance.process(&ctx).await;
        assert!(result.is_ok());
    }
}
