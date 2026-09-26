//! Environment configuration validation and loading
//!
//! This module provides validation for required environment variables
//! and ensures the application fails fast with clear error messages
//! if critical configuration is missing.

use anyhow::Result;
use std::env;

/// Required environment variables that must be set
const REQUIRED_VARS: &[&str] = &["DATABASE_URL", "ENCRYPTION_KEY", "JWT_SECRET"];

/// Environment variables that should be validated if present
const VALIDATED_VARS: &[(&str, fn(&str) -> bool)] = &[
    ("SERVER_PORT", validate_port),
    ("DB_POOL_MAX_CONNECTIONS", validate_db_pool_max_connections),
    ("DB_POOL_MIN_CONNECTIONS", validate_db_pool_min_connections),
    ("DB_POOL_CONNECT_TIMEOUT_SECONDS", validate_db_pool_connect_timeout),
    ("DB_POOL_IDLE_TIMEOUT_SECONDS", validate_db_pool_idle_timeout),
    ("DB_POOL_MAX_LIFETIME_SECONDS", validate_db_pool_max_lifetime),
    ("DB_BUSY_TIMEOUT_MS", validate_db_busy_timeout),
    ("DB_WRITE_POOL_MAX_CONNECTIONS", validate_db_write_pool_max_connections),
    ("RPC_MAX_RECORDS_PER_REQUEST", validate_rpc_max_records_per_request),
    ("RPC_MAX_TOTAL_RECORDS", validate_rpc_max_total_records),
    ("RPC_PAGINATION_DELAY_MS", validate_rpc_pagination_delay_ms),
    ("RPC_CIRCUIT_BREAKER_FAILURE_THRESHOLD", validate_circuit_breaker_failure_threshold),
    ("RPC_CIRCUIT_BREAKER_SUCCESS_THRESHOLD", validate_circuit_breaker_success_threshold),
    ("RPC_CIRCUIT_BREAKER_TIMEOUT_SECONDS", validate_circuit_breaker_timeout_seconds),
    ("RPC_MAX_RETRIES", validate_rpc_max_retries),
    ("RPC_INITIAL_BACKOFF_MS", validate_rpc_initial_backoff_ms),
    ("RPC_MAX_BACKOFF_MS", validate_rpc_max_backoff_ms),
    ("REQUEST_TIMEOUT_SECONDS", validate_request_timeout),
    ("SLOW_QUERY_THRESHOLD_MS", validate_slow_query_threshold),
    ("CACHE_CORRIDOR_METRICS_TTL", validate_cache_ttl),
    ("CACHE_ANCHOR_DATA_TTL", validate_cache_ttl),
    ("CACHE_DASHBOARD_STATS_TTL", validate_cache_ttl),
    ("MAX_IN_FLIGHT_REQUESTS", validate_max_in_flight_requests),
    ("COMPRESSION_MIN_SIZE", validate_compression_min_size),
    ("WEBHOOK_DISPATCHER_MAX_RESTARTS", validate_webhook_dispatcher_max_restarts),
    ("JWT_SECRET", validate_jwt_secret),
    ("ENCRYPTION_KEY", validate_encryption_key),
];

