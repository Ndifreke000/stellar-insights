use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::network_context_middleware::NetworkContext;
use crate::models::push_notification_service::{Config, Response, State};

/// Push notification service with network-context awareness.
#[derive(Clone)]
pub struct PushNotificationService {
    config: Config,
    state: Arc<RwLock<State>>,
}

impl PushNotificationService {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(State::default())),
        }
    }

    /// Process a push notification for the given network context.
    pub async fn process(&self, context: &NetworkContext) -> Result<Response> {
        if !self.config.enabled {
            tracing::warn!("Push notification service is disabled");
            return Ok(Response {
                success: false,
                message: "Service disabled".to_string(),
                network: format!("{:?}", context.network),
            });
        }

        let mut state = self.state.write().await;
        state.notifications_sent += 1;

        tracing::info!(
            network = ?context.network,
            notifications_sent = state.notifications_sent,
            max_retries = self.config.max_retries,
            "Push notification processed"
        );

        Ok(Response {
            success: true,
            message: "Notification processed".to_string(),
            network: format!("{:?}", context.network),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::network_context_middleware::NetworkContext;

    #[tokio::test]
    async fn test_basic_functionality() {
        let instance = PushNotificationService::new(Config::default());
        let result = instance.process(&NetworkContext::testnet()).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(resp.success);
    }

    #[tokio::test]
    async fn test_mainnet() {
        let instance = PushNotificationService::new(Config::default());
        let result = instance.process(&NetworkContext::mainnet()).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(resp.success);
        assert!(resp.network.to_lowercase().contains("mainnet"));
    }

    #[tokio::test]
    async fn test_disabled_service() {
        let config = Config {
            enabled: false,
            max_retries: 3,
        };
        let instance = PushNotificationService::new(config);
        let result = instance.process(&NetworkContext::testnet()).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(!resp.success);
        assert_eq!(resp.message, "Service disabled");
    }

    #[tokio::test]
    async fn test_notification_count_increments() {
        let instance = PushNotificationService::new(Config::default());
        instance.process(&NetworkContext::testnet()).await.unwrap();
        instance.process(&NetworkContext::mainnet()).await.unwrap();
        let state = instance.state.read().await;
        assert_eq!(state.notifications_sent, 2);
    }
}
