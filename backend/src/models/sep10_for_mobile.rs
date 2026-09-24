use serde::{Deserialize, Serialize};

/// Configuration for SEP-10 mobile authentication.
#[derive(Clone, Debug)]
pub struct Config {
    /// Whether mobile SEP-10 is enabled.
    pub enabled: bool,
    /// Maximum number of challenge retries before locking out.
    pub max_retries: u32,
    /// Challenge expiry in seconds.
    pub challenge_expiry_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 3,
            challenge_expiry_secs: 300,
        }
    }
}

/// Represents the outcome of a SEP-10 mobile authentication operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Human-readable status message.
    pub message: String,
    /// Network on which the operation was performed.
    pub network: String,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            success: true,
            message: "SEP-10 mobile challenge processed".to_string(),
            network: String::new(),
        }
    }
}

/// Internal mutable state tracked across SEP-10 mobile requests.
#[derive(Debug, Default)]
pub struct State {
    /// Total number of challenges issued since service start.
    pub challenges_issued: u64,
    /// Total number of successful verifications since service start.
    pub verifications_succeeded: u64,
    /// Total number of failed verification attempts since service start.
    pub verifications_failed: u64,
}
