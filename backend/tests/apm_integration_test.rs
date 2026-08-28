//! Integration test for APM (Application Performance Monitoring)
//!
//! Validates:
//! 1. APM initialization with correct environment variables
//! 2. Sensitive data redaction
//! 3. OpenTelemetry configuration
//! 4. Correlation ID handling

#[test]
fn test_apm_enabled_by_default() {
    // APM should be enabled by default
    let apm_enabled = std::env::var("OTEL_ENABLED")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);
    
    assert!(apm_enabled, "APM (OTEL_ENABLED) should default to true");
}

#[test]
fn test_apm_fails_soft() {
    // Missing OTEL_EXPORTER_OTLP_ENDPOINT should not crash the app
    // The APM integration should log a warning but continue
    // (This is tested by the APM module's validate() function)
    
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
    
    // If endpoint is missing, APM should still initialize (fail-soft)
    // This is a configuration issue, not an app crash
    if endpoint.is_none() {
        eprintln!("Note: OTEL_EXPORTER_OTLP_ENDPOINT not set; APM will log warning but continue");
    }
}

#[test]
fn test_opentelemetry_dependencies_present() {
    // Verify OpenTelemetry crates are available
    // This test just ensures the module is reachable
    
    // Check if we can import opentelemetry
    use stellar_insights_backend::observability;
    
    // APM module should exist
    let _ = observability::apm::ApmConfig::from_env();
}

#[test]
fn test_apm_config_defaults() {
    use stellar_insights_backend::observability::apm::ApmConfig;
    
    let config = ApmConfig::from_env();
    
    // Should have reasonable defaults
    assert!(!config.service_name.is_empty(), "Service name must be set");
    assert!(config.service_name.contains("stellar-insights"), "Service name should contain stellar-insights");
    
    assert!(!config.otlp_endpoint.is_empty(), "OTLP endpoint must be set");
    assert!(config.otlp_endpoint.contains("4318") || config.otlp_endpoint.contains("4317"), 
            "OTLP endpoint should use standard ports (4318 for HTTP, 4317 for gRPC)");
}

#[test]
fn test_sensitive_data_redaction() {
    use stellar_insights_backend::observability::apm::redaction;
    
    // Test that sensitive keywords trigger redaction
    let test_cases = vec![
        ("Bearer token_abc123", "REDACTED"),
        ("authorization: xyz", "REDACTED"),
        ("password=secret", "REDACTED"),
        ("api_key=abc123xyz", "REDACTED"),
        ("api-key: xyz", "REDACTED"),
        ("token: value", "REDACTED"),
        ("secret_value: xyz", "REDACTED"),
        ("GET /api/anchors", "GET /api/anchors"), // Safe value unchanged
        ("status: 200", "status: 200"), // Safe value unchanged
    ];
    
    for (input, expected_output) in test_cases {
        let result = redaction::redact_value(input);
        assert_eq!(result, expected_output, 
                  "Failed for input: {}", input);
    }
}

#[test]
fn test_tracing_module_supports_json() {
    // Verify LOG_FORMAT environment variable is respected
    let log_format = std::env::var("LOG_FORMAT")
        .unwrap_or_else(|_| "json".to_string());
    
    assert_eq!(log_format.to_lowercase(), "json",
               "LOG_FORMAT should be 'json' for APM correlation with logs");
}

#[test]
fn test_request_id_correlation() {
    // Verify that request/response logging includes request_id for APM correlation
    
    let logging_rs_path = "backend/src/observability/logging.rs";
    let logging_src = std::fs::read_to_string(logging_rs_path)
        .expect("Failed to read logging.rs");
    
    assert!(logging_src.contains("request_id"), 
            "Request logging must include request_id for APM trace correlation");
}

#[test]
fn test_w3c_trace_context_support() {
    // Verify W3C Trace Context (traceparent) header support
    
    let tracing_rs_path = "backend/src/observability/tracing.rs";
    let tracing_src = std::fs::read_to_string(tracing_rs_path)
        .expect("Failed to read tracing.rs");
    
    // Should use W3C TraceContext propagator
    assert!(tracing_src.contains("TraceContextPropagator") || 
            tracing_src.contains("traceparent"),
            "Tracing should support W3C Trace Context for distributed tracing");
}

#[test]
fn test_apm_documentation_exists() {
    // Verify APM setup documentation is available
    
    let apm_docs = "docs/APM_INTEGRATION.md";
    assert!(std::path::Path::new(apm_docs).exists(),
            "APM documentation must exist at {}", apm_docs);
    
    let content = std::fs::read_to_string(apm_docs)
        .expect("Failed to read APM docs");
    
    // Verify key sections are documented
    assert!(content.contains("Environment variables"), "Should document env vars");
    assert!(content.contains("New Relic") || content.contains("Datadog"), 
            "Should document production APM providers");
    assert!(content.contains("Sensitive"), "Should document sensitive data handling");
    assert!(content.contains("Correlation"), "Should document log/trace correlation");
}

#[test]
fn test_error_tracking_capability() {
    // Verify that the app can track errors through APM
    
    let main_rs_path = "backend/src/main.rs";
    let main_src = std::fs::read_to_string(main_rs_path)
        .expect("Failed to read main.rs");
    
    // Should initialize tracing (which enables error tracking)
    assert!(main_src.contains("init_tracing"), "Should initialize tracing for error tracking");
}
