//! Event Indexer Service
//!
//! This service indexes contract events, provides query interfaces,
//! and manages the event database for analytics and verification.
//!
//! # Features
//!
//! - **Event Storage**: Store and retrieve contract events with metadata
//! - **Query Interface**: Flexible filtering and sorting options
//! - **Verification Tracking**: Monitor event verification status
//! - **Statistics**: Aggregate event data for analytics
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use stellar_insights_backend::services::event_indexer::{EventIndexer, EventQuery, EventOrderBy};
//! use stellar_insights_backend::database::Database;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let db = Database::new(pool).await?;
//!     let indexer = EventIndexer::new(db);
//!     
//!     // Query events
//!     let query = EventQuery {
//!         contract_id: Some("contract_123".to_string()),
//!         limit: Some(100),
//!         order_by: Some(EventOrderBy::CreatedAtDesc),
//!         ..Default::default()
//!     };
//!     
//!     let events = indexer.query_events(query).await?;
//!     println!("Found {} events", events.len());
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Configuration
//!
//! The indexer supports various query options:
//!
//! - **Time-based filtering**: by creation time, ledger, or epoch
//! - **Status filtering**: by verification status
//! - **Pagination**: limit and offset for large result sets
//! - **Sorting**: multiple sort orders for different use cases
//!
use crate::database::Database;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::fmt::Write;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Indexed contract event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedEvent {
    pub id: String,
    pub contract_id: String,
    pub event_type: String,
    pub epoch: Option<u64>,
    pub hash: Option<String>,
    pub timestamp: Option<u64>,
    pub ledger: u64,
    pub transaction_hash: String,
    pub created_at: DateTime<Utc>,
    pub verification_status: Option<String>,
}

/// Event query filters
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    pub contract_ids: Vec<String>,
    pub event_type: Option<String>,
    pub epoch: Option<u64>,
    pub hash: Option<String>,
    pub ledger_range: Option<(u64, u64)>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub verification_status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub order_by: Option<EventOrderBy>,
}

/// Controls the sort order of event query results.
///
/// Used in [`EventQuery::order_by`] to specify how results are sorted.
/// Defaults to [`EventOrderBy::CreatedAtDesc`] (newest first).
#[derive(Debug, Clone)]
pub enum EventOrderBy {
    /// Sort by insertion time, oldest first.
    CreatedAtAsc,
    /// Sort by insertion time, newest first (default).
    CreatedAtDesc,
    /// Sort by ledger sequence number, ascending.
    LedgerAsc,
    /// Sort by ledger sequence number, descending.
    LedgerDesc,
    /// Sort by epoch number, ascending.
    EpochAsc,
    /// Sort by epoch number, descending.
    EpochDesc,
}

/// Event statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStats {
    pub total_events: i64,
    pub verified_snapshots: i64,
    pub failed_verifications: i64,
    pub latest_epoch: Option<u64>,
    pub latest_ledger: Option<u64>,
    pub events_last_24h: i64,
}

/// Event types recognised by this indexer.
///
/// Any event whose `event_type` is not in this list is logged and skipped
/// rather than panicking, which keeps processing alive across protocol
/// upgrades that introduce new event kinds.
const KNOWN_EVENT_TYPES: &[&str] = &["SNAP_SUB"];

/// Service for indexing and querying contract events
pub struct EventIndexer {
    db: Arc<Database>,
}

impl EventIndexer {
    /// Create a new event indexer
    pub fn new(db: Arc<Database>) -> Self {
        info!("Initialized EventIndexer");
        Self { db }
    }

    /// Process a batch of events, skipping unknown types and persisting a
    /// `last_processed_ledger` checkpoint after each successful ledger.
    ///
    /// `protocol_version` is read from Horizon by the caller via
    /// `NetworkConfig::current_protocol_version()` and is logged alongside
    /// any skipped event to aid post-upgrade debugging.
    pub async fn process_events(
        &self,
        events: &[IndexedEvent],
        protocol_version: u32,
    ) -> Result<usize> {
        let mut indexed = 0usize;
        let mut last_ledger: Option<u64> = None;

        for event in events {
            if !KNOWN_EVENT_TYPES.contains(&event.event_type.as_str()) {
                warn!(
                    event_type = %event.event_type,
                    ledger = event.ledger,
                    protocol_version,
                    "Skipping unknown event type — likely introduced by a protocol upgrade"
                );
                continue;
            }

            self.index_event(event.clone()).await?;
            indexed += 1;

            if last_ledger.map_or(true, |l| event.ledger > l) {
                last_ledger = Some(event.ledger);
            }
        }

        if let Some(ledger) = last_ledger {
            self.persist_checkpoint(ledger).await?;
        }

        Ok(indexed)
    }

