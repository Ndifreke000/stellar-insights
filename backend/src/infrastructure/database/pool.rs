// Database pool configuration - extracted from database.rs

use anyhow::Result;
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{ConnectOptions, SqlitePool};
use std::time::Duration;
use log::LevelFilter;

/// Configuration for database connection pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 2,
            connect_timeout_seconds: 30,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 1800,
        }
    }
}

/// SQL query logging configuration
#[derive(Debug, Clone)]
pub struct SqlLogConfig {
    pub level: LevelFilter,
    pub log_all_in_dev: bool,
    pub slow_query_threshold_ms: u64,
}

impl Default for SqlLogConfig {
    fn default() -> Self {
        Self {
            level: LevelFilter::Debug,
            log_all_in_dev: true,
            slow_query_threshold_ms: 100,
        }
    }
}

impl PoolConfig {
    /// Load pool configuration from environment variables
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            max_connections: std::env::var("DB_POOL_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            min_connections: std::env::var("DB_POOL_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2),
            connect_timeout_seconds: std::env::var("DB_POOL_CONNECT_TIMEOUT_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            idle_timeout_seconds: std::env::var("DB_POOL_IDLE_TIMEOUT_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(600),
            max_lifetime_seconds: std::env::var("DB_POOL_MAX_LIFETIME_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1800),
        }
    }

    /// Create a configured SQLite pool with WAL mode and SQL logging.
    pub async fn create_pool(&self, database_url: &str) -> Result<SqlitePool> {
        let sql_log = SqlLogConfig::from_env();

        let mut opts: SqliteConnectOptions = database_url
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid DATABASE_URL: {e}"))?;

        opts = opts.journal_mode(SqliteJournalMode::Wal);

        if sql_log.level != LevelFilter::Off {
            if sql_log.log_all_in_dev {
                opts = opts.log_statements(sql_log.level);
            } else {
                let threshold = Duration::from_millis(sql_log.slow_query_threshold_ms);
                opts = opts.log_slow_statements(sql_log.level, threshold);
            }
        }

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .acquire_timeout(Duration::from_secs(self.connect_timeout_seconds))
            .idle_timeout(Some(Duration::from_secs(self.idle_timeout_seconds)))
            .max_lifetime(Some(Duration::from_secs(self.max_lifetime_seconds)))
            .connect_with(opts)
            .await?;

        Ok(pool)
    }
}

