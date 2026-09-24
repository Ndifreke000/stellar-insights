use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::network_context_middleware::NetworkContext;
use crate::models::websocket_real_time_updates::WsConfig;

#[derive(Clone)]
pub struct WebSocketRealTimeUpdates {
    config: WsConfig,
    state: Arc<RwLock<usize>>,
}

impl WebSocketRealTimeUpdates {
    pub fn new(config: WsConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn process(&self, context: &NetworkContext) -> Result<()> {
        let mut connections = self.state.write().await;
        if *connections >= self.config.max_connections {
            anyhow::bail!("Max WebSocket connections reached");
        }
        *connections += 1;
        tracing::info!(
            network = ?context.network,
            connections = *connections,
            heartbeat_interval_secs = self.config.heartbeat_interval_secs,
            "WebSocket real-time updates processed"
        );
        Ok(())
    }
}
