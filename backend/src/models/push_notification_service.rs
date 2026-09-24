use serde::{Deserialize, Serialize};

/// Configuration for the push notification service.
#[derive(Clone, Debug)]
pub struct Config {
    pub enabled: bool,
    pub max_retries: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 3,
        }
    }
}

/// Represents the outcome of a push notification dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub success: bool,
    pub message: String,
    pub network: String,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            success: true,
            message: "Notification processed".to_string(),
            network: String::new(),
        }
    }
}

/// Internal mutable state tracked across requests.
#[derive(Debug, Default)]
pub struct State {
    pub notifications_sent: u64,
}
