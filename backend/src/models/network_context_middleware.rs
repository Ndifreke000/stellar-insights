use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    Testnet,
    Mainnet,
}

#[derive(Debug, Clone)]
pub struct NetworkContext {
    pub network: Network,
}

impl NetworkContext {
    pub fn testnet() -> Self {
        Self {
            network: Network::Testnet,
        }
    }

    pub fn mainnet() -> Self {
        Self {
            network: Network::Mainnet,
        }
    }
}
