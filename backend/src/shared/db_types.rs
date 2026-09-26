//! Shared types and traits to break circular dependencies between
//! `database.rs` and service modules (e.g., `services/analytics.rs`).
//!
//! Both `database.rs` and services import from this module instead of
//! importing from each other, creating a clean dependency hierarchy:
//!
//! ```text
//!   database.rs ──────► shared/db_types.rs ◄────── services/analytics.rs
//!   database.rs ──────► shared/db_traits.rs ◄───── services/analytics.rs
//! ```
//!
//! Closes #2380

use serde::{Deserialize, Serialize};
use std::fmt;

/// Database-agnostic row representation for analytics queries.
/// This allows `services/analytics.rs` to define what it needs without
/// importing from `database.rs`, and `database.rs` to produce these
/// rows without importing service types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsRow {
    pub timestamp: i64,
    pub metric_name: String,
    pub metric_value: f64,
    pub label: Option<String>,
}

/// Generic query parameters for paginated database access.
#[derive(Debug, Clone, Default)]
pub struct QueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub order_by: Option<String>,
    pub descending: bool,
}

/// Trait abstracting database access for analytics services.
/// Services depend on this trait, not on `database.rs` directly.
/// `database.rs` implements this trait, breaking the cycle.
pub trait AnalyticsRepository: Send + Sync {
    /// Fetch analytics rows for a given time range.
    fn fetch_analytics(
        &self,
        start_ts: i64,
        end_ts: i64,
        params: &QueryParams,
    ) -> Result<Vec<AnalyticsRow>, RepositoryError>;

    /// Store an analytics row.
    fn store_analytics(&self, row: &AnalyticsRow) -> Result<(), RepositoryError>;

    /// Count rows matching a time range.
    fn count_analytics(&self, start_ts: i64, end_ts: i64) -> Result<u64, RepositoryError>;
}

/// Trait abstracting general database connection management.
pub trait DatabaseProvider: Send + Sync {
    /// Check if the database connection is healthy.
    fn is_healthy(&self) -> bool;

    /// Get the current connection pool size.
    fn pool_size(&self) -> usize;

    /// Get the number of active connections.
    fn active_connections(&self) -> usize;
}

/// Unified error type for repository operations.
#[derive(Debug)]
pub enum RepositoryError {
    NotFound,
    ConnectionFailed(String),
    QueryFailed(String),
    SerializationError(String),
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Record not found"),
            Self::ConnectionFailed(msg) => write!(f, "Database connection failed: {}", msg),
            Self::QueryFailed(msg) => write!(f, "Query failed: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for RepositoryError {}

/// Shared configuration for database connections.
/// Both `database.rs` and services can use this without importing each other.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: 20,
            min_connections: 5,
            connection_timeout_secs: 30,
            idle_timeout_secs: 600,
        }
    }
}

/// Helper for building query parameters.
impl QueryParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn order_by(mut self, column: &str, descending: bool) -> Self {
        self.order_by = Some(column.to_string());
        self.descending = descending;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_params_builder() {
        let params = QueryParams::new()
            .limit(100)
            .offset(200)
            .order_by("timestamp", true);

        assert_eq!(params.limit, Some(100));
        assert_eq!(params.offset, Some(200));
        assert_eq!(params.order_by, Some("timestamp".to_string()));
        assert!(params.descending);
    }

    #[test]
    fn test_repository_error_display() {
        let err = RepositoryError::NotFound;
        assert_eq!(format!("{}", err), "Record not found");

        let err = RepositoryError::QueryFailed("syntax error".to_string());
        assert!(format!("{}", err).contains("syntax error"));
    }

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
    }

    struct MockRepository;

    impl AnalyticsRepository for MockRepository {
        fn fetch_analytics(
            &self,
            _start: i64,
            _end: i64,
            _params: &QueryParams,
        ) -> Result<Vec<AnalyticsRow>, RepositoryError> {
            Ok(vec![AnalyticsRow {
                timestamp: 1000,
                metric_name: "test".to_string(),
                metric_value: 42.0,
                label: None,
            }])
        }

        fn store_analytics(&self, _row: &AnalyticsRow) -> Result<(), RepositoryError> {
            Ok(())
        }

        fn count_analytics(&self, _start: i64, _end: i64) -> Result<u64, RepositoryError> {
            Ok(1)
        }
    }

    #[test]
    fn test_analytics_repository_trait() {
        let repo = MockRepository;
        let rows = repo.fetch_analytics(0, 100, &QueryParams::default()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].metric_value, 42.0);
    }
}
