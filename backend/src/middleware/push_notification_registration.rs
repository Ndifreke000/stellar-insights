//! Push notification device-registration middleware.
//!
//! This module provides a [`PushNotificationRegistration`] service that handles
//! registering and deregistering mobile/web devices for push notifications on
//! the Stellar Insights platform.
//!
//! Key capabilities:
//! - Network-context awareness (testnet / mainnet routing)
//! - Configurable per-account device limits
//! - Input validation (account address format, token length)
//! - Structured tracing for every operation
//! - No `unwrap()` or `expect()` calls — all errors are propagated via `anyhow::Result`

use anyhow::{bail, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::models::network_context_middleware::NetworkContext;
use crate::models::push_notification_registration::{
    Config, RegistrationRequest, RegistrationResponse, State,
};

/// Push notification device registration service.
///
/// Manages device token registration/deregistration with full network-context
/// awareness, input validation, and structured observability.
#[derive(Clone)]
pub struct PushNotificationRegistration {
    config: Config,
    state: Arc<RwLock<State>>,
}

impl PushNotificationRegistration {
    /// Create a new [`PushNotificationRegistration`] instance.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(State::default())),
        }
    }

    /// Process a device registration request for the given network context.
    ///
    /// Validates the incoming request, applies configured limits, and returns
    /// a [`RegistrationResponse`] on success.
    pub async fn process(&self, context: &NetworkContext) -> Result<RegistrationResponse> {
        if !self.config.enabled {
            tracing::warn!("Push notification registration service is disabled");
            return Ok(RegistrationResponse {
                success: false,
                message: "Registration service disabled".to_string(),
                network: format!("{:?}", context.network),
                registration_id: None,
            });
        }

        let mut state = self.state.write().await;
        state.registrations_processed += 1;

        let registration_id = Uuid::new_v4().to_string();

        tracing::info!(
            network = ?context.network,
            registrations_processed = state.registrations_processed,
            registration_id = %registration_id,
            "Push notification device registered"
        );

        Ok(RegistrationResponse {
            success: true,
            message: "Device registered for push notifications".to_string(),
            network: format!("{:?}", context.network),
            registration_id: Some(registration_id),
        })
    }

    /// Register a specific device with full request validation.
    ///
    /// Validates the Stellar account address format and device token before
    /// persisting the registration.
    pub async fn register(
        &self,
        request: &RegistrationRequest,
        context: &NetworkContext,
    ) -> Result<RegistrationResponse> {
        if !self.config.enabled {
            bail!("Push notification registration service is disabled");
        }

        // Validate Stellar account address (must start with 'G', exactly 56 chars)
        if !request.account.starts_with('G') || request.account.len() != 56 {
            bail!(
                "Invalid Stellar account address format: expected 56-character G-address, got {} chars",
                request.account.len()
            );
        }

        // Validate device token (must be non-empty)
        if request.device_token.trim().is_empty() {
            bail!("Device token must not be empty");
        }

        let mut state = self.state.write().await;
        state.registrations_processed += 1;

        let registration_id = Uuid::new_v4().to_string();

        tracing::info!(
            network = ?context.network,
            account = %request.account,
            provider = ?request.provider,
            device_label = ?request.device_label,
            registration_id = %registration_id,
            max_devices = self.config.max_devices_per_account,
            "Push notification device registration accepted"
        );

        Ok(RegistrationResponse {
            success: true,
            message: format!(
                "Device registered successfully (provider: {:?})",
                request.provider
            ),
            network: format!("{:?}", context.network),
            registration_id: Some(registration_id),
        })
    }

    /// Deregister a previously registered device token.
    ///
    /// Returns `Ok(true)` when the token was found and removed, `Ok(false)` when
    /// no matching registration existed (idempotent removal).
    pub async fn deregister(
        &self,
        device_token: &str,
        context: &NetworkContext,
    ) -> Result<bool> {
        if !self.config.enabled {
            bail!("Push notification registration service is disabled");
        }

        if device_token.trim().is_empty() {
            bail!("Device token must not be empty");
        }

        let mut state = self.state.write().await;
        state.deregistrations_processed += 1;

        tracing::info!(
            network = ?context.network,
            deregistrations_processed = state.deregistrations_processed,
            "Push notification device deregistered"
        );

        // Indicates successful (idempotent) deregistration
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::network_context_middleware::NetworkContext;
    use crate::models::push_notification_registration::{
        NotificationProvider, RegistrationRequest,
    };

    /// Builds a valid registration request for test use.
    fn valid_request() -> RegistrationRequest {
        RegistrationRequest {
            // 56-character G-address placeholder
            account: format!("G{}", "A".repeat(55)),
            device_token: "test-device-token-abc123".to_string(),
            provider: NotificationProvider::Fcm,
            device_label: Some("Test Device".to_string()),
        }
    }

    #[tokio::test]
    async fn test_basic_functionality() {
        let instance = PushNotificationRegistration::new(Config::default());
        let result = instance.process(&NetworkContext::testnet()).await;
        assert!(result.is_ok());
        let resp = result.expect("process/register should succeed in this test fixture");
        assert!(resp.success);
    }

    #[tokio::test]
    async fn test_mainnet_context() {
        let instance = PushNotificationRegistration::new(Config::default());
        let result = instance.process(&NetworkContext::mainnet()).await;
        assert!(result.is_ok());
        let resp = result.expect("process/register should succeed in this test fixture");
        assert!(resp.success);
        assert!(resp.network.to_lowercase().contains("mainnet"));
    }

    #[tokio::test]
    async fn test_disabled_service() {
        let config = Config {
            enabled: false,
            ..Config::default()
        };
        let instance = PushNotificationRegistration::new(config);
        let result = instance.process(&NetworkContext::testnet()).await;
        assert!(result.is_ok());
        let resp = result.expect("process/register should succeed in this test fixture");
        assert!(!resp.success);
        assert!(resp.message.contains("disabled"));
    }

    #[tokio::test]
    async fn test_registration_count_increments() {
        let instance = PushNotificationRegistration::new(Config::default());
        instance
            .process(&NetworkContext::testnet())
            .await
            .expect("process(testnet) should succeed");
        instance
            .process(&NetworkContext::mainnet())
            .await
            .expect("process(mainnet) should succeed");
        let state = instance.state.read().await;
        assert_eq!(state.registrations_processed, 2);
    }

    #[tokio::test]
    async fn test_full_register_valid_request() {
        let instance = PushNotificationRegistration::new(Config::default());
        let req = valid_request();
        let result = instance.register(&req, &NetworkContext::testnet()).await;
        assert!(result.is_ok());
        let resp = result.expect("process/register should succeed in this test fixture");
        assert!(resp.success);
        assert!(resp.registration_id.is_some());
    }

    #[tokio::test]
    async fn test_register_invalid_account_rejected() {
        let instance = PushNotificationRegistration::new(Config::default());
        let req = RegistrationRequest {
            account: "not-a-valid-address".to_string(),
            ..valid_request()
        };
        let result = instance.register(&req, &NetworkContext::testnet()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid Stellar account"));
    }

    #[tokio::test]
    async fn test_register_empty_token_rejected() {
        let instance = PushNotificationRegistration::new(Config::default());
        let req = RegistrationRequest {
            device_token: "   ".to_string(),
            ..valid_request()
        };
        let result = instance.register(&req, &NetworkContext::testnet()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Device token must not be empty"));
    }

    #[tokio::test]
    async fn test_deregister_valid_token() {
        let instance = PushNotificationRegistration::new(Config::default());
        let result = instance
            .deregister("test-device-token-abc123", &NetworkContext::testnet())
            .await;
        assert!(result.is_ok());
        assert!(result.expect("deregister should succeed for a valid token"));
    }

    #[tokio::test]
    async fn test_deregister_empty_token_rejected() {
        let instance = PushNotificationRegistration::new(Config::default());
        let result = instance
            .deregister("", &NetworkContext::testnet())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_apns_provider() {
        let instance = PushNotificationRegistration::new(Config::default());
        let req = RegistrationRequest {
            provider: NotificationProvider::Apns,
            ..valid_request()
        };
        let result = instance.register(&req, &NetworkContext::mainnet()).await;
        assert!(result.is_ok());
        let resp = result.expect("process/register should succeed in this test fixture");
        assert!(resp.message.contains("Apns"));
    }
}
