use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::api_versioning::VersioningConfig;
use crate::models::network_context_middleware::NetworkContext;

#[derive(Clone)]
pub struct ApiVersioning {
    config: VersioningConfig,
    state: Arc<RwLock<u32>>,
}

impl ApiVersioning {
    pub fn new(config: VersioningConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn process(&self, context: &NetworkContext) -> Result<()> {
        let mut request_count = self.state.write().await;
        *request_count += 1;
        tracing::info!(
            network = ?context.network,
            current_version = self.config.current_version,
            min_supported_version = self.config.min_supported_version,
            "API versioning processed"
        );
        Ok(())
    }
}