/// Validates all required environment variables are set
pub fn validate_env() -> Result<()> {
    // STELLAR_NETWORK is a fatal startup guard: a missing or unknown value would
    // silently route a testnet deployment against mainnet data.
    match env::var("STELLAR_NETWORK") {
        Ok(ref n) if n == "mainnet" || n == "testnet" => {}
        Ok(ref n) => panic!(
            "STELLAR_NETWORK must be set to 'mainnet' or 'testnet', got '{n}'"
        ),
        Err(_) => panic!("STELLAR_NETWORK must be set to 'mainnet' or 'testnet'"),
    }

    let mut errors = Vec::new();

    // Check required variables
    for var in REQUIRED_VARS {
        if env::var(var).is_err() {
            errors.push(format!("Missing required environment variable: {var}"));
        }
    }

    // Validate format of present variables
    for (var, validator) in VALIDATED_VARS {
        if let Ok(value) = env::var(var) {
            if !validator(&value) {
                errors.push(format!(
                    "Invalid value for environment variable {var}: '{value}'"
                ));
            }
        }
    }

    // Specific, actionable validation for JWT_SECRET
    if let Ok(jwt_secret) = env::var("JWT_SECRET") {
        if jwt_secret == "CHANGE_ME_generate_with_openssl_rand_base64_48" {
            errors.push(
                "JWT_SECRET is set to the placeholder value. \
                This is a critical security risk. \
                Generate a secure secret with: openssl rand -base64 48"
                    .to_string(),
            );
        } else if jwt_secret.len() < 32 {
            errors.push(format!(
                "JWT_SECRET is too short ({} characters). \
                Must be at least 32 characters. \
                Generate a secure secret with: openssl rand -base64 48",
                jwt_secret.len()
            ));
        }
    }

    // Specific, actionable validation for ENCRYPTION_KEY
    if let Ok(encryption_key) = env::var("ENCRYPTION_KEY") {
        if encryption_key == "CHANGE_ME_generate_with_openssl_rand_hex_32" {
            errors.push(
                "ENCRYPTION_KEY is set to the placeholder value. \
                This is a critical security risk. \
                Generate a secure encryption key with: openssl rand -hex 32"
                    .to_string(),
            );
        } else if encryption_key.len() < 64 {
            errors.push(format!(
                "ENCRYPTION_KEY is too short ({} characters). \
                Must be 64 characters (32 bytes as hex). \
                Generate a secure encryption key with: openssl rand -hex 32",
                encryption_key.len()
            ));
        }
    }

    // SNAPSHOT_CONTRACT_ID must not be the placeholder value when set.
    // This is the primary contract ID consumed by ContractService for
    // snapshot submission; a placeholder silently disables on-chain anchoring.
    if let Ok(contract_id) = env::var("SNAPSHOT_CONTRACT_ID") {
        if contract_id == "CHANGE_ME_source_contracts_env_testnet" {
            errors.push(
                "SNAPSHOT_CONTRACT_ID is set to the placeholder value. \
                Source contracts/.env.testnet to get the real deployed contract ID: \
                source contracts/.env.testnet && export SNAPSHOT_CONTRACT_ID=$PAYRAIDER_CONTRACT_ID"
                    .to_string(),
            );
        }
    }

    // SEP10_SERVER_PUBLIC_KEY must be a valid Stellar public key when set
    if let Ok(sep10_key) = env::var("SEP10_SERVER_PUBLIC_KEY") {
        if !validate_stellar_public_key(&sep10_key) {
            errors.push(
                "SEP10_SERVER_PUBLIC_KEY is not a valid Stellar public key. \
                It must start with 'G', be exactly 56 characters, use base32 encoding (A-Z, 2-7), \
                and must not be a placeholder. \
                Generate one with: stellar keys generate"
                    .to_string(),
            );
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

    // Soroban contract IDs
    log_var("SOROBAN_RPC_URL");
    log_var("SNAPSHOT_CONTRACT_ID");
    if env::var("STELLAR_SOURCE_SECRET_KEY").is_ok() {
        tracing::info!("  STELLAR_SOURCE_SECRET_KEY: [REDACTED]");
    }

    // Pool config
    log_var("DB_POOL_MAX_CONNECTIONS");
    log_var("DB_POOL_MIN_CONNECTIONS");
    log_var("DB_POOL_CONNECT_TIMEOUT_SECONDS");
    log_var("DB_POOL_IDLE_TIMEOUT_SECONDS");
    log_var("DB_POOL_MAX_LIFETIME_SECONDS");

    // Request timeout
    log_var("REQUEST_TIMEOUT_SECONDS");

    // CORS
    log_var("CORS_ALLOWED_ORIGINS");

    // Slack Bot
    if let Ok(slack_url) = env::var("SLACK_WEBHOOK_URL") {
        let sanitized = sanitize_url(&slack_url);
        tracing::info!("  SLACK_WEBHOOK_URL: {}", sanitized);
    }

    // Price feed (don't log API key)
    log_var("PRICE_FEED_PROVIDER");
    if env::var("PRICE_FEED_API_KEY").is_ok() {
        tracing::info!("  PRICE_FEED_API_KEY: [REDACTED]");
    }

    // RPC Pagination
    log_var("RPC_MAX_RECORDS_PER_REQUEST");
    log_var("RPC_MAX_TOTAL_RECORDS");
    log_var("RPC_PAGINATION_DELAY_MS");

    // Telegram
    if env::var("TELEGRAM_BOT_TOKEN").is_ok() {
        tracing::info!("  TELEGRAM_BOT_TOKEN: [REDACTED]");
    }
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
                return format!("{scheme}{user}:****{host_and_db}");
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
            return format!("{scheme}****@{host_and_path}");
        }
    }
    url.to_string()
}

/// Validate port number
fn validate_port(value: &str) -> bool {
    value.parse::<u16>().map(|p| p > 0).unwrap_or(false)
}

