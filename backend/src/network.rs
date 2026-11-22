use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(utoipa::ToSchema)]
pub enum StellarNetwork {
    Mainnet,
    Testnet,
}

impl fmt::Display for StellarNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mainnet => write!(f, "mainnet"),
            Self::Testnet => write!(f, "testnet"),
        }
    }
}

impl std::str::FromStr for StellarNetwork {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mainnet" => Ok(Self::Mainnet),
            "testnet" => Ok(Self::Testnet),
            _ => Err(format!(
                "Invalid network: {s}. Must be 'mainnet' or 'testnet'"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub network: StellarNetwork,
    pub rpc_url: String,
    pub horizon_url: String,
    pub network_passphrase: String,
}

impl NetworkConfig {
    /// Create network configuration from environment variables
    #[must_use]
    pub fn from_env() -> Self {
        let network_str =
            std::env::var("STELLAR_NETWORK").unwrap_or_else(|_| "mainnet".to_string());

        let network = network_str.parse::<StellarNetwork>().unwrap_or_else(|_| {
            tracing::warn!(
                "Invalid STELLAR_NETWORK value '{}', defaulting to mainnet",
                network_str
            );
            StellarNetwork::Mainnet
        });

        Self::for_network(network)
    }

    /// Create network configuration for a specific network
    #[must_use]
    pub fn for_network(network: StellarNetwork) -> Self {
        let (rpc_url, horizon_url, network_passphrase) = match network {
            StellarNetwork::Mainnet => (
                std::env::var("STELLAR_RPC_URL_MAINNET").unwrap_or_else(|_| {
                    panic!(
                        "STELLAR_RPC_URL_MAINNET must be set — \
                         refusing to start without an explicit mainnet RPC URL"
                    )
                }),
                std::env::var("STELLAR_HORIZON_URL_MAINNET").unwrap_or_else(|_| {
                    panic!(
                        "STELLAR_HORIZON_URL_MAINNET must be set — \
                         refusing to start without an explicit mainnet Horizon URL"
                    )
                }),
                "Public Global Stellar Network ; September 2015".to_string(),
            ),
            StellarNetwork::Testnet => (
                std::env::var("STELLAR_RPC_URL_TESTNET")
                    .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string()),
                std::env::var("STELLAR_HORIZON_URL_TESTNET")
                    .unwrap_or_else(|_| "https://horizon-testnet.stellar.org".to_string()),
                "Test SDF Network ; September 2015".to_string(),
            ),
        };

        Self {
            network,
            rpc_url,
            horizon_url,
            network_passphrase,
        }
    }

    /// Get the network passphrase for transaction signing
    #[must_use]
    pub fn network_passphrase(&self) -> &str {
        &self.network_passphrase
    }

    /// Check if this is the mainnet
    #[must_use]
    pub fn is_mainnet(&self) -> bool {
        self.network == StellarNetwork::Mainnet
    }

    /// Check if this is the testnet
    #[must_use]
    pub fn is_testnet(&self) -> bool {
        self.network == StellarNetwork::Testnet
    }

    /// Get a display-friendly network name
    #[must_use]
    pub const fn display_name(&self) -> &str {
        match self.network {
            StellarNetwork::Mainnet => "Stellar Mainnet",
            StellarNetwork::Testnet => "Stellar Testnet",
        }
    }

    /// Get network color for UI (hex color code)
    #[must_use]
    pub const fn color(&self) -> &str {
        match self.network {
            StellarNetwork::Mainnet => "#00D4AA", // Stellar green
            StellarNetwork::Testnet => "#FF6B35", // Orange for testnet
        }
    }

    /// Fetch the current Soroban protocol version from Horizon.
    pub async fn current_protocol_version(&self) -> Result<u32> {
        let url = format!("{}/ledgers?order=desc&limit=1", self.horizon_url);
        let resp: serde_json::Value = reqwest::Client::new()
            .get(&url)
            .send()
            .await
            .context("Failed to fetch latest ledger from Horizon")?
            .json()
            .await
            .context("Failed to parse Horizon ledger response")?;

        resp["_embedded"]["records"][0]["protocol_version"]
            .as_u64()
            .map(|v| v as u32)
            .ok_or_else(|| anyhow::anyhow!("Missing protocol_version in Horizon ledger response"))
    }

    /// Fetch the latest closed ledger's sequence, close time and protocol version.
    pub async fn latest_ledger(&self) -> Result<LedgerSnapshot> {
        let url = format!("{}/ledgers?order=desc&limit=1", self.horizon_url);
        let resp: serde_json::Value = reqwest::Client::new()
            .get(&url)
            .send()
            .await
            .context("Failed to fetch latest ledger from Horizon")?
            .json()
            .await
            .context("Failed to parse Horizon ledger response")?;

        let record = &resp["_embedded"]["records"][0];
        let sequence = record["sequence"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing sequence in Horizon ledger response"))?;
        let protocol_version = record["protocol_version"]
            .as_u64()
            .map(|v| v as u32)
            .ok_or_else(|| anyhow::anyhow!("Missing protocol_version in Horizon ledger response"))?;
        let closed_at = record["closed_at"]
            .as_str()
            .and_then(parse_rfc3339)
            .ok_or_else(|| anyhow::anyhow!("Missing closed_at in Horizon ledger response"))?;

        Ok(LedgerSnapshot {
            sequence,
            protocol_version,
            closed_at,
        })
    }
}

/// A point-in-time view of the most recently closed ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerSnapshot {
    pub sequence: u64,
    pub protocol_version: u32,
    pub closed_at: SystemTime,
}

/// Parse an RFC 3339 timestamp (e.g. `2024-01-01T00:00:00Z`) into a `SystemTime`.
fn parse_rfc3339(s: &str) -> Option<SystemTime> {
    let dt = chrono::DateTime::parse_from_rfc3339(s).ok()?;
    Some(SystemTime::from(dt))
}

/// Default number of seconds without a new ledger before the network is
/// considered halted. Stellar closes a ledger roughly every 5 seconds.
pub const DEFAULT_HALT_THRESHOLD_SECS: u64 = 60;

/// Health state of the Stellar network derived from ledger close times and
/// protocol version transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(utoipa::ToSchema)]
pub enum NetworkHealth {
    /// Ledgers are closing normally.
    Healthy,
    /// No ledger has closed within the halt threshold.
    Halted,
    /// The protocol version changed on the latest ledger.
    ProtocolUpgrade,
}

/// Banner payload surfaced to the frontend describing the current network
/// state. `severity` is one of `info`, `warning`, or `critical`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(utoipa::ToSchema)]
pub struct NetworkBanner {
    pub health: NetworkHealth,
    pub severity: String,
    pub title: String,
    pub message: String,
    /// Seconds since the last ledger closed, if known.
    pub seconds_since_close: Option<u64>,
    /// Current protocol version, if known.
    pub protocol_version: Option<u32>,
    /// Whether corridor alerts should be suppressed (true during a halt).
    pub suppress_alerts: bool,
}

/// Tracks network health across successive ledger observations.
#[derive(Debug, Clone)]
pub struct NetworkStatusTracker {
    halt_threshold: Duration,
    last_protocol_version: Option<u32>,
}

impl NetworkStatusTracker {
    /// Create a tracker with the given halt threshold (seconds without a ledger).
    #[must_use]
    pub fn new(halt_threshold_secs: u64) -> Self {
        Self {
            halt_threshold: Duration::from_secs(halt_threshold_secs),
            last_protocol_version: None,
        }
    }

