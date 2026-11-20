#![allow(clippy::too_many_lines)]

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::db::aggregates::CorridorAggregates;
use crate::distributed_lock::{DistributedLock, FencingToken};
use crate::models::event::{ContractEvent, IndexedEvent};
use crate::network::StellarNetwork;
use crate::rpc::stellar::{self, RpcClient};

/// Delete old events using batched deletion to avoid SQLite lock contention.
/// Also rolls up events to corridor metrics before deletion.
pub async fn cleanup_old_events_batched(
    pool: &SqlitePool,
    days_to_keep: i64,
    batch_size: i64,
    sleep_ms: u64,
) -> Result<i64> {
    info!(
        "Cleaning up events older than {} days in batches of {}",
        days_to_keep, batch_size
    );

    let cutoff_datetime = format!("datetime('now', '-{} days')", days_to_keep);

    let mut total_deleted = 0i64;
    let start_time = std::time::Instant::now();

    loop {
        // First, roll up events to corridor metrics before deletion
        // This ensures analytics survive beyond raw data retention
        let rolled_up: i64 = sqlx::query_scalar(
            r"
            INSERT INTO corridor_metrics (
                corridor_key, asset_a_code, asset_a_issuer, asset_b_code, asset_b_issuer,
                date, total_transactions, successful_transactions, failed_transactions,
                success_rate, volume_usd
            )
            SELECT
                c.source_asset_code || ':' || c.source_asset_issuer || '->' || 
                c.destination_asset_code || ':' || c.destination_issuer AS corridor_key,
                c.source_asset_code,
                c.source_asset_issuer,
                c.destination_asset_code,
                c.destination_asset_issuer,
                date(e.created_at) as date,
                COUNT(*) as total_transactions,
                SUM(CASE WHEN e.success = 1 THEN 1 ELSE 0 END) as successful_transactions,
                SUM(CASE WHEN e.success = 0 THEN 1 ELSE 0 END) as failed_transactions,
                AVG(CASE WHEN e.success = 1 THEN 100.0 ELSE 0.0 END) as success_rate,
                SUM(COALESCE(e.amount_usd, 0)) as volume_usd
            FROM contract_events e
            JOIN corridors c ON e.corridor_id = c.id
            WHERE e.created_at < ?
            GROUP BY c.id, date(e.created_at)
            ON CONFLICT (corridor_key, date) DO UPDATE SET
                total_transactions = corridor_metrics.total_transactions + EXCLUDED.total_transactions,
                successful_transactions = corridor_metrics.successful_transactions + EXCLUDED.successful_transactions,
                failed_transactions = corridor_metrics.failed_transactions + EXCLUDED.failed_transactions,
                success_rate = (corridor_metrics.success_rate + EXCLUDED.success_rate) / 2,
                volume_usd = corridor_metrics.volume_usd + EXCLUDED.volume_usd,
                updated_at = CURRENT_TIMESTAMP
            ",
        )
        .bind(cutoff_datetime.clone())
        .execute(pool)
        .await
        .context("Failed to roll up events to corridor metrics")?
        .rows_affected();

        info!("Rolled up {} events to corridor metrics", rolled_up);

        // Delete in a bounded batch
        let deleted: i64 = sqlx::query(
            format!(
                "DELETE FROM contract_events WHERE id IN (
                    SELECT id FROM contract_events WHERE created_at < {}
                    LIMIT {}
                )",
                cutoff_datetime, batch_size
            )
            .as_str(),
        )
        .execute(pool)
        .await
        .context("Failed to delete old events")?
        .rows_affected() as i64;

        if deleted == 0 {
            info!("No more events to delete");
            break;
        }

        total_deleted += deleted;
        info!("Deleted {} old events (batch total: {})", deleted, total_deleted);

        // Sleep between batches to avoid lock contention
        if sleep_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
        }
    }

    let elapsed = start_time.elapsed().as_millis() as i64;
    info!(
        "Cleaned up {} old events in {}ms",
        total_deleted, elapsed
    );

    Ok(total_deleted)
}