/// Validate positive number
fn validate_positive_number(value: &str) -> bool {
    value.parse::<u32>().map(|n| n > 0).unwrap_or(false)
}

/// Validate positive number with maximum bound
fn validate_positive_number_with_max(value: &str, max: u32) -> bool {
    value.parse::<u32>().map(|n| n > 0 && n <= max).unwrap_or(false)
}

/// Validate DB pool max connections: [1, 1000]
fn validate_db_pool_max_connections(value: &str) -> bool {
    validate_positive_number_with_max(value, 1000)
}

/// Validate DB pool min connections: [1, 100]
fn validate_db_pool_min_connections(value: &str) -> bool {
    validate_positive_number_with_max(value, 100)
}

/// Validate DB pool connect timeout: [1, 60] seconds
fn validate_db_pool_connect_timeout(value: &str) -> bool {
    validate_positive_number_with_max(value, 60)
}

/// Validate DB pool idle timeout: [60, 3600] seconds
fn validate_db_pool_idle_timeout(value: &str) -> bool {
    value
        .parse::<u64>()
        .map(|n| n >= 60 && n <= 3600)
        .unwrap_or(false)
}

/// Validate DB pool max lifetime: [300, 7200] seconds
fn validate_db_pool_max_lifetime(value: &str) -> bool {
    value
        .parse::<u64>()
        .map(|n| n >= 300 && n <= 7200)
        .unwrap_or(false)
}

/// Validate DB busy timeout: [100, 30000] ms
fn validate_db_busy_timeout(value: &str) -> bool {
    value
        .parse::<u64>()
        .map(|n| n >= 100 && n <= 30_000)
        .unwrap_or(false)
}

/// Validate DB write pool max connections: [1, 10]
fn validate_db_write_pool_max_connections(value: &str) -> bool {
    validate_positive_number_with_max(value, 10)
}

/// Validate RPC max records per request: [1, 1000]
fn validate_rpc_max_records_per_request(value: &str) -> bool {
    validate_positive_number_with_max(value, 1000)
}

/// Validate RPC max total records: [1, 100000]
fn validate_rpc_max_total_records(value: &str) -> bool {
    validate_positive_number_with_max(value, 100_000)
}

/// Validate RPC pagination delay: [0, 1000] ms
fn validate_rpc_pagination_delay_ms(value: &str) -> bool {
    value
        .parse::<u64>()
        .map(|n| n <= 1000)
        .unwrap_or(false)
}

/// Validate circuit breaker failure threshold: [1, 100]
fn validate_circuit_breaker_failure_threshold(value: &str) -> bool {
    validate_positive_number_with_max(value, 100)
}

/// Validate circuit breaker success threshold: [1, 100]
fn validate_circuit_breaker_success_threshold(value: &str) -> bool {
    validate_positive_number_with_max(value, 100)
}

/// Validate circuit breaker timeout: [1, 3600] seconds
fn validate_circuit_breaker_timeout_seconds(value: &str) -> bool {
    validate_positive_number_with_max(value, 3600)
}

/// Validate RPC max retries: [0, 20]
fn validate_rpc_max_retries(value: &str) -> bool {
    value
        .parse::<u32>()
        .map(|n| n <= 20)
        .unwrap_or(false)
}

/// Validate RPC initial backoff: [10, 10000] ms
fn validate_rpc_initial_backoff_ms(value: &str) -> bool {
    value
        .parse::<u64>()
        .map(|n| n >= 10 && n <= 10_000)
        .unwrap_or(false)
}

/// Validate RPC max backoff: [100, 60000] ms
fn validate_rpc_max_backoff_ms(value: &str) -> bool {
    value
        .parse::<u64>()
        .map(|n| n >= 100 && n <= 60_000)
        .unwrap_or(false)
}

/// Validate REQUEST_TIMEOUT_SECONDS: must be in range [5, 300]
fn validate_request_timeout(value: &str) -> bool {
    value
        .parse::<u64>()
        .map(|n| (5..=300).contains(&n))
        .unwrap_or(false)
}

/// Validate SLOW_QUERY_THRESHOLD_MS: must be in range [1, 60000]
fn validate_slow_query_threshold(value: &str) -> bool {
    value
        .parse::<u64>()
        .map(|n| (1..=60_000).contains(&n))
        .unwrap_or(false)
}

