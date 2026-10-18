//! SEP-10 authentication middleware for mobile clients.
//!
//! This module provides a [`Sep10ForMobile`] service that wraps the core SEP-10
//! challenge/verify flow with additional handling required by mobile clients:
//!
//! - Network-context awareness (testnet / mainnet routing)
//! - Configurable challenge expiry and retry limits
//! - Structured tracing for every operation
//! - No `unwrap()` or `expect()` calls — all errors are propagated via `anyhow::Result`

use anyhow::{bail, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::network_context_middleware::NetworkContext;
use crate::models::sep10_for_mobile::{Config, Response, State};

/// SEP-10 mobile authentication service.
///
/// Wraps the standard SEP-10 challenge/verify flow with mobile-specific
/// concerns: network-context routing, retry tracking, and structured logging.
#[derive(Clone)]
pub struct Sep10ForMobile {
    config: Config,
    state: Arc<RwLock<State>>,
}

impl Sep10ForMobile {
    /// Create a new [`Sep10ForMobile`] instance with the given configuration.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(State::default())),
        }
    }

    /// Issue a SEP-10 challenge for the given network context.
    ///
    /// Returns a [`Response`] that can be forwarded directly to the mobile client.
    /// Errors if the service is disabled or an unrecoverable condition is detected.
    pub async fn process(&self, context: &NetworkContext) -> Result<Response> {
        if !self.config.enabled {
            tracing::warn!("SEP-10 mobile service is disabled");
            return Ok(Response {
                success: false,
                message: "SEP-10 mobile service disabled".to_string(),
                network: format!("{:?}", context.network),
            });
        }

        let mut state = self.state.write().await;

        if state.verifications_failed >= u64::from(self.config.max_retries) {
            tracing::error!(
                network = ?context.network,
                failed_attempts = state.verifications_failed,
                max_retries = self.config.max_retries,
                "SEP-10 mobile: maximum retry limit reached"
            );
            bail!(
                "Maximum SEP-10 retry limit ({}) reached for network {:?}",
                self.config.max_retries,
                context.network
            );
        }

        state.challenges_issued += 1;

        tracing::info!(
            network = ?context.network,
            challenges_issued = state.challenges_issued,
            challenge_expiry_secs = self.config.challenge_expiry_secs,
            "SEP-10 mobile challenge issued"
        );

        Ok(Response {
            success: true,
            message: format!(
                "SEP-10 challenge issued (expires in {}s)",
                self.config.challenge_expiry_secs
            ),
            network: format!("{:?}", context.network),
        })
    }

    /// Record a successful verification result.
    ///
    /// Call this after the client returns a signed challenge and the server
    /// has confirmed the signature is valid.
    pub async fn record_verification_success(&self, context: &NetworkContext) -> Result<Response> {
        if !self.config.enabled {
            bail!("SEP-10 mobile service is disabled");
        }

        let mut state = self.state.write().await;
        state.verifications_succeeded += 1;

        tracing::info!(
            network = ?context.network,
            verifications_succeeded = state.verifications_succeeded,
            "SEP-10 mobile verification succeeded"
        );

        Ok(Response {
            success: true,
            message: "SEP-10 verification accepted".to_string(),
            network: format!("{:?}", context.network),
        })
    }

    /// Record a failed verification attempt.
    ///
    /// Increments the failure counter. Once the counter reaches `max_retries`
    /// subsequent calls to [`process`](Self::process) will return an error.
    pub async fn record_verification_failure(&self, context: &NetworkContext) -> Result<()> {
        let mut state = self.state.write().await;
        state.verifications_failed += 1;

        tracing::warn!(
            network = ?context.network,
            verifications_failed = state.verifications_failed,
            max_retries = self.config.max_retries,
            "SEP-10 mobile verification failed"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::network_context_middleware::NetworkContext;

    #[tokio::test]
    async fn test_basic_functionality() {
        let instance = Sep10ForMobile::new(Config::default());
        let result = instance.process(&NetworkContext::testnet()).await;
        assert!(result.is_ok());
        let resp = result.expect("process should succeed in this test fixture");
        assert!(resp.success);
    }

    #[tokio::test]
    async fn test_mainnet_context() {
        let instance = Sep10ForMobile::new(Config::default());
        let result = instance.process(&NetworkContext::mainnet()).await;
        assert!(result.is_ok());
        let resp = result.expect("process should succeed in this test fixture");
        assert!(resp.success);
        assert!(resp.network.to_lowercase().contains("mainnet"));
    }

    #[tokio::test]
    async fn test_disabled_service() {
        let config = Config {
            enabled: false,
            ..Config::default()
        };
        let instance = Sep10ForMobile::new(config);
        let result = instance.process(&NetworkContext::testnet()).await;
        assert!(result.is_ok());
        let resp = result.expect("process should succeed in this test fixture");
        assert!(!resp.success);
        assert!(resp.message.contains("disabled"));
    }

    #[tokio::test]
    async fn test_challenge_count_increments() {
        let instance = Sep10ForMobile::new(Config::default());
        instance
            .process(&NetworkContext::testnet())
            .await
            .expect("process(testnet) should succeed");
        instance
            .process(&NetworkContext::mainnet())
            .await
            .expect("process(mainnet) should succeed");
        let state = instance.state.read().await;
        assert_eq!(state.challenges_issued, 2);
    }

    #[tokio::test]
    async fn test_verification_success_tracking() {
        let instance = Sep10ForMobile::new(Config::default());
        // Issue challenge first
        instance
            .process(&NetworkContext::testnet())
            .await
            .expect("process(testnet) should succeed");
        // Record successful verification
        let result = instance
            .record_verification_success(&NetworkContext::testnet())
            .await;
        assert!(result.is_ok());
        let state = instance.state.read().await;
        assert_eq!(state.verifications_succeeded, 1);
    }

    #[tokio::test]
    async fn test_max_retry_limit_enforced() {
        let config = Config {
            max_retries: 2,
            ..Config::default()
        };
        let instance = Sep10ForMobile::new(config);
        // Exhaust retries
        instance
            .record_verification_failure(&NetworkContext::testnet())
            .await
            .expect("first record_verification_failure should succeed");
        instance
            .record_verification_failure(&NetworkContext::testnet())
            .await
            .expect("second record_verification_failure should succeed");
        // Next process call should fail
        let result = instance.process(&NetworkContext::testnet()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_challenge_expiry_reflected_in_response() {
        let config = Config {
            challenge_expiry_secs: 120,
            ..Config::default()
        };
        let instance = Sep10ForMobile::new(config);
        let resp = instance
            .process(&NetworkContext::testnet())
            .await
            .expect("process should succeed with a configured challenge expiry");
        assert!(resp.message.contains("120s"));
    }
}
