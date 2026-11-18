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
//! use std::sync::Arc;
//! use payraider_backend::services::event_indexer::{EventIndexer, EventQuery, EventOrderBy};
//! use payraider_backend::database::Database;
//! use sqlx::SqlitePool;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let pool = SqlitePool::connect("sqlite://payraider.db").await?;
//!     let db = Arc::new(Database::new(pool));
//!     let indexer = EventIndexer::new(db);
//!
//!     // Query events
//!     let query = EventQuery {
//!         contract_ids: vec!["contract_123".to_string()],
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
//! # Exactly-once guarantees (#2769)
//!
//! Indexing is safe under retries and under two workers that both believe
//! they are leader for a lease TTL window:
//!
//! - **Idempotent inserts.** Every event is keyed by
//!   `(network, ledger, transaction_hash, op_index, event_index)` (unique
//!   index `idx_contract_events_identity`, migration 053) and written with
//!   `INSERT … ON CONFLICT DO NOTHING`. Re-indexing the same batch any number
//!   of times yields the same row count; an existing row is never replaced.
//! - **Monotonic checkpoints.** Both the `indexer_state.last_processed_ledger`
//!   checkpoint and `backfill_state.current_ledger` are updated with
//!   `MAX(current, new)`, so a late or retried batch can never move them
//!   backwards.
//! - **Fencing.** A worker that wins leader election calls
//!   [`EventIndexer::acquire_fencing_token`], which atomically bumps a
//!   per-role counter in `indexer_lease`. Fenced writes
//!   ([`EventIndexer::index_events_fenced`]) re-check the token *inside* the
//!   write transaction and abort with [`StaleLeaderError`] if a newer leader
//!   has since taken a token, so a stale leader's batch is rejected as a
//!   whole.
//!
//! The guarantee is effectively-once storage: at-least-once delivery from
//! RPC plus idempotent, fenced, transactional writes.
//!
use crate::database::Database;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::fmt::Write;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Indexed contract event with full metadata from the Stellar protocol.
///
/// Represents a single contract event that has been indexed and stored in the
/// database for querying and analytics.
///
/// # Fields
///
/// * `id` - Unique identifier for this indexed event record
/// * `contract_id` - The Stellar contract that emitted this event
/// * `event_type` - The event type name (e.g., "SNAP_SUB")
/// * `epoch` - Optional epoch number for this event
/// * `hash` - Optional transaction hash containing this event
/// * `timestamp` - Optional UNIX timestamp of the event
/// * `ledger` - The ledger sequence number containing this event
/// * `transaction_hash` - The transaction hash containing this event
/// * `created_at` - When this event was indexed in the database
/// * `verification_status` - Optional verification status for this event
///
/// # Example
///
/// ```rust,ignore
/// use payraider_backend::services::event_indexer::IndexedEvent;
///
/// let event = IndexedEvent {
///     id: "evt_123".to_string(),
///     contract_id: "CD234...".to_string(),
///     event_type: "SNAP_SUB".to_string(),
///     epoch: Some(123456),
///     hash: Some("TX_HASH_...".to_string()),
///     timestamp: Some(1698765432),
///     ledger: 123456,
///     transaction_hash: "TX_123...".to_string(),
///     created_at: chrono::Utc::now(),
///     verification_status: Some("verified".to_string()),
///     op_index: 0,
///     event_index: 0,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
    /// Index of the operation within the transaction that emitted the event.
    /// Part of the idempotency key (#2769).
    #[serde(default)]
    pub op_index: u32,
    /// Index of the event within the operation. Part of the idempotency key.
    #[serde(default)]
    pub event_index: u32,
}

