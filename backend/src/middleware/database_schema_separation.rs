use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::database_schema_separation::SchemaConfig;
use crate::models::network_context_middleware::NetworkContext;

#[derive(Clone)]
pub struct DatabaseSchemaSeparation {
    config: SchemaConfig,
    state: Arc<RwLock<String>>,
}

impl DatabaseSchemaSeparation {
    pub fn new(config: SchemaConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(String::new())),
        }
    }

    pub async fn process(&self, context: &NetworkContext) -> Result<()> {
        let schema = match context.network {
            crate::models::network_context_middleware::Network::Testnet => "testnet",
            crate::models::network_context_middleware::Network::Mainnet => "mainnet",
        };
        let mut s = self.state.write().await;
        *s = schema.to_string();
        tracing::info!(schema = %schema, "Database schema set for network");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_functionality() {
        let cfg = SchemaConfig::default();
        let instance = DatabaseSchemaSeparation::new(cfg);
        let ctx = crate::models::network_context_middleware::NetworkContext::testnet();
        let result = instance.process(&ctx).await;
        assert!(result.is_ok());
    }
}
