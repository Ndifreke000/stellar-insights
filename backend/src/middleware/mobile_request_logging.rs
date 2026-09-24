use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::mobile_request_logging::MobileLogConfig;
use crate::models::network_context_middleware::NetworkContext;

#[derive(Clone)]
pub struct MobileRequestLogging {
    config: MobileLogConfig,
    state: Arc<RwLock<u64>>,
}

impl MobileRequestLogging {
    pub fn new(config: MobileLogConfig) -> Self {
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
            log_headers = self.config.log_headers,
            log_body = self.config.log_body,
            total_requests = *request_count,
            "Mobile request logged"
        );
        Ok(())
    }
}