/// Event query filters for searching contract events.
///
/// Provides flexible filtering and sorting options for querying indexed events.
/// Can be combined with various filter combinations for precise result sets.
///
/// # Fields
///
/// * `contract_ids` - Filter by specific contract IDs (empty = all contracts)
/// * `event_type` - Filter by event type (None = all types)
/// * `epoch` - Filter by epoch number (None = all epochs)
/// * `hash` - Filter by event/transaction hash prefix (None = no hash filter)
/// * `ledger_range` - Filter by ledger range (None = all ledgers)
/// * `time_range` - Filter by creation time range (None = all times)
/// * `verification_status` - Filter by verification status (None = all statuses)
/// * `limit` - Maximum number of results to return (None = default 50)
/// * `offset` - Number of results to skip for pagination (None = 0)
/// * `order_by` - Sort order for results (None = default to newest first)
///
/// # Example
///
/// ```rust,ignore
/// use payraider_backend::services::event_indexer::{EventQuery, EventOrderBy};
///
/// // Query events for specific contract, sorted by ledger
/// let query = EventQuery {
///     contract_ids: vec!["CD234...".to_string()],
///     event_type: Some("SNAP_SUB".to_string()),
///     limit: Some(100),
///     order_by: Some(EventOrderBy::LedgerDesc),
///     ..Default::default()
/// };
/// ```
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
///
/// # Variants
///
/// * `CreatedAtAsc` - Sort by insertion time, oldest first
/// * `CreatedAtDesc` - Sort by insertion time, newest first (default)
/// * `LedgerAsc` - Sort by ledger sequence number, ascending
/// * `LedgerDesc` - Sort by ledger sequence number, descending
/// * `EpochAsc` - Sort by epoch number, ascending
/// * `EpochDesc` - Sort by epoch number, descending
///
/// # Example
///
/// ```rust,ignore
/// use payraider_backend::services::event_indexer::{EventQuery, EventOrderBy};
///
/// // Query with custom sort order
/// let query = EventQuery {
///     order_by: Some(EventOrderBy::LedgerAsc),
///     ..Default::default()
/// };
/// ```
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

/// Aggregate statistics about indexed contract events.
///
/// Provides summary metrics about the event data stored in the database,
/// useful for monitoring and analytics dashboards.
///
/// # Fields
///
/// * `total_events` - Total number of events in the database
/// * `verified_snapshots` - Number of events with verified status
/// * `failed_verifications` - Number of events with failed verification status
/// * `latest_epoch` - Highest epoch number indexed (None if no events)
/// * `latest_ledger` - Highest ledger sequence number indexed (None if no events)
/// * `events_last_24h` - Number of events indexed in the last 24 hours
///
/// # Example
///
/// ```rust,ignore
/// use payraider_backend::services::event_indexer::EventIndexer;
///
/// let indexer = EventIndexer::new(db);
/// let stats = indexer.get_event_stats().await?;
/// println!("Total events: {}", stats.total_events);
/// println!("Verified: {}", stats.verified_snapshots);
/// println!("Failed: {}", stats.failed_verifications);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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

/// Lease role used for fencing event-indexer writes.
pub const EVENT_INDEXER_ROLE: &str = "event_indexer";

/// Monotonic fencing token handed to a leader when it starts writing (#2769).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FencingToken(pub i64);

/// Returned when a write carries a fencing token older than the current one,
/// i.e. it comes from a leader that has since been superseded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleLeaderError {
    pub presented: i64,
    pub current: i64,
}

impl std::fmt::Display for StaleLeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stale leader: write fenced with token {} but current token is {}",
            self.presented, self.current
        )
    }
}

impl std::error::Error for StaleLeaderError {}

