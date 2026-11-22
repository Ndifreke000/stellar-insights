use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::network_context_middleware::NetworkContext;

/// Default number of seconds without a closed ledger before the network is
/// considered halted. Overridable via `NetworkStatusEndpoint::with_stall_threshold`.
pub const DEFAULT_STALL_THRESHOLD_SECS: u64 = 30;

#[derive(Clone, Debug)]
pub struct NetworkStatus {
    pub operational: bool,
    pub latency_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// True when no ledger has closed for longer than the stall threshold.
    pub halted: bool,
    /// Seconds since the last observed ledger close, if known.
    pub seconds_since_last_ledger: Option<u64>,
    /// Protocol version currently observed on the network.
    pub protocol_version: Option<u32>,
    /// Protocol version announced for the next upgrade, if any.
    pub upcoming_protocol_version: Option<u32>,
}

impl NetworkStatus {
    /// Whether corridor alerts should be suppressed (network is halted).
    pub fn suppress_corridor_alerts(&self) -> bool {
        self.halted
    }
}

#[derive(Clone)]
pub struct NetworkStatusEndpoint {
    status: Arc<RwLock<NetworkStatus>>,
    stall_threshold_secs: u64,
}

impl NetworkStatusEndpoint {
    pub fn new() -> Self {
        Self::with_stall_threshold(DEFAULT_STALL_THRESHOLD_SECS)
    }

    pub fn with_stall_threshold(stall_threshold_secs: u64) -> Self {
        Self {
            status: Arc::new(RwLock::new(NetworkStatus {
                operational: true,
                latency_ms: 0,
                timestamp: chrono::Utc::now(),
                halted: false,
                seconds_since_last_ledger: None,
                protocol_version: None,
                upcoming_protocol_version: None,
            })),
            stall_threshold_secs,
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

    /// Record the close time of the most recent ledger and recompute halt state.
    ///
    /// A halt is declared when the gap since the last close exceeds the
    /// configured stall threshold.
    pub async fn record_ledger_close(
        &self,
        closed_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<NetworkStatus> {
        let mut status = self.status.write().await;
        let now = chrono::Utc::now();
        let gap = (now - closed_at).num_seconds().max(0) as u64;

        status.seconds_since_last_ledger = Some(gap);
        status.halted = gap > self.stall_threshold_secs;
        status.timestamp = now;

        if status.halted {
            tracing::warn!(
                gap_secs = gap,
                threshold_secs = self.stall_threshold_secs,
                "Stellar network halt detected: no ledger close within threshold"
            );
        }

        Ok(status.clone())
    }

    /// Re-evaluate halt state against the current wall clock without a new
    /// ledger close. Useful for periodic polling.
    pub async fn evaluate_stall(&self) -> Result<NetworkStatus> {
        let mut status = self.status.write().await;
        if let Some(gap) = status.seconds_since_last_ledger {
            let now = chrono::Utc::now();
            let elapsed = gap + (now - status.timestamp).num_seconds().max(0) as u64;
            status.seconds_since_last_ledger = Some(elapsed);
            status.halted = elapsed > self.stall_threshold_secs;
            status.timestamp = now;
        }
        Ok(status.clone())
    }

    /// Track the observed protocol version and any announced upcoming upgrade.
    pub async fn record_protocol_version(
        &self,
        current: u32,
        upcoming: Option<u32>,
    ) -> Result<NetworkStatus> {
        let mut status = self.status.write().await;

        if status.protocol_version != Some(current) {
            tracing::info!(
                previous = ?status.protocol_version,
                current = current,
                "Stellar protocol version changed"
            );
        }

        status.protocol_version = Some(current);
        status.upcoming_protocol_version = upcoming;
        status.timestamp = chrono::Utc::now();

        Ok(status.clone())
    }

    /// Banner payload for the frontend reflecting halt / upgrade state.
    pub async fn banner(&self) -> Result<Option<NetworkBanner>> {
        let status = self.status.read().await;

        if status.halted {
            return Ok(Some(NetworkBanner {
                level: BannerLevel::Critical,
                kind: BannerKind::Halt,
                message: format!(
                    "Stellar network appears halted: no ledger close for {}s. Corridor alerts are paused.",
                    status.seconds_since_last_ledger.unwrap_or(0)
                ),
            }));
        }

        if let Some(upcoming) = status.upcoming_protocol_version {
            return Ok(Some(NetworkBanner {
                level: BannerLevel::Info,
                kind: BannerKind::ProtocolUpgrade,
                message: format!(
                    "Stellar protocol upgrade to version {} is upcoming.",
                    upcoming
                ),
            }));
        }

        Ok(None)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BannerLevel {
    Info,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BannerKind {
    Halt,
    ProtocolUpgrade,
}

#[derive(Clone, Debug)]
pub struct NetworkBanner {
    pub level: BannerLevel,
    pub kind: BannerKind,
    pub message: String,
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
        assert!(
            status.latency_ms <= 10_000,
            "latency sanity check: {}ms exceeds 10s",
            status.latency_ms
        );
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
        assert!(!status.halted);
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

    #[tokio::test]
    async fn test_recent_ledger_close_not_halted() {
        let endpoint = NetworkStatusEndpoint::with_stall_threshold(30);
        let status = endpoint
            .record_ledger_close(chrono::Utc::now())
            .await
            .unwrap();

        assert!(!status.halted);
        assert!(!status.suppress_corridor_alerts());
        assert!(endpoint.banner().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_stall_triggers_halt_and_suppresses_alerts() {
        let endpoint = NetworkStatusEndpoint::with_stall_threshold(5);
        let stale = chrono::Utc::now() - chrono::Duration::seconds(60);

        let status = endpoint.record_ledger_close(stale).await.unwrap();
        assert!(status.halted);
        assert!(status.suppress_corridor_alerts());

        let banner = endpoint.banner().await.unwrap().expect("halt banner");
        assert_eq!(banner.kind, BannerKind::Halt);
        assert_eq!(banner.level, BannerLevel::Critical);
    }

    #[tokio::test]
    async fn test_protocol_upgrade_banner() {
        let endpoint = NetworkStatusEndpoint::new();
        endpoint
            .record_ledger_close(chrono::Utc::now())
            .await
            .unwrap();
        endpoint.record_protocol_version(21, Some(22)).await.unwrap();

        let banner = endpoint.banner().await.unwrap().expect("upgrade banner");
        assert_eq!(banner.kind, BannerKind::ProtocolUpgrade);
        assert_eq!(banner.level, BannerLevel::Info);
    }
}