/// Incremental VACUUM to reclaim space after deletions.
pub async fn incremental_vacuum(
    pool: &SqlitePool,
    pages_per_batch: i32,
    max_pages: i32,
) -> Result<i32> {
    info!(
        "Starting incremental VACUUM (pages per batch: {}, max pages: {})",
        pages_per_batch, max_pages
    );

    let mut total_pages_freed = 0i32;

    loop {
        // Get the number of free pages before vacuum
        let free_pages: i32 = sqlx::query_scalar("PRAGMA freelist_count")
            .fetch_one(pool)
            .await
            .context("Failed to get freelist count")?;

        if free_pages == 0 {
            info!("No more pages to vacuum");
            break;
        }

        // Vacuum a bounded number of pages
        let pages_to_vacuum = pages_per_batch.min(free_pages);

        sqlx::query(&format!("PRAGMA incremental_vacuum({})", pages_to_vacuum))
            .execute(pool)
            .await
            .context("Failed to vacuum pages")?;

        total_pages_freed += pages_to_vacuum;
        info!("Vacuumed {} pages (total: {})", pages_to_vacuum, total_pages_freed);

        // Check if we've reached the limit
        if total_pages_freed >= max_pages {
            info!("Reached max pages to vacuum ({})", max_pages);
            break;
        }

        // Small sleep between vacuum batches
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    info!("Completed incremental VACUUM: {} pages freed", total_pages_freed);
    Ok(total_pages_freed)
}

/// Roll up events to corridor metrics before deletion.
pub async fn rollup_events_to_aggregates(
    pool: &SqlitePool,
    cutoff_datetime: &str,
) -> Result<i64> {
    info!("Rolling up events before {} to corridor metrics", cutoff_datetime);

    let rolled_up: i64 = sqlx::query(
        r"
        INSERT INTO corridor_metrics (
            corridor_key, asset_a_code, asset_a_issuer, asset_b_code, asset_b_issuer,
            date, total_transactions, successful_transactions, failed_transactions,
            success_rate, volume_usd
        )
        SELECT
            c.source_asset_code || ':' || c.source_asset_issuer || '->' || 
            c.destination_asset_code || ':' || c.destination_asset_issuer AS corridor_key,
            c.source_asset_code,
            c.source_asset_issuer,
            c.destination_asset_code,
            c.destination_asset_issuer,
            date(e.created_at) as date,
            COUNT(*) as total_transactions,
            SUM(CASE WHEN e.success = 1 THEN 1 ELSE 0 END) as successful_transactions,
            SUM(CASE WHEN e.success = 0 THEN 1 ELSE 0 END) as failed_transactions,
            AVG(CASE WHEN e.success = 1 THEN 100.0 ELSE 0.0 END) as success_rate,
            SUM(COALESCE(e.amount_usd, 0)) as volume_usd
        FROM contract_events e
        JOIN corridors c ON e.corridor_id = c.id
        WHERE e.created_at < ?
        GROUP BY c.id, date(e.created_at)
        ON CONFLICT (corridor_key, date) DO UPDATE SET
            total_transactions = corridor_metrics.total_transactions + EXCLUDED.total_transactions,
            successful_transactions = corridor_metrics.successful_transactions + EXCLUDED.successful_transactions,
            failed_transactions = corridor_metrics.failed_transactions + EXCLUDED.failed_transactions,
            success_rate = (corridor_metrics.success_rate + EXCLUDED.success_rate) / 2,
            volume_usd = corridor_metrics.volume_usd + EXCLUDED.volume_usd,
            updated_at = CURRENT_TIMESTAMP
        "
    )
    .bind(cutoff_datetime)
    .execute(pool)
    .await
    .context("Failed to roll up events to corridor metrics")?
    .rows_affected() as i64;

    info!("Rolled up {} events to corridor metrics", rolled_up);
    Ok(rolled_up)
}
