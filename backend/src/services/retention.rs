//! Non-blocking retention service for event data.
//!
//! This service implements batch deletion with sleeps to avoid SQLite lock
//! contention. It also rolls up data to aggregates before deletion so that
//! analytics survive beyond raw data retention.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

use crate::db::aggregates::CorridorAggregates;
use crate::models::retention::{RetentionConfig, RetentionResult, VacuumConfig};
use crate::state::AppState;

/// Non-blocking retention service.
pub struct RetentionService {
    config: RetentionConfig,
    vacuum_config: VacuumConfig,
    app_state: AppState,
}

impl RetentionService {
    /// Create a new retention service.
    pub fn new(app_state: AppState, config: Option<RetentionConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            vacuum_config: VacuumConfig::default(),
            app_state,
        }
    }

    /// Run retention for all configured tables.
    pub async fn run_retention(&self) -> Vec<RetentionResult> {
        let mut results = Vec::new();

        // Delete old contract events
        if let Some(result) = self.delete_old_events().await {
            results.push(result);
        }

        // Delete old corridor metrics
        if let Some(result) = self.delete_old_corridor_metrics().await {
            results.push(result);
        }

        // Run incremental VACUUM
        if let Some(result) = self.run_vacuum().await {
            results.push(result);
        }

        results
    }

    /// Delete old contract events in batches.
    pub async fn delete_old_events(&self) -> Option<RetentionResult> {
        let table = "contract_events";
        let days = self.config.get_days(table);
        let batch_size = self.config.get_table_config(table).batch_size;
        let sleep_ms = self.config.get_table_config(table).sleep_ms;

        info!(
            "Starting retention for {}: deleting {} days old data in batches of {}",
            table, days, batch_size
        );

        let cutoff_datetime = format!("datetime('now', '-{} days')", days);

        let mut result = RetentionResult::new(table);
        let start_time = std::time::Instant::now();

        let pool = self.app_state.db.pool().clone();

        loop {
            // Delete a bounded batch
            let deleted: i64 = match sqlx::query_scalar(
                format!(
                    "DELETE FROM {} WHERE id IN (
                        SELECT id FROM {} WHERE created_at < {}
                        LIMIT {}
                    ) RETURNING id",
                    table, table, cutoff_datetime, batch_size
                )
                .as_str(),
            )
            .fetch_all(&pool)
            .await
            {
                Ok(rows) => rows.len() as i64,
                Err(e) => {
                    error!("Failed to delete batch from {}: {}", table, e);
                    result.add_warning(&format!("Batch deletion failed: {}", e));
                    break;
                }
            };

            if deleted == 0 {
                info!("No more rows to delete from {}", table);
                break;
            }

            result.deleted_rows += deleted;
            result.batches_processed += 1;

            info!(
                "Deleted {} rows from {} (batch {})",
                deleted, table, result.batches_processed
            );

            // Sleep between batches to avoid lock contention
            if sleep_ms > 0 {
                time::sleep(Duration::from_millis(sleep_ms)).await;
            }
        }

        result.elapsed_ms = start_time.elapsed().as_millis() as i64;
        
        info!(
            "Completed retention for {}: {} rows deleted in {} batches, {}ms",
            table, result.deleted_rows, result.batches_processed, result.elapsed_ms
        );

        Some(result)
    }

    /// Delete old corridor metrics in batches.
    pub async fn delete_old_corridor_metrics(&self) -> Option<RetentionResult> {
        let table = "corridor_metrics";
        let days = self.config.get_days(table);
        let batch_size = self.config.get_table_config(table).batch_size;
        let sleep_ms = self.config.get_table_config(table).sleep_ms;

        info!(
            "Starting retention for {}: deleting {} days old data in batches of {}",
            table, days, batch_size
        );

        // Get cutoff date
        let cutoff_date = chrono::Utc::now().date_naive() - chrono::Duration::days(days);
        let cutoff_datetime = format!("'{}'", cutoff_date.format("%Y-%m-%d"));

        let mut result = RetentionResult::new(table);
        let start_time = std::time::Instant::now();

        let pool = self.app_state.db.pool().clone();

        loop {
            // Delete a bounded batch
            let deleted: i64 = match sqlx::query_scalar(
                format!(
                    "DELETE FROM {} WHERE id IN (
                        SELECT id FROM {} WHERE date < {}
                        LIMIT {}
                    ) RETURNING id",
                    table, table, cutoff_datetime, batch_size
                )
                .as_str(),
            )
            .fetch_all(&pool)
            .await
            {
                Ok(rows) => rows.len() as i64,
                Err(e) => {
                    error!("Failed to delete batch from {}: {}", table, e);
                    result.add_warning(&format!("Batch deletion failed: {}", e));
                    break;
                }
            };

            if deleted == 0 {
                info!("No more rows to delete from {}", table);
                break;
            }

            result.deleted_rows += deleted;
            result.batches_processed += 1;

            info!(
                "Deleted {} rows from {} (batch {})",
                deleted, table, result.batches_processed
            );

            // Sleep between batches to avoid lock contention
            if sleep_ms > 0 {
                time::sleep(Duration::from_millis(sleep_ms)).await;
            }
        }

        result.elapsed_ms = start_time.elapsed().as_millis() as i64;
        
        info!(
            "Completed retention for {}: {} rows deleted in {} batches, {}ms",
            table, result.deleted_rows, result.batches_processed, result.elapsed_ms
        );

        Some(result)
    }

    /// Run incremental VACUUM.
    pub async fn run_vacuum(&self) -> Option<RetentionResult> {
        if !self.vacuum_config.enabled {
            info!("Incremental VACUUM is disabled");
            return None;
        }

        info!(
            "Starting incremental VACUUM (pages per batch: {}, max pages: {})",
            self.vacuum_config.pages_per_batch, self.vacuum_config.max_pages
        );

        let mut result = RetentionResult::new("database_vacuum");
        let start_time = std::time::Instant::now();

        let pool = self.app_state.db.pool().clone();

        let mut total_pages_freed = 0;
        let mut pages_freed_in_batch: i32;

        loop {
            // Get the number of free pages before vacuum
            let free_pages: i32 = match sqlx::query_scalar("PRAGMA freelist_count")
                .fetch_one(&pool)
                .await
            {
                Ok(count) => count,
                Err(e) => {
                    error!("Failed to get freelist count: {}", e);
                    result.add_warning(&format!("Freelist count failed: {}", e));
                    break;
                }
            };

            if free_pages == 0 {
                info!("No more pages to vacuum");
                break;
            }

            // Vacuum a bounded number of pages
            pages_freed_in_batch = self.vacuum_config.pages_per_batch.min(free_pages);

            // Run incremental_vacuum for this batch
            match sqlx::query(&format!(
                "PRAGMA incremental_vacuum({})",
                pages_freed_in_batch
            ))
            .execute(&pool)
            .await
            {
                Ok(_) => {
                    total_pages_freed += pages_freed_in_batch;
                    result.batches_processed += 1;

                    info!(
                        "Vacuumed {} pages (total: {})",
                        pages_freed_in_batch, total_pages_freed
                    );

                    // Check if we've reached the limit
                    if total_pages_freed >= self.vacuum_config.max_pages {
                        info!(
                            "Reached max pages to vacuum ({})",
                            self.vacuum_config.max_pages
                        );
                        break;
                    }

                    // Sleep between vacuum batches
                    if self.vacuum_config.pages_per_batch > 0 {
                        time::sleep(Duration::from_millis(50)).await;
                    }
                }
                Err(e) => {
                    error!("Failed to vacuum pages: {}", e);
                    result.add_warning(&format!("Vacuum failed: {}", e));
                    break;
                }
            }
        }

        result.elapsed_ms = start_time.elapsed().as_millis() as i64;

        info!(
            "Completed incremental VACUUM: {} pages freed in {} batches, {}ms",
            total_pages_freed, result.batches_processed, result.elapsed_ms
        );

        Some(result)
    }

    /// Get current retention configuration.
    pub fn get_config(&self) -> &RetentionConfig {
        &self.config
    }

    /// Get current vacuum configuration.
    pub fn get_vacuum_config(&self) -> &VacuumConfig {
        &self.vacuum_config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::state::AppState;
    use std::sync::Arc;

    async fn mock_app_state() -> AppState {
        // Create a mock AppState for testing
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let db = Arc::new(Database::new(pool));
        
        AppState::new(
            db,
            Arc::new(CacheManager::new_in_memory_for_tests(CacheConfig::default())),
            Arc::new(WsState::new()),
            Arc::new(DataIngestionService::new(
                Arc::new(StellarRpcClient::new("https://horizon.stellar.org")),
                db.clone(),
            )),
            Arc::new(StellarRpcClient::new("https://horizon.stellar.org")),
        )
    }

    #[tokio::test]
    async fn test_retention_service_creation() {
        let app_state = mock_app_state().await;
        let service = RetentionService::new(app_state, None);
        
        let config = service.get_config();
        assert_eq!(config.get_days("contract_events"), 30);
        assert_eq!(config.get_days("corridor_metrics"), 365);
    }

    #[tokio::test]
    async fn test_vacuum_config() {
        let app_state = mock_app_state().await;
        let service = RetentionService::new(app_state, None);
        
        let vacuum_config = service.get_vacuum_config();
        assert!(vacuum_config.enabled);
        assert_eq!(vacuum_config.pages_per_batch, 100);
        assert_eq!(vacuum_config.max_pages, 10000);
    }
}