/// Validate cache TTL: must be in range [10, 86400] seconds
fn validate_cache_ttl(value: &str) -> bool {
    value
        .parse::<u64>()
        .map(|n| n >= 10 && n <= 86_400)
        .unwrap_or(false)
}

/// Validate MAX_IN_FLIGHT_REQUESTS: must be in range [1, 10000]
fn validate_max_in_flight_requests(value: &str) -> bool {
    validate_positive_number_with_max(value, 10_000)
}

/// Validate COMPRESSION_MIN_SIZE: must be in range [100, 65535]
fn validate_compression_min_size(value: &str) -> bool {
    value
        .parse::<u16>()
        .map(|n| n >= 100 && n <= 65_535)
        .unwrap_or(false)
}

/// Validate WEBHOOK_DISPATCHER_MAX_RESTARTS: must be in range [0, 100]
fn validate_webhook_dispatcher_max_restarts(value: &str) -> bool {
    value
        .parse::<u32>()
        .map(|n| n <= 100)
        .unwrap_or(false)
}

/// Validate JWT secret
/// Must not be the placeholder value and should be at least 32 characters
fn validate_jwt_secret(value: &str) -> bool {
    // Check if it's the placeholder value
    if value == "CHANGE_ME_generate_with_openssl_rand_base64_48" {
        return false;
    }

    // Ensure minimum length of 32 characters for security
    value.len() >= 32
}

/// Validate encryption key
/// Must not be the placeholder value and should be 64 characters (32 bytes as hex)
fn validate_encryption_key(value: &str) -> bool {
    // Check if it's the placeholder value
    if value == "CHANGE_ME_generate_with_openssl_rand_hex_32" {
        return false;
    }

    // Ensure minimum length of 64 characters (32 bytes × 2 for hex encoding)
    value.len() >= 64
}