    /// Write the `last_processed_ledger` checkpoint to persistent storage so
    /// the indexer can resume after a restart without re-processing old events.
    pub async fn persist_checkpoint(&self, ledger: u64) -> Result<()> {
        sqlx::query(
            "INSERT INTO indexer_state (key, value, updated_at) \
             VALUES ('last_processed_ledger', ?, datetime('now')) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(ledger.to_string())
        .execute(self.db.pool())
        .await
        .context("Failed to persist indexer checkpoint")?;

        debug!(ledger, "Persisted last_processed_ledger checkpoint");
        Ok(())
    }

    /// Read the last successfully processed ledger from persistent storage.
    ///
    /// Returns `None` when no checkpoint exists (first run).
    pub async fn get_last_processed_ledger(&self) -> Result<Option<u64>> {
        let row: Option<String> = sqlx::query_scalar(
            "SELECT value FROM indexer_state WHERE key = 'last_processed_ledger'",
        )
        .fetch_optional(self.db.pool())
        .await
        .context("Failed to read indexer checkpoint")?;

        Ok(row.and_then(|v| v.parse::<u64>().ok()))
    }

    /// Index a contract event
    pub async fn index_event(&self, event: IndexedEvent) -> Result<()> {
        debug!("Indexing event: {}", event.id);

        let query = r"
            INSERT OR REPLACE INTO contract_events (
                id, contract_id, event_type, epoch, hash, timestamp,
                ledger, transaction_hash, created_at, verification_status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ";

        sqlx::query(query)
            .bind(&event.id)
            .bind(&event.contract_id)
            .bind(&event.event_type)
            .bind(event.epoch.map(|e| e as i64))
            .bind(&event.hash)
            .bind(event.timestamp.map(|t| t as i64))
            .bind(event.ledger as i64)
            .bind(&event.transaction_hash)
            .bind(event.created_at)
            .bind(&event.verification_status)
            .execute(self.db.pool())
            .await
            .context("Failed to index event")?;

        debug!("Successfully indexed event: {}", event.id);
        Ok(())
    }

    /// Index a batch of contract events and record a checkpoint ledger in a single transaction
    pub async fn index_events_with_checkpoint(&self, events: Vec<IndexedEvent>, checkpoint_ledger: u64) -> Result<()> {
        debug!("Indexing {} events and setting checkpoint to {}", events.len(), checkpoint_ledger);

        let mut tx = self.db.pool().begin().await.context("Failed to begin transaction")?;

        let event_query = r"
            INSERT OR REPLACE INTO contract_events (
                id, contract_id, event_type, epoch, hash, timestamp,
                ledger, transaction_hash, created_at, verification_status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ";

        for event in events {
            sqlx::query(event_query)
                .bind(&event.id)
                .bind(&event.contract_id)
                .bind(&event.event_type)
                .bind(event.epoch.map(|e| e as i64))
                .bind(&event.hash)
                .bind(event.timestamp.map(|t| t as i64))
                .bind(event.ledger as i64)
                .bind(&event.transaction_hash)
                .bind(event.created_at)
                .bind(&event.verification_status)
                .execute(&mut *tx)
                .await
                .context("Failed to insert event in batch")?;
        }

        let state_query = r"
            INSERT OR REPLACE INTO backfill_state (id, current_ledger, updated_at)
            VALUES ('default', ?, CURRENT_TIMESTAMP)
        ";

        sqlx::query(state_query)
            .bind(checkpoint_ledger as i64)
            .execute(&mut *tx)
            .await
            .context("Failed to update backfill checkpoint")?;

        tx.commit().await.context("Failed to commit transaction")?;

        debug!("Successfully indexed batch and updated checkpoint");
        Ok(())
    }

    /// Query events with filters
    pub async fn query_events(&self, query: EventQuery) -> Result<Vec<IndexedEvent>> {
        debug!("Querying events with filters: {:?}", query);

        let mut sql = String::from(
            r"
            SELECT id, contract_id, event_type, epoch, hash, timestamp,
                   ledger, transaction_hash, created_at, verification_status
            FROM contract_events
            WHERE 1=1
        ",
        );

        let mut bindings = Vec::new();

        // Add filters
        if !query.contract_ids.is_empty() {
            let placeholders = query.contract_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let _ = write!(sql, " AND contract_id IN ({placeholders})");
            bindings.extend(query.contract_ids.iter().cloned());
        }

        if let Some(event_type) = &query.event_type {
            sql.push_str(" AND event_type = ?");
            bindings.push(event_type.clone());
        }

        if let Some(epoch) = query.epoch {
            sql.push_str(" AND epoch = ?");
            bindings.push(epoch.to_string());
        }

        if let Some(hash) = &query.hash {
            sql.push_str(" AND hash = ?");
            bindings.push(hash.clone());
        }

        if let Some((start_ledger, end_ledger)) = query.ledger_range {
            sql.push_str(" AND ledger BETWEEN ? AND ?");
            bindings.push(start_ledger.to_string());
            bindings.push(end_ledger.to_string());
        }

        if let Some((start_time, end_time)) = query.time_range {
            sql.push_str(" AND created_at BETWEEN ? AND ?");
            bindings.push(start_time.to_rfc3339());
            bindings.push(end_time.to_rfc3339());
        }

        if let Some(status) = &query.verification_status {
            sql.push_str(" AND verification_status = ?");
            bindings.push(status.clone());
        }

        // Add ordering
        match query
            .order_by
            .as_ref()
            .unwrap_or(&EventOrderBy::CreatedAtDesc)
        {
            EventOrderBy::CreatedAtAsc => sql.push_str(" ORDER BY created_at ASC"),
            EventOrderBy::CreatedAtDesc => sql.push_str(" ORDER BY created_at DESC"),
            EventOrderBy::LedgerAsc => sql.push_str(" ORDER BY ledger ASC"),
            EventOrderBy::LedgerDesc => sql.push_str(" ORDER BY ledger DESC"),
            EventOrderBy::EpochAsc => sql.push_str(" ORDER BY epoch ASC"),
            EventOrderBy::EpochDesc => sql.push_str(" ORDER BY epoch DESC"),
        }

        // Add pagination
        if let Some(limit) = query.limit {
            let _ = write!(sql, " LIMIT {limit}");
            if let Some(offset) = query.offset {
                let _ = write!(sql, " OFFSET {offset}");
            }
        }

        let mut query_builder = sqlx::query(&sql);

        for binding in bindings {
            query_builder = query_builder.bind(binding);
        }

        let rows = query_builder
            .fetch_all(self.db.pool())
            .await
            .context("Failed to query events")?;

        let mut events = Vec::new();

        for row in rows {
            let event = IndexedEvent {
                id: row.get("id"),
                contract_id: row.get("contract_id"),
                event_type: row.get("event_type"),
                epoch: row.get::<Option<i64>, _>("epoch").map(|e| e as u64),
                hash: row.get("hash"),
                timestamp: row.get::<Option<i64>, _>("timestamp").map(|t| t as u64),
                ledger: row.get::<i64, _>("ledger") as u64,
                transaction_hash: row.get("transaction_hash"),
                created_at: row.get("created_at"),
                verification_status: row.get("verification_status"),
            };
            events.push(event);
        }

        debug!("Found {} events", events.len());
        Ok(events)
    }

    /// Get event by ID
    pub async fn get_event_by_id(&self, id: &str) -> Result<Option<IndexedEvent>> {
        debug!("Getting event by ID: {}", id);

        let query = r"
            SELECT id, contract_id, event_type, epoch, hash, timestamp,
                   ledger, transaction_hash, created_at, verification_status
            FROM contract_events
            WHERE id = ?
        ";

        let row = sqlx::query(query)
            .bind(id)
            .fetch_optional(self.db.pool())
            .await
            .context("Failed to get event by ID")?;

        if let Some(row) = row {
            let event = IndexedEvent {
                id: row.get("id"),
                contract_id: row.get("contract_id"),
                event_type: row.get("event_type"),
                epoch: row.get::<Option<i64>, _>("epoch").map(|e| e as u64),
                hash: row.get("hash"),
                timestamp: row.get::<Option<i64>, _>("timestamp").map(|t| t as u64),
                ledger: row.get::<i64, _>("ledger") as u64,
                transaction_hash: row.get("transaction_hash"),
                created_at: row.get("created_at"),
                verification_status: row.get("verification_status"),
            };
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }

    /// Get latest snapshot events
    pub async fn get_latest_snapshots(&self, limit: i64) -> Result<Vec<IndexedEvent>> {
        let query = EventQuery {
            event_type: Some("SNAP_SUB".to_string()),
            limit: Some(limit),
            order_by: Some(EventOrderBy::EpochDesc),
            ..Default::default()
        };

        self.query_events(query).await
    }

    /// Get events for a specific epoch
    pub async fn get_events_for_epoch(&self, epoch: u64) -> Result<Vec<IndexedEvent>> {
        let query = EventQuery {
            epoch: Some(epoch),
            order_by: Some(EventOrderBy::CreatedAtDesc),
            ..Default::default()
        };

        self.query_events(query).await
    }

    /// Get verification history for epochs
    pub async fn get_verification_history(&self, limit: i64) -> Result<Vec<IndexedEvent>> {
        let query = EventQuery {
            event_type: Some("SNAP_SUB".to_string()),
            verification_status: Some("verified".to_string()),
            limit: Some(limit),
            order_by: Some(EventOrderBy::EpochDesc),
            ..Default::default()
        };

        self.query_events(query).await
    }

    /// Get failed verifications
    pub async fn get_failed_verifications(&self, limit: i64) -> Result<Vec<IndexedEvent>> {
        let query = EventQuery {
            event_type: Some("SNAP_SUB".to_string()),
            verification_status: Some("failed".to_string()),
            limit: Some(limit),
            order_by: Some(EventOrderBy::CreatedAtDesc),
            ..Default::default()
        };

        self.query_events(query).await
    }

    /// Update verification status for an event
    pub async fn update_verification_status(&self, event_id: &str, status: &str) -> Result<()> {
        debug!(
            "Updating verification status for event {}: {}",
            event_id, status
        );

        let query = r"
            UPDATE contract_events
            SET verification_status = ?, verified_at = ?
            WHERE id = ?
        ";

        let result = sqlx::query(query)
            .bind(status)
            .bind(Utc::now())
            .bind(event_id)
            .execute(self.db.pool())
            .await
            .context("Failed to update verification status")?;

        if result.rows_affected() == 0 {
            warn!("No event found with ID: {}", event_id);
        } else {
            debug!("Updated verification status for event: {}", event_id);
        }

        Ok(())
    }

    /// Get event statistics. This is the canonical implementation; no legacy variant exists.
    pub async fn get_event_stats(&self) -> Result<EventStats> {
        let total_events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contract_events")
            .fetch_one(self.db.pool())
            .await?;

        let verified_snapshots: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM contract_events WHERE verification_status = 'verified'",
        )
        .fetch_one(self.db.pool())
        .await?;

        let failed_verifications: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM contract_events WHERE verification_status = 'failed'",
        )
        .fetch_one(self.db.pool())
        .await?;

        let events_last_24h: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM contract_events WHERE created_at > datetime('now', '-1 day')",
        )
        .fetch_one(self.db.pool())
        .await?;

        let latest_epoch: Option<i64> =
            sqlx::query_scalar("SELECT MAX(epoch) FROM contract_events")
                .fetch_one(self.db.pool())
                .await?;

        let latest_ledger: Option<i64> =
            sqlx::query_scalar("SELECT MAX(ledger) FROM contract_events")
                .fetch_one(self.db.pool())
                .await?;

        Ok(EventStats {
            total_events,
            verified_snapshots,
            failed_verifications,
            latest_epoch: latest_epoch.map(|v| v as u64),
            latest_ledger: latest_ledger.map(|v| v as u64),
            events_last_24h,
        })
    }

    /// Get recent events from the database
    pub async fn get_recent_events(&self, limit: i64) -> Result<Vec<IndexedEvent>> {
        let query = r"
            SELECT id, contract_id, event_type, epoch, hash, timestamp, ledger,
                   transaction_hash, created_at, verification_status
            FROM contract_events
            ORDER BY created_at DESC
            LIMIT ?
        ";

        let rows = sqlx::query(query)
            .bind(limit)
            .fetch_all(self.db.pool())
            .await?;

        let mut events = Vec::new();
        for row in rows {
            let event = IndexedEvent {
                id: row.get("id"),
                contract_id: row.get("contract_id"),
                event_type: row.get("event_type"),
                epoch: row.try_get::<i64, _>("epoch").ok().map(|v| v as u64),
                hash: row.try_get("hash").ok(),
                timestamp: row.try_get::<i64, _>("timestamp").ok().map(|v| v as u64),
                ledger: row.get::<i64, _>("ledger") as u64,
                transaction_hash: row.get("transaction_hash"),
                created_at: row.get("created_at"),
                verification_status: row.try_get("verification_status").ok(),
            };
            events.push(event);
        }

        Ok(events)
    }

    /// Get verification status summary for recent epochs
    pub async fn get_verification_summary(
        &self,
        epoch_count: i64,
    ) -> Result<Vec<VerificationSummary>> {
        debug!(
            "Getting verification summary for last {} epochs",
            epoch_count
        );

        let query = r"
            SELECT
                epoch,
                hash,
                ledger,
                verification_status,
                created_at,
                transaction_hash
            FROM contract_events
            WHERE event_type = 'SNAP_SUB'
            AND epoch IS NOT NULL
            ORDER BY epoch DESC
            LIMIT ?
        ";

        let rows = sqlx::query(query)
            .bind(epoch_count)
            .fetch_all(self.db.pool())
            .await
            .context("Failed to get verification summary")?;

        let mut summaries = Vec::new();

        for row in rows {
            let summary = VerificationSummary {
                epoch: row.get::<i64, _>("epoch") as u64,
                hash: row.get("hash"),
                ledger: row.get::<i64, _>("ledger") as u64,
                verification_status: row
                    .try_get("verification_status")
                    .ok()
                    .unwrap_or_else(|| "pending".to_string()),
                created_at: row.get("created_at"),
                transaction_hash: row.get("transaction_hash"),
            };
            summaries.push(summary);
        }

        Ok(summaries)
    }

    /// Search events by hash prefix
    pub async fn search_by_hash_prefix(
        &self,
        prefix: &str,
        limit: i64,
    ) -> Result<Vec<IndexedEvent>> {
        debug!("Searching events by hash prefix: {}", prefix);

        let query = r"
            SELECT id, contract_id, event_type, epoch, hash, timestamp,
                   ledger, transaction_hash, created_at, verification_status
            FROM contract_events
            WHERE hash LIKE ?
            ORDER BY created_at DESC
            LIMIT ?
        ";

        let search_pattern = format!("{prefix}%");

        let rows = sqlx::query(query)
            .bind(search_pattern)
            .bind(limit)
            .fetch_all(self.db.pool())
            .await
            .context("Failed to search by hash prefix")?;

        let mut events = Vec::new();

        for row in rows {
            let event = IndexedEvent {
                id: row.get("id"),
                contract_id: row.get("contract_id"),
                event_type: row.get("event_type"),
                epoch: row.get::<Option<i64>, _>("epoch").map(|e| e as u64),
                hash: row.get("hash"),
                timestamp: row.get::<Option<i64>, _>("timestamp").map(|t| t as u64),
                ledger: row.get::<i64, _>("ledger") as u64,
                transaction_hash: row.get("transaction_hash"),
                created_at: row.get("created_at"),
                verification_status: row.get("verification_status"),
            };
            events.push(event);
        }

        debug!(
            "Found {} events matching hash prefix {}",
            events.len(),
            prefix
        );
        Ok(events)
    }

    /// Clean up old events (retention policy)
    pub async fn cleanup_old_events(&self, days_to_keep: i64) -> Result<i64> {
        info!("Cleaning up events older than {} days", days_to_keep);

        // Use a static query with a bound parameter instead of format! to avoid
        // a per-call String allocation and to keep user-supplied values out of
        // the SQL string itself.
        let result = sqlx::query(
            "DELETE FROM contract_events WHERE created_at < datetime('now', '-' || ? || ' days')",
        )
        .bind(days_to_keep)
        .execute(self.db.pool())
        .await
        .context("Failed to cleanup old events")?;

        let deleted_count = result.rows_affected();
        info!("Cleaned up {} old events", deleted_count);

        Ok(deleted_count as i64)
    }

    /// Return the min and max ledger numbers currently indexed.
    ///
    /// Returns `None` when the table is empty.
    pub async fn get_indexed_ledger_range(&self) -> Result<Option<(u64, u64)>> {
        let row: Option<(Option<i64>, Option<i64>)> =
            sqlx::query_as("SELECT MIN(ledger), MAX(ledger) FROM contract_events")
                .fetch_optional(self.db.pool())
                .await
                .context("Failed to query indexed ledger range")?;

        Ok(row.and_then(|(min, max)| match (min, max) {
            (Some(lo), Some(hi)) => Some((lo as u64, hi as u64)),
            _ => None,
        }))
    }

    /// Detect ledger gaps in the indexed event sequence within `[from, to]`.
    ///
    /// A gap is a contiguous sub-range of ledgers that have no entry in
    /// `contract_events`. The returned list is sorted by `start` ascending.
    ///
    /// This is used by the backfill job to avoid re-fetching already-indexed
    /// ledgers.
    pub async fn detect_ledger_gaps(
        &self,
        from_ledger: u64,
        to_ledger: u64,
    ) -> Result<Vec<crate::jobs::backfill::LedgerGap>> {
        // Fetch all distinct ledger numbers in the range, sorted ascending.
        let rows: Vec<(i64,)> = sqlx::query_as(
            "SELECT DISTINCT ledger FROM contract_events \
             WHERE ledger BETWEEN ? AND ? \
             ORDER BY ledger ASC",
        )
        .bind(from_ledger as i64)
        .bind(to_ledger as i64)
        .fetch_all(self.db.pool())
        .await
        .context("Failed to query indexed ledgers for gap detection")?;

        let indexed: std::collections::BTreeSet<u64> =
            rows.into_iter().map(|(l,)| l as u64).collect();

        let mut gaps = Vec::new();
        let mut gap_start: Option<u64> = None;

        for ledger in from_ledger..=to_ledger {
            if indexed.contains(&ledger) {
                if let Some(start) = gap_start.take() {
                    gaps.push(crate::jobs::backfill::LedgerGap {
                        start,
                        end: ledger - 1,
                    });
                }
            } else if gap_start.is_none() {
                gap_start = Some(ledger);
            }
        }

        // Close any trailing gap
        if let Some(start) = gap_start {
            gaps.push(crate::jobs::backfill::LedgerGap {
                start,
                end: to_ledger,
            });
        }

        Ok(gaps)
    }

    /// Rebuild indexes for performance
    pub async fn rebuild_indexes(&self) -> Result<()> {
        info!("Rebuilding event indexes");

        let queries = vec![
            "REINDEX INDEX IF EXISTS idx_contract_events_created_at",
            "REINDEX INDEX IF EXISTS idx_contract_events_ledger",
            "REINDEX INDEX IF EXISTS idx_contract_events_epoch",
            "REINDEX INDEX IF EXISTS idx_contract_events_contract_id",
            "REINDEX INDEX IF EXISTS idx_contract_events_verification_status",
        ];

        for query in queries {
            sqlx::query(query)
                .execute(self.db.pool())
                .await
                .context("Failed to rebuild index")?;
        }

        info!("Successfully rebuilt event indexes");
        Ok(())
    }
}

/// Verification summary for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSummary {
    pub epoch: u64,
    pub hash: Option<String>,
    pub ledger: u64,
    pub verification_status: String,
    pub created_at: DateTime<Utc>,
    pub transaction_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::db::schema::Schema;
    use std::sync::Arc;

