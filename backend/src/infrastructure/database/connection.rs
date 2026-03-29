// Database connection wrapper - extracted from database.rs

use anyhow::Result;
use sqlx::SqlitePool;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::admin_audit_log::AdminAuditLogger;
use crate::features::analytics::metrics::compute_anchor_metrics;
use crate::cache::CacheManager;
use crate::features::anchors::models::Anchor;
use crate::features::corridors::models::CorridorRecord;

pub struct Database {
    pool: SqlitePool,
    pub admin_audit_logger: AdminAuditLogger,
    slow_query_threshold_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct PoolMetrics {
    pub size: u32,
    pub idle: usize,
    pub active: u32,
}

impl Database {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        let admin_audit_logger = AdminAuditLogger::new(pool.clone());
        let slow_query_threshold_ms = std::env::var("SLOW_QUERY_THRESHOLD_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);
        Self {
            pool,
            admin_audit_logger,
            slow_query_threshold_ms,
        }
    }

    pub async fn execute_with_timing<T, F>(&self, operation: &str, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start = Instant::now();
        let result = f.await;
        let elapsed = start.elapsed();
        let status = if result.is_ok() { "success" } else { "error" };

        if elapsed.as_millis() as u64 > self.slow_query_threshold_ms {
            log::warn!(
                "Slow query detected: '{}' took {}ms (threshold: {}ms)",
                operation,
                elapsed.as_millis(),
                self.slow_query_threshold_ms,
            );
        }

        // TODO: observability::metrics::observe_db_query
        result
    }

    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    #[must_use]
    pub fn pool_metrics(&self) -> PoolMetrics {
        let size = self.pool.size();
        let idle = self.pool.num_idle();
        let active = size.saturating_sub(idle as u32);
        PoolMetrics { size, idle, active }
    }
}

