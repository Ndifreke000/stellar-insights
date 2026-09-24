use serde::{Deserialize, Serialize};

/// Supported push notification providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationProvider {
    /// Apple Push Notification service.
    Apns,
    /// Firebase Cloud Messaging.
    Fcm,
    /// Web Push (VAPID).
    WebPush,
}

impl Default for NotificationProvider {
    fn default() -> Self {
        Self::Fcm
    }
}

/// Configuration for the push notification registration service.
#[derive(Clone, Debug)]
pub struct Config {
    /// Whether registration is open (feature flag).
    pub enabled: bool,
    /// Maximum number of devices a single account may register.
    pub max_devices_per_account: u32,
    /// Token time-to-live in seconds (0 = never expire).
    pub token_ttl_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            max_devices_per_account: 10,
            token_ttl_secs: 0,
        }
    }
}

/// A request to register a device for push notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationRequest {
    /// Stellar account address of the subscribing user.
    pub account: String,
    /// Provider-issued device token / registration ID.
    pub device_token: String,
    /// Push notification provider.
    pub provider: NotificationProvider,
    /// Optional human-readable device label (e.g. "iPhone 15 Pro").
    pub device_label: Option<String>,
}

/// Response returned after a successful registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResponse {
    /// Whether the registration was accepted.
    pub success: bool,
    /// Human-readable status message.
    pub message: String,
    /// Network on which the registration was processed.
    pub network: String,
    /// Unique registration ID assigned by the service.
    pub registration_id: Option<String>,
}

impl Default for RegistrationResponse {
    fn default() -> Self {
        Self {
            success: true,
            message: "Device registered for push notifications".to_string(),
            network: String::new(),
            registration_id: None,
        }
    }
}

/// Internal mutable state tracked across registration requests.
#[derive(Debug, Default)]
pub struct State {
    /// Total number of registrations processed since service start.
    pub registrations_processed: u64,
    /// Total number of deregistrations processed since service start.
    pub deregistrations_processed: u64,
}
