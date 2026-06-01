use std::sync::Arc;
use anyhow::Result;
use axum::{Router, middleware, extract::{Request, State}, middleware::Next};

use crate::apm::{ApmConfig, ApmManager};

/// APM Integration helper for easy setup
pub struct ApmIntegration {
    pub manager: Arc<ApmManager>,
}

impl ApmIntegration {
    /// Initialize APM from environment variables
    pub fn from_env() -> Result<Self> {
        let config = ApmConfig::default();
        let manager = Arc::new(ApmManager::new(config)?);
        
        Ok(Self { manager })
    }

    /// Initialize APM with custom configuration
    pub fn with_config(config: ApmConfig) -> Result<Self> {
        let manager = Arc::new(ApmManager::new(config)?);
        
        Ok(Self { manager })
    }

    /// Add APM middleware to an Axum router
    pub fn add_middleware<S>(& self, router: Router<S>) -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        let apm = self.manager.clone();
        router.layer(axum::middleware::from_fn_with_state(
            apm,
            |State(apm): State<Arc<ApmManager>>, req: Request, next: Next| async move {
                crate::middleware::ApmMiddleware::track_http_request(State(apm), req, next).await
            },
        ))
    }

    /// Get the APM manager instance
    pub fn manager(&self) -> Arc<ApmManager> {
        self.manager.clone()
    }

    /// Shutdown APM gracefully
    pub async fn shutdown(&self) -> Result<()> {
        self.manager.shutdown().await
    }
}

/// Helper macro for instrumenting functions with APM
#[macro_export]
macro_rules! apm_instrument {
    ($apm:expr, $name:expr, $body:expr) => {{
        let _span = tracing::info_span!($name);
        let _enter = _span.enter();
        $body
    }};
}

/// Helper macro for tracking database operations
#[macro_export]
macro_rules! apm_track_db {
    ($apm:expr, $operation:expr, $table:expr, $body:expr) => {{
        $crate::middleware::ApmMiddleware::track_database_operation(
            &$apm,
            $operation,
            Some($table),
            async { $body },
        )
        .await
    }};
}

/// Helper macro for tracking Stellar RPC operations
#[macro_export]
macro_rules! apm_track_stellar {
    ($apm:expr, $operation:expr, $endpoint:expr, $body:expr) => {{
        $crate::middleware::ApmMiddleware::track_stellar_operation(
            &$apm,
            $operation,
            $endpoint,
            async { $body },
        )
        .await
    }};
}

/// Helper macro for tracking background jobs
#[macro_export]
macro_rules! apm_track_job {
    ($apm:expr, $job_name:expr, $job_type:expr, $body:expr) => {{
        $crate::middleware::ApmMiddleware::track_background_job(
            &$apm,
            $job_name,
            $job_type,
            async { $body },
        )
        .await
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apm_integration_from_env() {
        // This test requires environment variables to be set
        // In CI/CD, ensure APM_ENABLED=false for testing
        std::env::set_var("APM_ENABLED", "false");
        let result = ApmIntegration::from_env();
        assert!(result.is_ok());
    }

    #[test]
    fn test_apm_integration_with_config() {
        let config = crate::apm::ApmConfig {
            service_name: "test-service".to_string(),
            service_version: "1.0.0".to_string(),
            environment: "test".to_string(),
            enabled: false,
            platform: crate::apm::ApmPlatform::OpenTelemetry,
            sample_rate: 1.0,
            otlp_endpoint: None,
            new_relic_license_key: None,
            datadog_api_key: None,
        };

        let result = ApmIntegration::with_config(config);
        assert!(result.is_ok());
    }
}
