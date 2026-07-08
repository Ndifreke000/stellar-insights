/// Multi-network configuration and management
/// Supports running multiple network contexts simultaneously (mainnet, testnet, custom)
use crate::network::{NetworkConfig, StellarNetwork};
use crate::rpc::StellarRpcClient;
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for multi-network operations
#[derive(Debug, Clone)]
pub struct MultiNetworkConfig {
    /// Map of network names to their configurations
    pub networks: HashMap<String, NetworkConfig>,
    /// The primary network to use when no network is specified
    pub primary_network: StellarNetwork,
}

/// Context for network-specific operations
#[derive(Debug, Clone)]
pub struct NetworkContext {
    pub network: StellarNetwork,
    pub config: NetworkConfig,
    pub rpc_url: String,
    pub horizon_url: String,
}

impl NetworkContext {
    /// Create a new network context
    pub fn new(network: StellarNetwork) -> Self {
        let config = NetworkConfig::for_network(network);
        let rpc_url = config.rpc_url.clone();
        let horizon_url = config.horizon_url.clone();

        Self {
            network,
            config,
            rpc_url,
            horizon_url,
        }
    }

    /// Get network identifier for logging and tracing
    pub fn network_id(&self) -> String {
        self.network.to_string()
    }

    /// Get display name for network
    pub fn display_name(&self) -> &str {
        self.config.display_name()
    }
}

impl MultiNetworkConfig {
    /// Create default multi-network config with mainnet and testnet
    pub fn default_networks() -> Self {
        let mut networks = HashMap::new();
        networks.insert(
            "mainnet".to_string(),
            NetworkConfig::for_network(StellarNetwork::Mainnet),
        );
        networks.insert(
            "testnet".to_string(),
            NetworkConfig::for_network(StellarNetwork::Testnet),
        );

        Self {
            networks,
            primary_network: StellarNetwork::Mainnet,
        }
    }

    /// Create multi-network config from environment
    pub fn from_env() -> Self {
        let mut config = Self::default_networks();

        // Override primary network from env if specified
        if let Ok(primary) = std::env::var("STELLAR_PRIMARY_NETWORK") {
            if let Ok(network) = primary.parse::<StellarNetwork>() {
                config.primary_network = network;
            }
        }

        config
    }

    /// Get network context by network type
    pub fn get_context(&self, network: StellarNetwork) -> Option<NetworkContext> {
        self.networks.values().find(|c| c.network == network)?;

        Some(NetworkContext::new(network))
    }

    /// Get primary network context
    pub fn get_primary_context(&self) -> NetworkContext {
        NetworkContext::new(self.primary_network)
    }

    /// Get all available networks
    pub fn available_networks(&self) -> Vec<NetworkContext> {
        vec![
            NetworkContext::new(StellarNetwork::Mainnet),
            NetworkContext::new(StellarNetwork::Testnet),
        ]
    }

    /// Check if a network is available
    pub fn has_network(&self, network: StellarNetwork) -> bool {
        self.networks.values().any(|c| c.network == network)
    }
}

/// Manager for multi-network RPC clients
pub struct MultiNetworkRpcManager {
    clients: HashMap<String, Arc<StellarRpcClient>>,
    primary_network: StellarNetwork,
}

impl MultiNetworkRpcManager {
    /// Create a new multi-network RPC manager
    pub fn new(primary_client: Arc<StellarRpcClient>) -> Self {
        let mut clients = HashMap::new();
        clients.insert("mainnet".to_string(), primary_client);

        Self {
            clients,
            primary_network: StellarNetwork::Mainnet,
        }
    }

    /// Add a client for a specific network
    pub fn add_client(&mut self, network: StellarNetwork, client: Arc<StellarRpcClient>) {
        let key = network.to_string();
        self.clients.insert(key, client);
    }

    /// Get a client for a specific network
    pub fn get_client(&self, network: StellarNetwork) -> Option<Arc<StellarRpcClient>> {
        self.clients.get(&network.to_string()).cloned()
    }

    /// Get the primary network client
    pub fn primary_client(&self) -> Option<Arc<StellarRpcClient>> {
        self.clients.get(&self.primary_network.to_string()).cloned()
    }

    /// Get all available clients
    pub fn all_clients(&self) -> Vec<(StellarNetwork, Arc<StellarRpcClient>)> {
        self.clients
            .iter()
            .filter_map(|(key, client)| {
                key.parse::<StellarNetwork>()
                    .ok()
                    .map(|net| (net, client.clone()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_network_config_creation() {
        let _guard = crate::lock_env_test();
        let config = MultiNetworkConfig::default_networks();
        assert!(config.has_network(StellarNetwork::Mainnet));
        assert!(config.has_network(StellarNetwork::Testnet));
        assert_eq!(config.primary_network, StellarNetwork::Mainnet);
    }

    #[test]
    fn test_network_context_creation() {
        let _guard = crate::lock_env_test();
        let context = NetworkContext::new(StellarNetwork::Mainnet);
        assert_eq!(context.network, StellarNetwork::Mainnet);
        assert_eq!(context.network_id(), "mainnet");
    }

    #[test]
    fn test_get_context() {
        let _guard = crate::lock_env_test();
        let config = MultiNetworkConfig::default_networks();
        let context = config.get_context(StellarNetwork::Testnet);
        assert!(context.is_some());
        assert_eq!(context.unwrap().network, StellarNetwork::Testnet);
    }

    #[test]
    fn test_available_networks() {
        let _guard = crate::lock_env_test();
        let config = MultiNetworkConfig::default_networks();
        let networks = config.available_networks();
        assert_eq!(networks.len(), 2);
    }
}
