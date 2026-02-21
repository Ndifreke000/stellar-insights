use std::env;
use stellar_insights_backend::database::PoolConfig;

#[test]
fn test_pool_config_defaults() {
    let config = PoolConfig::default();

    assert_eq!(config.max_connections, 10);
    assert_eq!(config.min_connections, 2);
    assert_eq!(config.connect_timeout_seconds, 30);
    assert_eq!(config.idle_timeout_seconds, 600);
    assert_eq!(config.max_lifetime_seconds, 1800);
}

#[test]
fn test_pool_config_from_env() {
    // Set environment variables
    env::set_var("DB_POOL_MAX_CONNECTIONS", "20");
    env::set_var("DB_POOL_MIN_CONNECTIONS", "5");
    env::set_var("DB_POOL_CONNECT_TIMEOUT_SECONDS", "60");
    env::set_var("DB_POOL_IDLE_TIMEOUT_SECONDS", "300");
    env::set_var("DB_POOL_MAX_LIFETIME_SECONDS", "3600");

    let config = PoolConfig::from_env();

    assert_eq!(config.max_connections, 20);
    assert_eq!(config.min_connections, 5);
    assert_eq!(config.connect_timeout_seconds, 60);
    assert_eq!(config.idle_timeout_seconds, 300);
    assert_eq!(config.max_lifetime_seconds, 3600);

    // Clean up
    env::remove_var("DB_POOL_MAX_CONNECTIONS");
    env::remove_var("DB_POOL_MIN_CONNECTIONS");
    env::remove_var("DB_POOL_CONNECT_TIMEOUT_SECONDS");
    env::remove_var("DB_POOL_IDLE_TIMEOUT_SECONDS");
    env::remove_var("DB_POOL_MAX_LIFETIME_SECONDS");
}

#[test]
fn test_pool_config_from_env_with_defaults() {
    // Ensure no env vars are set
    env::remove_var("DB_POOL_MAX_CONNECTIONS");
    env::remove_var("DB_POOL_MIN_CONNECTIONS");
    env::remove_var("DB_POOL_CONNECT_TIMEOUT_SECONDS");
    env::remove_var("DB_POOL_IDLE_TIMEOUT_SECONDS");
    env::remove_var("DB_POOL_MAX_LIFETIME_SECONDS");

    let config = PoolConfig::from_env();

    // Should use defaults
    assert_eq!(config.max_connections, 10);
    assert_eq!(config.min_connections, 2);
    assert_eq!(config.connect_timeout_seconds, 30);
    assert_eq!(config.idle_timeout_seconds, 600);
    assert_eq!(config.max_lifetime_seconds, 1800);
}

#[test]
fn test_pool_config_from_env_partial() {
    // Set only some environment variables
    env::set_var("DB_POOL_MAX_CONNECTIONS", "15");
    env::remove_var("DB_POOL_MIN_CONNECTIONS");
    env::set_var("DB_POOL_CONNECT_TIMEOUT_SECONDS", "45");

    let config = PoolConfig::from_env();

    assert_eq!(config.max_connections, 15);
    assert_eq!(config.min_connections, 2); // default
    assert_eq!(config.connect_timeout_seconds, 45);
    assert_eq!(config.idle_timeout_seconds, 600); // default
    assert_eq!(config.max_lifetime_seconds, 1800); // default

    // Clean up
    env::remove_var("DB_POOL_MAX_CONNECTIONS");
    env::remove_var("DB_POOL_CONNECT_TIMEOUT_SECONDS");
}

#[tokio::test]
async fn test_pool_creation() {
    let config = PoolConfig {
        max_connections: 5,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 300,
        max_lifetime_seconds: 900,
    };

    // Use in-memory SQLite for testing
    let result = config.create_pool("sqlite::memory:").await;

    assert!(result.is_ok());
    let pool = result.unwrap();

    // Verify pool was created
    assert_eq!(pool.size(), 0); // No connections yet
}

#[tokio::test]
async fn test_pool_metrics() {
    use stellar_insights_backend::database::Database;

    let config = PoolConfig::default();
    let pool = config.create_pool("sqlite::memory:").await.unwrap();
    let db = Database::new(pool);

    let metrics = db.pool_metrics();

    // Initial state
    assert_eq!(metrics.size, 0);
    assert_eq!(metrics.idle, 0);
}
