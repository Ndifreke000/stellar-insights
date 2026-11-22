//! Retention configuration and non-blocking deletion.
//!
//! This module implements a non-blocking retention system that deletes old
//! data in bounded batches with sleeps to avoid SQLite lock contention.
//!
//! Features:
//! - Configurable retention per table
//! - Batch deletion with configurable batch size
//! - Incremental VACUUM support
//! - Rollup to aggregates before deletion

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for retention policies per table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Default retention in days for tables without specific config.
    pub default_days: i64,
    /// Per-table retention configuration.
    pub table_configs: HashMap<String, TableRetention>,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        let mut table_configs = HashMap::new();
        
        // Contract events: 30 days retention by default
        table_configs.insert(
            "contract_events".to_string(),
            TableRetention {
                days: 30,
                batch_size: 1000,
                sleep_ms: 100,
                preserve_aggregates: true,
            },
        );
        
        // Corridor metrics: 365 days retention (preserve aggregates)
        table_configs.insert(
            "corridor_metrics".to_string(),
            TableRetention {
                days: 365,
                batch_size: 1000,
                sleep_ms: 100,
                preserve_aggregates: true,
            },
        );
        
        Self {
            default_days: 30,
            table_configs,
        }
    }
}

impl RetentionConfig {
    /// Get retention config for a table.
    pub fn get_table_config(&self, table: &str) -> TableRetention {
        self.table_configs
            .get(table)
            .cloned()
            .unwrap_or(TableRetention {
                days: self.default_days,
                batch_size: 1000,
                sleep_ms: 100,
                preserve_aggregates: true,
            })
    }

    /// Get retention in days for a table.
    pub fn get_days(&self, table: &str) -> i64 {
        self.table_configs
            .get(table)
            .map(|c| c.days)
            .unwrap_or(self.default_days)
    }
}

/// Retention configuration for a single table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRetention {
    /// Number of days to retain data.
    pub days: i64,
    /// Number of rows to delete per batch.
    pub batch_size: i64,
    /// Milliseconds to sleep between batches.
    pub sleep_ms: u64,
    /// Whether to roll up to aggregates before deletion.
    pub preserve_aggregates: bool,
}

/// Result of a retention run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionResult {
    /// Table name.
    pub table: String,
    /// Total rows deleted.
    pub deleted_rows: i64,
    /// Number of batches processed.
    pub batches_processed: i64,
    /// Total time taken in milliseconds.
    pub elapsed_ms: i64,
    /// Any warnings encountered.
    pub warnings: Vec<String>,
}

impl RetentionResult {
    /// Create a new result.
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            deleted_rows: 0,
            batches_processed: 0,
            elapsed_ms: 0,
            warnings: Vec::new(),
        }
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: &str) {
        self.warnings.push(warning.to_string());
    }
}

/// Retention execution errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionError {
    pub table: String,
    pub message: String,
    pub batch: i64,
}

impl std::fmt::Display for RetentionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Retention error for table '{}': {} (batch {})",
            self.table, self.message, self.batch
        )
    }
}

impl std::error::Error for RetentionError {}

/// Configuration for incremental VACUUM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VacuumConfig {
    /// Whether to run incremental VACUUM after deletions.
    pub enabled: bool,
    /// Number of pages to vacuum per batch.
    pub pages_per_batch: i32,
    /// Maximum pages to vacuum in total.
    pub max_pages: i32,
}

impl Default for VacuumConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pages_per_batch: 100,
            max_pages: 10000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RetentionConfig::default();
        
        assert_eq!(config.get_days("contract_events"), 30);
        assert_eq!(config.get_days("corridor_metrics"), 365);
        assert_eq!(config.get_days("unknown_table"), 30); // default
    }

    #[test]
    fn test_table_retention() {
        let config = RetentionConfig::default();
        let table_config = config.get_table_config("contract_events");
        
        assert_eq!(table_config.days, 30);
        assert_eq!(table_config.batch_size, 1000);
        assert_eq!(table_config.sleep_ms, 100);
        assert!(table_config.preserve_aggregates);
    }

    #[test]
    fn test_retention_result() {
        let mut result = RetentionResult::new("test_table");
        result.deleted_rows = 1000;
        result.batches_processed = 5;
        result.add_warning("Slow deletion detected");
        
        assert_eq!(result.table, "test_table");
        assert_eq!(result.deleted_rows, 1000);
        assert_eq!(result.batches_processed, 5);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "Slow deletion detected");
    }
}
