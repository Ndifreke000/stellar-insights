//! Environment configuration validation and loading
//!
//! This module provides validation for required environment variables
//! and ensures the application fails fast with clear error messages
//! if critical configuration is missing.

use anyhow::{Context, Result};
use std::env;

/// Required environment variables that must be set
const REQUIRED_VARS: &[&str] = &["DATABASE_URL"];

/// Environment variables that should be validated if present
const VALIDATED_VARS: &[(&str, fn(&str) -> bool)] = &[
    ("SERVER_PORT", validate_port),
    ("DB_POOL_MAX_CONNECTIONS", validate_positive_number),
    ("DB_POOL_MIN_CONNECTIONS", validate_positive_number),
    ("RPC_MAX_RECORDS_PER_REQUEST", validate_positive_number),
    ("RPC_MAX_TOTAL_RECORDS", validate_positive_number),
    ("RPC_PAGINATION_DELAY_MS", validate_positive_number),
];

/// Validates all required environment variables are set
pub fn validate_env() -> Result<()> {
    let mut errors = Vec::new();

    // Check required variables
    for var in REQUIRED_VARS {
        if env::var(var).is_err() {
            errors.push(format!("Missing required environment variable: {}", var));
        }
    }

    // Validate format of present variables
    for (var, validator) in VALIDATED_VARS {
        if let Ok(value) = env::var(var) {
            if !validator(&value) {
                errors.push(format!(
                    "Invalid value for environment variable {}: '{}'",
                    var, value
                ));
            }
        }
    }

    if !errors.is_empty() {
        anyhow::bail!(
            "Environment configuration errors:\n  - {}",
            errors.join("\n  - ")
        );
    }

    Ok(())
}

/// Logs all configured environment variables (without sensitive values)
pub fn log_env_config() {
    tracing::info!("Environment configuration:");

    // Database
    if let Ok(db_url) = env::var("DATABASE_URL") {
        let sanitized = sanitize_database_url(&db_url);
        tracing::info!("  DATABASE_URL: {}", sanitized);
    }

    // Server
    log_var("SERVER_HOST");
    log_var("SERVER_PORT");
    log_var("RUST_LOG");

    // Redis
    if let Ok(redis_url) = env::var("REDIS_URL") {
        let sanitized = sanitize_url(&redis_url);
        tracing::info!("  REDIS_URL: {}", sanitized);
    }

    // Network
    log_var("STELLAR_NETWORK");
    log_var("RPC_MOCK_MODE");

    // Pool config
    log_var("DB_POOL_MAX_CONNECTIONS");
    log_var("DB_POOL_MIN_CONNECTIONS");
    log_var("DB_POOL_CONNECT_TIMEOUT_SECONDS");
    log_var("DB_POOL_IDLE_TIMEOUT_SECONDS");
    log_var("DB_POOL_MAX_LIFETIME_SECONDS");

    // CORS
    log_var("CORS_ALLOWED_ORIGINS");

    // Price feed (don't log API key)
    log_var("PRICE_FEED_PROVIDER");
    if env::var("PRICE_FEED_API_KEY").is_ok() {
        tracing::info!("  PRICE_FEED_API_KEY: [REDACTED]");
    }

    // RPC Pagination
    log_var("RPC_MAX_RECORDS_PER_REQUEST");
    log_var("RPC_MAX_TOTAL_RECORDS");
    log_var("RPC_PAGINATION_DELAY_MS");
}

/// Helper to log a single environment variable
fn log_var(name: &str) {
    if let Ok(value) = env::var(name) {
        tracing::info!("  {}: {}", name, value);
    }
}

/// Sanitize database URL to hide credentials
fn sanitize_database_url(url: &str) -> String {
    if url.starts_with("sqlite:") {
        return url.to_string();
    }

    // For postgres/mysql URLs, hide password
    if let Some(at_pos) = url.rfind('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            if let Some(scheme_end) = url.find("://") {
                let scheme = &url[..scheme_end + 3];
                let user = &url[scheme_end + 3..colon_pos];
                let host_and_db = &url[at_pos..];
                return format!("{}{}:****{}", scheme, user, host_and_db);
            }
        }
    }

    "[REDACTED]".to_string()
}

/// Sanitize generic URL to hide credentials
fn sanitize_url(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end + 3];
            let host_and_path = &url[at_pos + 1..];
            return format!("{}****@{}", scheme, host_and_path);
        }
    }
    url.to_string()
}

/// Validate port number
fn validate_port(value: &str) -> bool {
    value.parse::<u16>().is_ok()
}

/// Validate positive number
fn validate_positive_number(value: &str) -> bool {
    value.parse::<u32>().map(|n| n > 0).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_sqlite_url() {
        let url = "sqlite:./stellar_insights.db";
        assert_eq!(sanitize_database_url(url), url);
    }

    #[test]
    fn test_sanitize_postgres_url() {
        let url = "postgresql://user:secret123@localhost:5432/db";
        let sanitized = sanitize_database_url(url);
        assert_eq!(sanitized, "postgresql://user:****@localhost:5432/db");
        assert!(!sanitized.contains("secret123"));
    }

    #[test]
    fn test_sanitize_redis_url() {
        let url = "redis://user:pass@localhost:6379";
        let sanitized = sanitize_url(url);
        assert_eq!(sanitized, "redis://****@localhost:6379");
        assert!(!sanitized.contains("pass"));
    }

    #[test]
    fn test_validate_port() {
        assert!(validate_port("8080"));
        assert!(validate_port("80"));
        assert!(validate_port("65535"));
        assert!(!validate_port("0"));
        assert!(!validate_port("70000"));
        assert!(!validate_port("abc"));
        assert!(!validate_port("-1"));
    }

    #[test]
    fn test_validate_positive_number() {
        assert!(validate_positive_number("1"));
        assert!(validate_positive_number("100"));
        assert!(!validate_positive_number("0"));
        assert!(!validate_positive_number("-1"));
        assert!(!validate_positive_number("abc"));
    }
}
