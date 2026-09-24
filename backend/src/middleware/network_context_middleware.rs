use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::network_context_middleware::NetworkContext;

#[derive(Clone)]
pub struct NetworkContextMiddleware {
    state: Arc<RwLock<String>>,
}

impl NetworkContextMiddleware {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(String::new())),
        }
    }

    pub async fn process(&self, context: &NetworkContext) -> Result<()> {
        // Validate network
        match context.network {
            crate::models::network_context_middleware::Network::Testnet
            | crate::models::network_context_middleware::Network::Mainnet => {
                let mut s = self.state.write().await;
                *s = format!("{:?}", context.network);
                tracing::info!(network = ?context.network, "Processed network context");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_functionality() {
        let instance = NetworkContextMiddleware::new();
        let ctx = NetworkContext::testnet();
        let result = instance.process(&ctx).await;
        assert!(result.is_ok());
    }
}
