use crate::network::{NetworkConfig, StellarNetwork};
use axum::{
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub network: StellarNetwork,
    pub display_name: String,
    pub rpc_url: String,
    pub horizon_url: String,
    pub network_passphrase: String,
    pub color: String,
    pub is_mainnet: bool,
    pub is_testnet: bool,
}

#[derive(Debug, Deserialize)]
pub struct SwitchNetworkRequest {
    pub network: StellarNetwork,
}

#[derive(Debug, Serialize)]
pub struct SwitchNetworkResponse {
    pub success: bool,
    pub message: String,
    pub network_info: NetworkInfo,
}

/// Get current network information
pub async fn get_network_info() -> Result<Json<NetworkInfo>, StatusCode> {
    let network_config = NetworkConfig::from_env();

    let network_info = NetworkInfo {
        network: network_config.network,
        display_name: network_config.display_name().to_string(),
        rpc_url: network_config.rpc_url.clone(),
        horizon_url: network_config.horizon_url.clone(),
        network_passphrase: network_config.network_passphrase.clone(),
        color: network_config.color().to_string(),
        is_mainnet: network_config.is_mainnet(),
        is_testnet: network_config.is_testnet(),
    };

    Ok(Json(network_info))
}

/// Get available networks
pub async fn get_available_networks() -> Json<Vec<NetworkInfo>> {
    let networks = vec![
        NetworkConfig::for_network(StellarNetwork::Mainnet),
        NetworkConfig::for_network(StellarNetwork::Testnet),
    ];

    let network_infos = networks
        .into_iter()
        .map(|config| NetworkInfo {
            network: config.network,
            display_name: config.display_name().to_string(),
            rpc_url: config.rpc_url.clone(),
            horizon_url: config.horizon_url.clone(),
            network_passphrase: config.network_passphrase.clone(),
            color: config.color().to_string(),
            is_mainnet: config.is_mainnet(),
            is_testnet: config.is_testnet(),
        })
        .collect();

    Json(network_infos)
}

/// Switch network (Note: This is a placeholder - actual switching would require server restart)
pub async fn switch_network(
    Json(request): Json<SwitchNetworkRequest>,
) -> Result<Json<SwitchNetworkResponse>, StatusCode> {
    info!("Network switch requested to: {}", request.network);

    // In a real implementation, you might want to:
    // 1. Update environment variables
    // 2. Restart services with new network configuration
    // 3. Clear network-specific caches

    // For now, we'll return information about what the switch would do
    let target_config = NetworkConfig::for_network(request.network);

    let response = SwitchNetworkResponse {
        success: false, // Set to false since we're not actually switching
        message: format!(
            "Network switch to {} requested. Server restart required to apply changes.",
            target_config.display_name()
        ),
        network_info: NetworkInfo {
            network: target_config.network,
            display_name: target_config.display_name().to_string(),
            rpc_url: target_config.rpc_url.clone(),
            horizon_url: target_config.horizon_url.clone(),
            network_passphrase: target_config.network_passphrase.clone(),
            color: target_config.color().to_string(),
            is_mainnet: target_config.is_mainnet(),
            is_testnet: target_config.is_testnet(),
        },
    };

    warn!(
        "Network switch to {} requires server restart - not implemented in this endpoint",
        request.network
    );

    Ok(Json(response))
}

/// Create network routes
pub fn routes() -> Router {
    Router::new()
        .route("/info", get(get_network_info))
        .route("/available", get(get_available_networks))
        .route("/switch", post(switch_network))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_get_network_info() {
        let result = get_network_info().await;
        assert!(result.is_ok());

        let network_info = result.unwrap().0;
        assert!(!network_info.display_name.is_empty());
        assert!(!network_info.rpc_url.is_empty());
        assert!(!network_info.horizon_url.is_empty());
    }

    #[tokio::test]
    async fn test_get_available_networks() {
        let result = get_available_networks().await;
        let networks = result.0;

        assert_eq!(networks.len(), 2);
        assert!(networks.iter().any(|n| n.is_mainnet));
        assert!(networks.iter().any(|n| n.is_testnet));
    }

    #[tokio::test]
    async fn test_switch_network() {
        let request = SwitchNetworkRequest {
            network: StellarNetwork::Testnet,
        };

        let result = switch_network(Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(!response.success); // Should be false since we don't actually switch
        assert!(response.message.contains("restart required"));
        assert_eq!(response.network_info.network, StellarNetwork::Testnet);
    }
}