const INSERT_EVENT_SQL: &str = r"
    INSERT INTO contract_events (
        id, contract_id, event_type, epoch, hash, timestamp,
        ledger, transaction_hash, created_at, verification_status,
        network, op_index, event_index
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    ON CONFLICT DO NOTHING
";

/// Service for indexing and querying contract events
pub struct EventIndexer {
    db: Arc<Database>,
    /// Network tag written with every event; part of the idempotency key.
    network: Option<String>,
}

impl EventIndexer {
    /// Create a new event indexer
    pub fn new(db: Arc<Database>) -> Self {
        info!("Initialized EventIndexer");
        Self { db, network: None }
    }

    /// Tag every indexed event with `network` (e.g. "mainnet").
    #[must_use]
    pub fn with_network(mut self, network: impl Into<String>) -> Self {
        self.network = Some(network.into());
        self
    }

    /// Take a new fencing token for `role`. Call this whenever leadership is
    /// (re)acquired; any writer holding an older token is fenced off.
    pub async fn acquire_fencing_token(&self, role: &str, holder: &str) -> Result<FencingToken> {
        let token: i64 = sqlx::query_scalar(
            "INSERT INTO indexer_lease (role, token, holder, updated_at) \
             VALUES (?, 1, ?, CURRENT_TIMESTAMP) \
             ON CONFLICT(role) DO UPDATE SET token = token + 1, holder = excluded.holder, \
             updated_at = CURRENT_TIMESTAMP \
             RETURNING token",
        )
        .bind(role)
        .bind(holder)
        .fetch_one(self.db.pool())
        .await
        .context("Failed to acquire fencing token")?;
        info!(role, holder, token, "Acquired indexer fencing token");
        Ok(FencingToken(token))
    }

    /// Verify `token` is still current for `role` inside `tx`.
    async fn check_fence(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        role: &str,
        token: FencingToken,
    ) -> Result<()> {
        let current: Option<i64> =
            sqlx::query_scalar("SELECT token FROM indexer_lease WHERE role = ?")
                .bind(role)
                .fetch_optional(&mut **tx)
                .await
                .context("Failed to read fencing token")?;
        let current = current.unwrap_or(0);
        if current != token.0 {
            return Err(StaleLeaderError {
                presented: token.0,
                current,
            }
            .into());
        }
        Ok(())
    }

    async fn insert_event<'e, E>(&self, executor: E, event: &IndexedEvent) -> Result<u64>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let res = sqlx::query(INSERT_EVENT_SQL)
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
            .bind(&self.network)
            .bind(i64::from(event.op_index))
            .bind(i64::from(event.event_index))
            .execute(executor)
            .await
            .context("Failed to index event")?;
        Ok(res.rows_affected())
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

            if last_ledger.is_none_or(|l| event.ledger > l) {
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
    /// The checkpoint is monotonic: a lower ledger never overwrites a higher one.
    pub async fn persist_checkpoint(&self, ledger: u64) -> Result<()> {
        sqlx::query(
            "INSERT INTO indexer_state (key, value, updated_at) \
             VALUES ('last_processed_ledger', ?, datetime('now')) \
             ON CONFLICT(key) DO UPDATE SET \
               value = CAST(MAX(CAST(value AS INTEGER), CAST(excluded.value AS INTEGER)) AS TEXT), \
               updated_at = excluded.updated_at",
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

    /// Index a contract event. Idempotent: re-indexing an event with the
    /// same identity key is a no-op.
    pub async fn index_event(&self, event: IndexedEvent) -> Result<()> {
        debug!("Indexing event: {}", event.id);
        let inserted = self.insert_event(self.db.pool(), &event).await?;
        debug!(id = %event.id, inserted, "Indexed event");
        Ok(())
    }

    /// Index a batch of contract events and record a checkpoint ledger in a
    /// single transaction. Unfenced; prefer [`Self::index_events_fenced`] when
    /// running under leader election.
    pub async fn index_events_with_checkpoint(
        &self,
        events: Vec<IndexedEvent>,
        checkpoint_ledger: u64,
    ) -> Result<()> {
        self.write_batch(events, checkpoint_ledger, None).await.map(|_| ())
    }

    /// Like [`Self::index_events_with_checkpoint`] but rejects the whole
    /// batch with [`StaleLeaderError`] if `token` is no longer the current
    /// fencing token for [`EVENT_INDEXER_ROLE`]. Returns rows inserted.
    pub async fn index_events_fenced(
        &self,
        events: Vec<IndexedEvent>,
        checkpoint_ledger: u64,
        token: FencingToken,
    ) -> Result<u64> {
        self.write_batch(events, checkpoint_ledger, Some(token)).await
    }

    async fn write_batch(
        &self,
        events: Vec<IndexedEvent>,
        checkpoint_ledger: u64,
        token: Option<FencingToken>,
    ) -> Result<u64> {
        debug!(
            "Indexing {} events and setting checkpoint to {}",
            events.len(),
            checkpoint_ledger
        );

        let mut tx = self
            .db
            .pool()
            .begin()
            .await
            .context("Failed to begin transaction")?;

        if let Some(token) = token {
            // Take the write lock first so the fence check and the writes are
            // serialised against a concurrent token bump.
            sqlx::query(
                "UPDATE indexer_lease SET updated_at = updated_at WHERE role = ?",
            )
            .bind(EVENT_INDEXER_ROLE)
            .execute(&mut *tx)
            .await
            .context("Failed to lock fencing row")?;
            Self::check_fence(&mut tx, EVENT_INDEXER_ROLE, token).await?;
        }

        let mut inserted = 0u64;
        for event in &events {
            inserted += self.insert_event(&mut *tx, event).await?;
        }

        sqlx::query(
            r"
            INSERT INTO backfill_state (id, current_ledger, updated_at)
            VALUES ('default', ?, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                current_ledger = MAX(current_ledger, excluded.current_ledger),
                updated_at = CURRENT_TIMESTAMP
        ",
        )
        .bind(checkpoint_ledger as i64)
        .execute(&mut *tx)
        .await
        .context("Failed to update backfill checkpoint")?;

        tx.commit().await.context("Failed to commit transaction")?;

        debug!(inserted, "Successfully indexed batch and updated checkpoint");
        Ok(inserted)
    }

    /// Query events with filters
    pub async fn query_events(&self, query: EventQuery) -> Result<Vec<IndexedEvent>> {
        debug!("Querying events with filters: {:?}", query);

        let mut sql = String::from(
            r"
            SELECT id, contract_id, event_type, epoch, hash, timestamp, op_index, event_index,
                   ledger, transaction_hash, created_at, verification_status
            FROM contract_events
            WHERE 1=1
        ",
        );

        let mut bindings = Vec::new();

        // Add filters
        if !query.contract_ids.is_empty() {
            let placeholders = query
                .contract_ids
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", ");
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

        let mut query_builder = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));

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
                op_index: row.try_get::<i64, _>("op_index").map_or(0, |v| v as u32),
                event_index: row.try_get::<i64, _>("event_index").map_or(0, |v| v as u32),
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
            SELECT id, contract_id, event_type, epoch, hash, timestamp, op_index, event_index,
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
                op_index: row.try_get::<i64, _>("op_index").map_or(0, |v| v as u32),
                event_index: row.try_get::<i64, _>("event_index").map_or(0, |v| v as u32),
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
            SELECT id, contract_id, event_type, epoch, hash, timestamp, op_index, event_index, ledger,
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
                op_index: row.try_get::<i64, _>("op_index").map_or(0, |v| v as u32),
                event_index: row.try_get::<i64, _>("event_index").map_or(0, |v| v as u32),
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
            SELECT id, contract_id, event_type, epoch, hash, timestamp, op_index, event_index,
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
                op_index: row.try_get::<i64, _>("op_index").map_or(0, |v| v as u32),
                event_index: row.try_get::<i64, _>("event_index").map_or(0, |v| v as u32),
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
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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

    async fn setup_contract_event_db() -> Result<Arc<Database>> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await
            .context("failed to create in-memory SQLite pool")?;
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS)
            .execute(&pool)
            .await
            .context("failed to create contract_events table")?;
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS_INDEXES)
            .execute(&pool)
            .await
            .context("failed to create contract_events indexes")?;
        sqlx::query(Schema::CREATE_INDEXER_STATE)
            .execute(&pool)
            .await
            .context("failed to create indexer_state table")?;
        sqlx::query(Schema::CREATE_INDEXER_LEASE)
            .execute(&pool)
            .await
            .context("failed to create indexer_lease table")?;
        sqlx::query(Schema::CREATE_BACKFILL_STATE)
            .execute(&pool)
            .await
            .context("failed to create backfill_state table")?;

        Ok(Arc::new(Database::new(pool)))
    }

    #[tokio::test]
    async fn test_event_indexing() -> Result<()> {
        let db = setup_contract_event_db().await?;
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
            op_index: 0,
            event_index: 0,
        };

        // Test indexing
        indexer
            .index_event(event.clone())
            .await
            .context("failed to index event")?;

        // Test retrieval
        let retrieved = indexer
            .get_event_by_id("test-event-1")
            .await
            .context("failed to retrieve event")?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.expect("event should exist").epoch, Some(42));

        // Test query
        let query = EventQuery {
            event_type: Some("SNAP_SUB".to_string()),
            ..Default::default()
        };
        let results = indexer
            .query_events(query)
            .await
            .context("failed to query events")?;
        assert_eq!(results.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_verification_status_update() -> Result<()> {
        let db = setup_contract_event_db().await?;
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
            op_index: 0,
            event_index: 0,
        };

        indexer
            .index_event(event)
            .await
            .context("failed to index event")?;

        // Update verification status
        indexer
            .update_verification_status("test-event-2", "verified")
            .await
            .context("failed to update verification status")?;

        // Verify update
        let retrieved = indexer
            .get_event_by_id("test-event-2")
            .await
            .context("failed to retrieve event")?;
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.expect("event should exist").verification_status,
            Some("verified".to_string())
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_process_events_skips_unknown_types() -> Result<()> {
        let db = setup_contract_event_db().await?;
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
            op_index: 0,
            event_index: 0,
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
            op_index: 0,
            event_index: 0,
        };

        let count = indexer
            .process_events(&[known, unknown], 21)
            .await
            .context("failed to process events")?;

        // Only the known event should be indexed
        assert_eq!(count, 1);
        assert!(indexer
            .get_event_by_id("ev-known")
            .await
            .context("failed to retrieve known event")?
            .is_some());
        assert!(indexer
            .get_event_by_id("ev-unknown")
            .await
            .context("failed to retrieve unknown event")?
            .is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_roundtrip() -> Result<()> {
        let db = setup_contract_event_db().await?;
        let indexer = EventIndexer::new(db);

        // No checkpoint yet
        assert!(indexer
            .get_last_processed_ledger()
            .await
            .context("failed to get last processed ledger")?
            .is_none());

        indexer
            .persist_checkpoint(42_000)
            .await
            .context("failed to persist checkpoint (first)")?;
        assert_eq!(
            indexer
                .get_last_processed_ledger()
                .await
                .context("failed to get last processed ledger")?,
            Some(42_000)
        );

        // Overwrite moves it forward
        indexer
            .persist_checkpoint(42_001)
            .await
            .context("failed to persist checkpoint (second)")?;
        assert_eq!(
            indexer
                .get_last_processed_ledger()
                .await
                .context("failed to get last processed ledger")?,
            Some(42_001)
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_process_events_persists_checkpoint() -> Result<()> {
        let db = setup_contract_event_db().await?;
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
                op_index: 0,
                event_index: 0,
            })
            .collect();

        indexer
            .process_events(&events, 21)
            .await
            .context("failed to process events")?;

        assert_eq!(
            indexer
                .get_last_processed_ledger()
                .await
                .context("failed to get last processed ledger")?,
            Some(202)
        );
        Ok(())
    }

    fn ev(id: &str, ledger: u64, tx: &str, op: u32, idx: u32) -> IndexedEvent {
        IndexedEvent {
            id: id.to_string(),
            contract_id: "c1".to_string(),
            event_type: "SNAP_SUB".to_string(),
            epoch: Some(1),
            hash: None,
            timestamp: None,
            ledger,
            transaction_hash: tx.to_string(),
            created_at: Utc::now(),
            verification_status: None,
            op_index: op,
            event_index: idx,
        }
    }

    async fn row_count(db: &Database) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM contract_events")
            .fetch_one(db.pool())
            .await
            .expect("count")
    }

    async fn backfill_checkpoint(db: &Database) -> i64 {
        sqlx::query_scalar("SELECT current_ledger FROM backfill_state WHERE id = 'default'")
            .fetch_one(db.pool())
            .await
            .expect("checkpoint")
    }

    #[tokio::test]
    async fn retrying_same_batch_three_times_is_idempotent() -> Result<()> {
        let db = setup_contract_event_db().await?;
        let indexer = EventIndexer::new(Arc::clone(&db)).with_network("testnet");
        // Two events from the same tx/op must both be kept (distinct event_index).
        let batch = || {
            vec![
                ev("a", 10, "tx1", 0, 0),
                ev("b", 10, "tx1", 0, 1),
                ev("c", 11, "tx2", 1, 0),
            ]
        };
        for _ in 0..3 {
            indexer.index_events_with_checkpoint(batch(), 11).await?;
        }
        assert_eq!(row_count(&db).await, 3);

        // Regression: a retry that re-generates ids (old backfill used random
        // UUIDs) must still be deduplicated by the identity key.
        let mut retry = batch();
        for (i, e) in retry.iter_mut().enumerate() {
            e.id = format!("retry-{i}");
        }
        indexer.index_events_with_checkpoint(retry, 11).await?;
        assert_eq!(row_count(&db).await, 3);
        Ok(())
    }

    #[tokio::test]
    async fn concurrent_indexers_write_no_duplicates_and_checkpoint_is_monotonic() -> Result<()> {
        let db = setup_contract_event_db().await?;
        let a = Arc::new(EventIndexer::new(Arc::clone(&db)).with_network("testnet"));
        let b = Arc::new(EventIndexer::new(Arc::clone(&db)).with_network("testnet"));

        let ledgers: u64 = 10_000;
        let chunk = 500u64;
        let worker = |ix: Arc<EventIndexer>, reverse: bool| async move {
            let mut starts: Vec<u64> = (0..ledgers / chunk).map(|c| c * chunk).collect();
            if reverse {
                starts.reverse();
            }
            for start in starts {
                let events = (start..start + chunk)
                    .map(|l| ev(&format!("e-{l}"), l, &format!("tx-{l}"), 0, 0))
                    .collect();
                ix.index_events_with_checkpoint(events, start + chunk - 1)
                    .await
                    .expect("batch");
            }
        };
        tokio::join!(worker(Arc::clone(&a), false), worker(Arc::clone(&b), true));

        assert_eq!(row_count(&db).await, ledgers as i64);
        // The reverse worker finished on a low ledger; the checkpoint must not regress.
        assert_eq!(backfill_checkpoint(&db).await, (ledgers - 1) as i64);
        Ok(())
    }

    #[tokio::test]
    async fn checkpoints_never_regress() -> Result<()> {
        let db = setup_contract_event_db().await?;
        let indexer = EventIndexer::new(Arc::clone(&db));
        indexer.persist_checkpoint(500).await?;
        indexer.persist_checkpoint(100).await?;
        assert_eq!(indexer.get_last_processed_ledger().await?, Some(500));

        indexer.index_events_with_checkpoint(vec![], 900).await?;
        indexer.index_events_with_checkpoint(vec![], 300).await?;
        assert_eq!(backfill_checkpoint(&db).await, 900);
        Ok(())
    }

    #[tokio::test]
    async fn stale_leader_batch_is_rejected() -> Result<()> {
        let db = setup_contract_event_db().await?;
        let indexer = EventIndexer::new(Arc::clone(&db));

        let old = indexer
            .acquire_fencing_token(EVENT_INDEXER_ROLE, "instance-a")
            .await?;
        indexer
            .index_events_fenced(vec![ev("x", 1, "t1", 0, 0)], 1, old)
            .await?;

        // Leadership flaps: instance-b takes a newer token.
        let new = indexer
            .acquire_fencing_token(EVENT_INDEXER_ROLE, "instance-b")
            .await?;
        assert!(new > old);

        let err = indexer
            .index_events_fenced(vec![ev("y", 2, "t2", 0, 0)], 2, old)
            .await
            .expect_err("stale leader must be fenced");
        assert!(err.downcast_ref::<StaleLeaderError>().is_some());
        // Nothing from the stale batch landed, checkpoint untouched.
        assert_eq!(row_count(&db).await, 1);
        assert_eq!(backfill_checkpoint(&db).await, 1);

        indexer
            .index_events_fenced(vec![ev("y", 2, "t2", 0, 0)], 2, new)
            .await?;
        assert_eq!(row_count(&db).await, 2);
        Ok(())
    }

    #[tokio::test]
    async fn fenced_write_without_any_lease_is_rejected() -> Result<()> {
        let db = setup_contract_event_db().await?;
        let indexer = EventIndexer::new(db);
        let err = indexer
            .index_events_fenced(vec![], 1, FencingToken(7))
            .await
            .expect_err("no lease row");
        assert!(err.downcast_ref::<StaleLeaderError>().is_some());
        Ok(())
    }
}
