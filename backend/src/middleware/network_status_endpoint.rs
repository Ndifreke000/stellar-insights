use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::network_context_middleware::NetworkContext;

#[derive(Clone, Debug)]
pub struct NetworkStatus {
    pub operational: bool,
    pub latency_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
pub struct NetworkStatusEndpoint {
    status: Arc<RwLock<NetworkStatus>>,
}

impl NetworkStatusEndpoint {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(NetworkStatus {
                operational: true,
                latency_ms: 0,
                timestamp: chrono::Utc::now(),
            })),
        }
    }

    pub async fn check_status(&self, context: &NetworkContext) -> Result<NetworkStatus> {
        let mut status = self.status.write().await;

        let start = std::time::Instant::now();

        match context.network {
            crate::models::network_context_middleware::Network::Testnet
            | crate::models::network_context_middleware::Network::Mainnet => {
                status.operational = true;
                status.latency_ms = start.elapsed().as_millis() as u64;
                status.timestamp = chrono::Utc::now();

                tracing::info!(
                    network = ?context.network,
                    latency_ms = status.latency_ms,
                    "Network status checked"
                );
            }
        }

        Ok(status.clone())
    }

    pub async fn get_status(&self) -> Result<NetworkStatus> {
        let status = self.status.read().await;
        Ok(status.clone())
    }

    pub async fn set_operational(&self, operational: bool) -> Result<()> {
        let mut status = self.status.write().await;
        status.operational = operational;
        status.timestamp = chrono::Utc::now();

        tracing::info!(
            operational = operational,
            "Network operational status updated"
        );

        Ok(())
    }
}

impl Default for NetworkStatusEndpoint {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_status_testnet() {
        let endpoint = NetworkStatusEndpoint::new();
        let ctx = NetworkContext::testnet();

        let result = endpoint.check_status(&ctx).await;
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.operational);
        assert!(status.latency_ms <= 10_000, "latency sanity check: {}ms exceeds 10s", status.latency_ms);
    }

    #[tokio::test]
    async fn test_check_status_mainnet() {
        let endpoint = NetworkStatusEndpoint::new();
        let ctx = NetworkContext::mainnet();

        let result = endpoint.check_status(&ctx).await;
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.operational);
    }

    #[tokio::test]
    async fn test_get_status() {
        let endpoint = NetworkStatusEndpoint::new();
        let result = endpoint.get_status().await;

        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.operational);
    }

    #[tokio::test]
    async fn test_set_operational() {
        let endpoint = NetworkStatusEndpoint::new();

        let result = endpoint.set_operational(false).await;
        assert!(result.is_ok());

        let status = endpoint.get_status().await;
        assert!(status.is_ok());
        assert!(!status.unwrap().operational);
    }

    #[tokio::test]
    async fn test_timestamp_updates() {
        let endpoint = NetworkStatusEndpoint::new();

        let status1 = endpoint.get_status().await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let _ = endpoint.set_operational(false).await;
        let status2 = endpoint.get_status().await.unwrap();

        assert!(status2.timestamp >= status1.timestamp);
    }
}