    /// Create a tracker using the default halt threshold.
    #[must_use]
    pub fn with_default_threshold() -> Self {
        Self::new(DEFAULT_HALT_THRESHOLD_SECS)
    }

    /// Evaluate the current network health from the latest ledger snapshot.
    ///
    /// A halt is detected when the ledger's close time is older than the halt
    /// threshold relative to `now`. A protocol upgrade is reported when the
    /// protocol version differs from the previously observed version.
    pub fn evaluate(&mut self, snapshot: &LedgerSnapshot, now: SystemTime) -> NetworkBanner {
        let seconds_since_close = now
            .duration_since(snapshot.closed_at)
            .unwrap_or(Duration::ZERO)
            .as_secs();

        let protocol_changed = self
            .last_protocol_version
            .is_some_and(|prev| prev != snapshot.protocol_version);
        self.last_protocol_version = Some(snapshot.protocol_version);

        if seconds_since_close > self.halt_threshold.as_secs() {
            return NetworkBanner {
                health: NetworkHealth::Halted,
                severity: "critical".to_string(),
                title: "Stellar network halt detected".to_string(),
                message: format!(
                    "No ledger has closed in {seconds_since_close}s (threshold {}s). \
                     Corridor alerts are paused until the network resumes.",
                    self.halt_threshold.as_secs()
                ),
                seconds_since_close: Some(seconds_since_close),
                protocol_version: Some(snapshot.protocol_version),
                suppress_alerts: true,
            };
        }

        if protocol_changed {
            return NetworkBanner {
                health: NetworkHealth::ProtocolUpgrade,
                severity: "info".to_string(),
                title: "Stellar protocol upgrade".to_string(),
                message: format!(
                    "The network upgraded to protocol version {}.",
                    snapshot.protocol_version
                ),
                seconds_since_close: Some(seconds_since_close),
                protocol_version: Some(snapshot.protocol_version),
                suppress_alerts: false,
            };
        }

        NetworkBanner {
            health: NetworkHealth::Healthy,
            severity: "info".to_string(),
            title: "Stellar network healthy".to_string(),
            message: "Ledgers are closing normally.".to_string(),
            seconds_since_close: Some(seconds_since_close),
            protocol_version: Some(snapshot.protocol_version),
            suppress_alerts: false,
        }
    }

