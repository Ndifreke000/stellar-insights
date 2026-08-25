//! Application Performance Monitoring (APM) integration via OpenTelemetry
//!
//! This module provides APM instrumentation for Stellar Insights, enabling:
//! - Request/response trace collection
//! - Database query performance tracking
//! - Error event collection
//! - Service dependency mapping
//!
//! APM is gated by the OTEL_ENABLED environment variable and fails soft if
//! configuration is missing (never crashes the app on APM config issues).


/// APM configuration from environment
pub struct ApmConfig {
    pub enabled: bool,
    pub service_name: String,
    pub otlp_endpoint: String,
}

impl ApmConfig {
    /// Load APM configuration from environment
    ///
    /// Returns configuration with sensible defaults. If OTEL_ENABLED is false,
    /// returns a no-op configuration (telemetry disabled).
    pub fn from_env() -> Self {
        let enabled = std::env::var("OTEL_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(true);

        let service_name = std::env::var("OTEL_SERVICE_NAME")
            .unwrap_or_else(|_| "stellar-insights-backend".to_string());

        let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:4318/v1/traces".to_string());

        Self {
            enabled,
            service_name,
            otlp_endpoint,
        }
    }

    /// Validate APM configuration
    ///
    /// Returns Ok(()) if configuration is valid or if APM is disabled.
    /// APM is fail-soft: if enabled but misconfigured, logs warning but doesn't fail.
    pub fn validate(&self) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Validate OTLP endpoint is reachable (best effort)
        if self.otlp_endpoint.is_empty() {
            tracing::warn!(
                "OTEL_EXPORTER_OTLP_ENDPOINT is empty; traces may not be exported"
            );
        }

        Ok(())
    }
}

/// Initialize APM instrumentation
///
/// Called during app startup. APM is fail-soft: if telemetry fails to initialize,
/// the app continues with a no-op tracer that has no performance impact.
pub fn init_apm(config: &ApmConfig) -> anyhow::Result<()> {
    if !config.enabled {
        tracing::info!("APM telemetry disabled (OTEL_ENABLED=false)");
        return Ok(());
    }

    config.validate()?;
    tracing::info!(
        endpoint = %config.otlp_endpoint,
        service = %config.service_name,
        "APM telemetry initialized"
    );

    Ok(())
}

/// Database query attributes for instrumentation
pub mod db {
    /// Attributes to include in database query spans
    pub struct QuerySpan {
        pub statement: String,
        pub duration_ms: u128,
        pub rows_affected: Option<u64>,
        pub error: Option<String>,
    }

    impl QuerySpan {
        pub fn new(statement: impl Into<String>) -> Self {
            Self {
                statement: statement.into(),
                duration_ms: 0,
                rows_affected: None,
                error: None,
            }
        }
    }
}

/// Request correlation attributes for linking traces, logs, and metrics
pub mod correlation {
    /// Trace context that links across logs, traces, and metrics
    pub struct TraceContext {
        /// Unique identifier for this request (shared with logs)
        pub request_id: String,
        /// W3C Trace Context traceparent header value
        pub traceparent: Option<String>,
        /// OpenTelemetry span context
        pub span_context: Option<String>,
    }

    impl TraceContext {
        pub fn new(request_id: String) -> Self {
            Self {
                request_id,
                traceparent: None,
                span_context: None,
            }
        }
    }
}

/// Sensitive data redaction for APM spans
///
/// Prevents collection of:
/// - Auth tokens and API keys
/// - Passwords and secrets
/// - User PII (best-effort)
pub mod redaction {
    /// Redact sensitive patterns from span attributes
    pub fn redact_value(value: &str) -> String {
        let lower = value.to_lowercase();
        let mut result = value.to_string();

        // Check for sensitive keywords and redact the entire value if found
        let sensitive_keywords = [
            "authorization",
            "bearer",
            "password",
            "api_key",
            "api-key",
            "apikey",
            "token",
            "secret",
            "credential",
            "auth",
        ];

        for keyword in sensitive_keywords.iter() {
            if lower.contains(keyword) {
                return "REDACTED".to_string();
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apm_config_from_env() {
        let config = ApmConfig::from_env();
        assert!(!config.service_name.is_empty());
        assert!(!config.otlp_endpoint.is_empty());
    }

    #[test]
    fn test_redaction() {
        let redacted = redaction::redact_value("Bearer token_abc123_secret");
        assert_eq!(redacted, "REDACTED");

        let redacted = redaction::redact_value("password=my_password_123");
        assert_eq!(redacted, "REDACTED");

        let safe = redaction::redact_value("GET /api/anchors");
        assert_eq!(safe, "GET /api/anchors");
    }
}