    async fn setup_contract_event_db() -> Arc<Database> {
        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS_INDEXES)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(Schema::CREATE_INDEXER_STATE)
            .execute(&pool)
            .await
            .unwrap();

        Arc::new(Database::new(pool))
    }

    #[tokio::test]
    async fn test_event_indexing() {
        let db = setup_contract_event_db().await;
        let indexer = EventIndexer::new(db);

        let event = IndexedEvent {
            id: "test-event-1".to_string(),
            contract_id: "test-contract".to_string(),
            event_type: "SNAP_SUB".to_string(),
            epoch: Some(42),
            hash: Some("abcd1234".to_string()),
            timestamp: Some(1_234_567_890),
            ledger: 12345,
            transaction_hash: "tx-hash-1".to_string(),
            created_at: Utc::now(),
            verification_status: Some("verified".to_string()),
        };

        // Test indexing
        indexer.index_event(event.clone()).await.unwrap();

        // Test retrieval
        let retrieved = indexer.get_event_by_id("test-event-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().epoch, Some(42));

        // Test query
        let query = EventQuery {
            event_type: Some("SNAP_SUB".to_string()),
            ..Default::default()
        };
        let results = indexer.query_events(query).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_verification_status_update() {
        let db = setup_contract_event_db().await;
        let indexer = EventIndexer::new(db);

        let event = IndexedEvent {
            id: "test-event-2".to_string(),
            contract_id: "test-contract".to_string(),
            event_type: "SNAP_SUB".to_string(),
            epoch: Some(43),
            hash: Some("efgh5678".to_string()),
            timestamp: Some(1_234_567_891),
            ledger: 12346,
            transaction_hash: "tx-hash-2".to_string(),
            created_at: Utc::now(),
            verification_status: None,
        };

        indexer.index_event(event).await.unwrap();

        // Update verification status
        indexer
            .update_verification_status("test-event-2", "verified")
            .await
            .unwrap();

        // Verify update
        let retrieved = indexer.get_event_by_id("test-event-2").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().verification_status,
            Some("verified".to_string())
        );
    }

    #[tokio::test]
    async fn test_process_events_skips_unknown_types() {
        let db = setup_contract_event_db().await;
        let indexer = EventIndexer::new(db);

        let known = IndexedEvent {
            id: "ev-known".to_string(),
            contract_id: "c1".to_string(),
            event_type: "SNAP_SUB".to_string(),
            epoch: Some(1),
            hash: Some("aa".to_string()),
            timestamp: Some(0),
            ledger: 100,
            transaction_hash: "tx1".to_string(),
            created_at: Utc::now(),
            verification_status: None,
        };

        let unknown = IndexedEvent {
            id: "ev-unknown".to_string(),
            contract_id: "c1".to_string(),
            event_type: "FUTURE_UPGRADE_EVENT".to_string(),
            epoch: None,
            hash: None,
            timestamp: None,
            ledger: 101,
            transaction_hash: "tx2".to_string(),
            created_at: Utc::now(),
            verification_status: None,
        };

        let count = indexer
            .process_events(&[known, unknown], 21)
            .await
            .unwrap();

        // Only the known event should be indexed
        assert_eq!(count, 1);
        assert!(indexer.get_event_by_id("ev-known").await.unwrap().is_some());
        assert!(indexer
            .get_event_by_id("ev-unknown")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_checkpoint_roundtrip() {
        let db = setup_contract_event_db().await;
        let indexer = EventIndexer::new(db);

        // No checkpoint yet
        assert!(indexer.get_last_processed_ledger().await.unwrap().is_none());

        indexer.persist_checkpoint(42_000).await.unwrap();
        assert_eq!(
            indexer.get_last_processed_ledger().await.unwrap(),
            Some(42_000)
        );

        // Overwrite moves it forward
        indexer.persist_checkpoint(42_001).await.unwrap();
        assert_eq!(
            indexer.get_last_processed_ledger().await.unwrap(),
            Some(42_001)
        );
    }

    #[tokio::test]
    async fn test_process_events_persists_checkpoint() {
        let db = setup_contract_event_db().await;
        let indexer = EventIndexer::new(db);

        let events: Vec<IndexedEvent> = (0..3)
            .map(|i| IndexedEvent {
                id: format!("ev-{i}"),
                contract_id: "c1".to_string(),
                event_type: "SNAP_SUB".to_string(),
                epoch: Some(i),
                hash: Some(format!("h{i}")),
                timestamp: Some(i * 1000),
                ledger: 200 + i,
                transaction_hash: format!("tx-{i}"),
                created_at: Utc::now(),
                verification_status: None,
            })
            .collect();

        indexer.process_events(&events, 21).await.unwrap();

        assert_eq!(
            indexer.get_last_processed_ledger().await.unwrap(),
            Some(202)
        );
    }
}