/// Validate Stellar public key format
/// Must start with 'G' and be exactly 56 characters (Ed25519 public key in base32)
fn validate_stellar_public_key(value: &str) -> bool {
    if !value.starts_with('G') || value.len() != 56 {
        return false;
    }

    // Check if it's not the placeholder value
    if value == "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX" {
        return false;
    }

    // Validate base32 characters (A-Z, 2-7)
    value
        .chars()
        .all(|c| c.is_ascii_uppercase() || ('2'..='7').contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_sqlite_url() {
        let url = "sqlite:./payraider.db";
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
    fn test_validate_jwt_secret() {
        // Valid secrets
        assert!(validate_jwt_secret("a".repeat(32).as_str()));
        assert!(validate_jwt_secret(
            "this_is_a_very_secure_jwt_secret_key_12345"
        ));

        // Invalid - placeholder
        assert!(!validate_jwt_secret(
            "CHANGE_ME_generate_with_openssl_rand_base64_48"
        ));

        // Invalid - too short
        assert!(!validate_jwt_secret("short"));
        assert!(!validate_jwt_secret("only_31_chars_long_x"));
    }

    #[test]
    fn test_validate_encryption_key() {
        // Valid keys (64+ characters)
        assert!(validate_encryption_key("a".repeat(64).as_str()));
        assert!(validate_encryption_key("0123456789abcdef".repeat(4).as_str()));

        // Invalid - placeholder
        assert!(!validate_encryption_key(
            "CHANGE_ME_generate_with_openssl_rand_hex_32"
        ));

        // Invalid - too short
        assert!(!validate_encryption_key("short"));
        assert!(!validate_encryption_key("a".repeat(63).as_str()));
    }

    #[test]
    fn test_validate_positive_number() {
        assert!(validate_positive_number("1"));
        assert!(validate_positive_number("100"));
        assert!(!validate_positive_number("0"));
        assert!(!validate_positive_number("-1"));
        assert!(!validate_positive_number("abc"));
    }

    #[test]
    fn test_validate_env_rejects_jwt_placeholder() {
        let _guard = crate::lock_env_test();
        std::env::set_var("STELLAR_NETWORK", "testnet");
        std::env::set_var("DATABASE_URL", "sqlite://test.db");
        std::env::set_var("ENCRYPTION_KEY", "a".repeat(32));
        std::env::set_var(
            "JWT_SECRET",
            "CHANGE_ME_generate_with_openssl_rand_base64_48",
        );

        let result = validate_env();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("placeholder"),
            "Error should mention 'placeholder', got: {msg}"
        );
        assert!(
            msg.contains("openssl rand -base64 48"),
            "Error should include generation command, got: {msg}"
        );

        std::env::remove_var("STELLAR_NETWORK");
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("ENCRYPTION_KEY");
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_validate_env_rejects_short_jwt_secret() {
        let _guard = crate::lock_env_test();
        std::env::set_var("STELLAR_NETWORK", "testnet");
        std::env::set_var("DATABASE_URL", "sqlite://test.db");
        std::env::set_var("ENCRYPTION_KEY", "a".repeat(32));
        std::env::set_var("JWT_SECRET", "tooshort");

        let result = validate_env();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("too short"),
            "Error should mention 'too short', got: {msg}"
        );
        assert!(
            msg.contains("32 characters"),
            "Error should mention minimum length, got: {msg}"
        );

        std::env::remove_var("STELLAR_NETWORK");
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("ENCRYPTION_KEY");
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_validate_env_accepts_valid_jwt_secret() {
        let _guard = crate::lock_env_test();
        std::env::set_var("STELLAR_NETWORK", "mainnet");
        std::env::set_var("DATABASE_URL", "sqlite://test.db");
        std::env::set_var("ENCRYPTION_KEY", "a".repeat(64));
        std::env::set_var("JWT_SECRET", "a".repeat(48));

        let result = validate_env();
        assert!(result.is_ok(), "Should accept a valid JWT secret");

        std::env::remove_var("STELLAR_NETWORK");
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("ENCRYPTION_KEY");
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    #[should_panic(expected = "STELLAR_NETWORK must be set to 'mainnet' or 'testnet'")]
    fn test_validate_env_panics_when_stellar_network_missing() {
        let _guard = crate::lock_env_test();
        std::env::remove_var("STELLAR_NETWORK");
        std::env::set_var("DATABASE_URL", "sqlite://test.db");
        std::env::set_var("ENCRYPTION_KEY", "a".repeat(32));
        std::env::set_var("JWT_SECRET", "a".repeat(48));
        let _ = validate_env();
    }

    #[test]
    #[should_panic(expected = "STELLAR_NETWORK must be set to 'mainnet' or 'testnet'")]
    fn test_validate_env_panics_on_unknown_network() {
        let _guard = crate::lock_env_test();
        std::env::set_var("STELLAR_NETWORK", "devnet");
        std::env::set_var("DATABASE_URL", "sqlite://test.db");
        std::env::set_var("ENCRYPTION_KEY", "a".repeat(32));
        std::env::set_var("JWT_SECRET", "a".repeat(48));
        let _ = validate_env();
    }

    #[test]
    fn test_validate_env_rejects_encryption_key_placeholder() {
        let _guard = crate::lock_env_test();
        std::env::set_var("STELLAR_NETWORK", "testnet");
        std::env::set_var("DATABASE_URL", "sqlite://test.db");
        std::env::set_var(
            "ENCRYPTION_KEY",
            "CHANGE_ME_generate_with_openssl_rand_hex_32",
        );
        std::env::set_var("JWT_SECRET", "a".repeat(48));

        let result = validate_env();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("placeholder"),
            "Error should mention 'placeholder', got: {msg}"
        );
        assert!(
            msg.contains("openssl rand -hex 32"),
            "Error should include generation command, got: {msg}"
        );

        std::env::remove_var("STELLAR_NETWORK");
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("ENCRYPTION_KEY");
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_validate_env_rejects_short_encryption_key() {
        let _guard = crate::lock_env_test();
        std::env::set_var("STELLAR_NETWORK", "testnet");
        std::env::set_var("DATABASE_URL", "sqlite://test.db");
        std::env::set_var("ENCRYPTION_KEY", "tooshort");
        std::env::set_var("JWT_SECRET", "a".repeat(48));

        let result = validate_env();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("too short"),
            "Error should mention 'too short', got: {msg}"
        );
        assert!(
            msg.contains("64 characters"),
            "Error should mention minimum length, got: {msg}"
        );

        std::env::remove_var("STELLAR_NETWORK");
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("ENCRYPTION_KEY");
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_validate_env_accepts_valid_encryption_key() {
        let _guard = crate::lock_env_test();
        std::env::set_var("STELLAR_NETWORK", "mainnet");
        std::env::set_var("DATABASE_URL", "sqlite://test.db");
        std::env::set_var("ENCRYPTION_KEY", "a".repeat(64));
        std::env::set_var("JWT_SECRET", "a".repeat(48));

        let result = validate_env();
        assert!(result.is_ok(), "Should accept a valid ENCRYPTION_KEY");

        std::env::remove_var("STELLAR_NETWORK");
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("ENCRYPTION_KEY");
        std::env::remove_var("JWT_SECRET");
    }
}