    /// Whether corridor alerts should currently be suppressed.
    #[must_use]
    pub fn alerts_suppressed(&self, banner: &NetworkBanner) -> bool {
        banner.suppress_alerts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_from_str() {
        assert_eq!(
            "mainnet".parse::<StellarNetwork>().unwrap(),
            StellarNetwork::Mainnet
        );
        assert_eq!(
            "testnet".parse::<StellarNetwork>().unwrap(),
            StellarNetwork::Testnet
        );
        assert_eq!(
            "MAINNET".parse::<StellarNetwork>().unwrap(),
            StellarNetwork::Mainnet
        );
        assert_eq!(
            "TESTNET".parse::<StellarNetwork>().unwrap(),
            StellarNetwork::Testnet
        );

        assert!("invalid".parse::<StellarNetwork>().is_err());
    }

    #[test]
    fn test_network_display() {
        assert_eq!(StellarNetwork::Mainnet.to_string(), "mainnet");
        assert_eq!(StellarNetwork::Testnet.to_string(), "testnet");
    }

    #[test]
    fn test_network_config_creation() {
        // Mainnet now requires explicit env vars — set them for test isolation.
        std::env::set_var("STELLAR_RPC_URL_MAINNET", "https://rpc.example.com");
        std::env::set_var("STELLAR_HORIZON_URL_MAINNET", "https://horizon.example.com");

        let mainnet_config = NetworkConfig::for_network(StellarNetwork::Mainnet);
        assert!(mainnet_config.is_mainnet());
        assert!(!mainnet_config.is_testnet());
        assert_eq!(mainnet_config.display_name(), "Stellar Mainnet");

        let testnet_config = NetworkConfig::for_network(StellarNetwork::Testnet);
        assert!(!testnet_config.is_mainnet());
        assert!(testnet_config.is_testnet());
        assert_eq!(testnet_config.display_name(), "Stellar Testnet");
    }

    fn snapshot(sequence: u64, protocol_version: u32, closed_at: SystemTime) -> LedgerSnapshot {
        LedgerSnapshot {
            sequence,
            protocol_version,
            closed_at,
        }
    }

    #[test]
    fn test_healthy_network() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let snap = snapshot(100, 20, now - Duration::from_secs(5));
        let mut tracker = NetworkStatusTracker::new(60);

        let banner = tracker.evaluate(&snap, now);
        assert_eq!(banner.health, NetworkHealth::Healthy);
        assert!(!banner.suppress_alerts);
        assert_eq!(banner.seconds_since_close, Some(5));
    }

    #[test]
    fn test_halt_detected_and_alerts_suppressed() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let snap = snapshot(100, 20, now - Duration::from_secs(120));
        let mut tracker = NetworkStatusTracker::new(60);

        let banner = tracker.evaluate(&snap, now);
        assert_eq!(banner.health, NetworkHealth::Halted);
        assert!(banner.suppress_alerts);
        assert!(tracker.alerts_suppressed(&banner));
        assert_eq!(banner.severity, "critical");
    }

    #[test]
    fn test_protocol_upgrade_detected() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let mut tracker = NetworkStatusTracker::new(60);

        let first = snapshot(100, 20, now - Duration::from_secs(5));
        assert_eq!(
            tracker.evaluate(&first, now).health,
            NetworkHealth::Healthy
        );

        let upgraded = snapshot(101, 21, now - Duration::from_secs(5));
        let banner = tracker.evaluate(&upgraded, now);
        assert_eq!(banner.health, NetworkHealth::ProtocolUpgrade);
        assert_eq!(banner.protocol_version, Some(21));
        assert!(!banner.suppress_alerts);
    }

    #[test]
    fn test_parse_rfc3339() {
        assert!(parse_rfc3339("2024-01-01T00:00:00Z").is_some());
        assert!(parse_rfc3339("not-a-date").is_none());
    }
}
