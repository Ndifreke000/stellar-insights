use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StellarNetwork {
    Mainnet,
    Testnet,
}

impl fmt::Display for StellarNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StellarNetwork::Mainnet => write!(f, "mainnet"),
            StellarNetwork::Testnet => write!(f, "testnet"),
        }
    }
}

impl std::str::FromStr for StellarNetwork {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mainnet" => Ok(StellarNetwork::Mainnet),
            "testnet" => Ok(StellarNetwork::Testnet),
            _ => Err(format!(
                "Invalid network: {}. Must be 'mainnet' or 'testnet'",
                s
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
    pub fn for_network(network: StellarNetwork) -> Self {
        let (rpc_url, horizon_url, network_passphrase) = match network {
            StellarNetwork::Mainnet => (
                std::env::var("STELLAR_RPC_URL_MAINNET")
                    .unwrap_or_else(|_| "https://stellar.api.onfinality.io/public".to_string()),
                std::env::var("STELLAR_HORIZON_URL_MAINNET")
                    .unwrap_or_else(|_| "https://horizon.stellar.org".to_string()),
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
    pub fn network_passphrase(&self) -> &str {
        &self.network_passphrase
    }

    /// Check if this is the mainnet
    pub fn is_mainnet(&self) -> bool {
        self.network == StellarNetwork::Mainnet
    }

    /// Check if this is the testnet
    pub fn is_testnet(&self) -> bool {
        self.network == StellarNetwork::Testnet
    }

    /// Get a display-friendly network name
    pub fn display_name(&self) -> &str {
        match self.network {
            StellarNetwork::Mainnet => "Stellar Mainnet",
            StellarNetwork::Testnet => "Stellar Testnet",
        }
    }

    /// Get network color for UI (hex color code)
    pub fn color(&self) -> &str {
        match self.network {
            StellarNetwork::Mainnet => "#00D4AA", // Stellar green
            StellarNetwork::Testnet => "#FF6B35", // Orange for testnet
        }
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
        let mainnet_config = NetworkConfig::for_network(StellarNetwork::Mainnet);
        assert!(mainnet_config.is_mainnet());
        assert!(!mainnet_config.is_testnet());
        assert_eq!(mainnet_config.display_name(), "Stellar Mainnet");

        let testnet_config = NetworkConfig::for_network(StellarNetwork::Testnet);
        assert!(!testnet_config.is_mainnet());
        assert!(testnet_config.is_testnet());
        assert_eq!(testnet_config.display_name(), "Stellar Testnet");
    }
}
