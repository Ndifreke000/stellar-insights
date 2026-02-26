//! Event Indexer Service
//!
//! This service indexes contract events, provides query interfaces,
//! and manages the event database for analytics and verification.

use crate::database::Database;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

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
    pub contract_id: Option<String>,
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

/// Event ordering options
#[derive(Debug, Clone)]
pub enum EventOrderBy {
    CreatedAtAsc,
    CreatedAtDesc,
    LedgerAsc,
    LedgerDesc,
    EpochAsc,
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

    /// Index a contract event
    pub async fn index_event(&self, event: IndexedEvent) -> Result<()> {
        debug!("Indexing event: {}", event.id);

        let query = r#"
            INSERT OR REPLACE INTO contract_events (
                id, contract_id, event_type, epoch, hash, timestamp, 
                ledger, transaction_hash, created_at, verification_status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

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

    /// Query events with filters
    pub async fn query_events(&self, query: EventQuery) -> Result<Vec<IndexedEvent>> {
        debug!("Querying events with filters: {:?}", query);

        let mut sql = String::from(
            r#"
            SELECT id, contract_id, event_type, epoch, hash, timestamp, 
                   ledger, transaction_hash, created_at, verification_status
            FROM contract_events
            WHERE 1=1
        "#,
        );

        let mut bindings = Vec::new();

        // Add filters
        if let Some(contract_id) = &query.contract_id {
            sql.push_str(" AND contract_id = ?");
            bindings.push(contract_id.clone());
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
        match query.order_by.as_ref().unwrap_or(&EventOrderBy::CreatedAtDesc) {
            EventOrderBy::CreatedAtAsc => sql.push_str(" ORDER BY created_at ASC"),
            EventOrderBy::CreatedAtDesc => sql.push_str(" ORDER BY created_at DESC"),
            EventOrderBy::LedgerAsc => sql.push_str(" ORDER BY ledger ASC"),
            EventOrderBy::LedgerDesc => sql.push_str(" ORDER BY ledger DESC"),
            EventOrderBy::EpochAsc => sql.push_str(" ORDER BY epoch ASC"),
            EventOrderBy::EpochDesc => sql.push_str(" ORDER BY epoch DESC"),
        }

        // Add pagination
        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
            if let Some(offset) = query.offset {
                sql.push_str(&format!(" OFFSET {}", offset));
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

        let query = r#"
            SELECT id, contract_id, event_type, epoch, hash, timestamp, 
                   ledger, transaction_hash, created_at, verification_status
            FROM contract_events
            WHERE id = ?
        "#;

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
    pub async fn update_verification_status(
        &self,
        event_id: &str,
        status: &str,
    ) -> Result<()> {
        debug!("Updating verification status for event {}: {}", event_id, status);

        let query = r#"
            UPDATE contract_events 
            SET verification_status = ?, verified_at = ?
            WHERE id = ?
        "#;

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

    /// Get event statistics
    pub async fn get_event_stats(&self) -> Result<EventStats> {
        debug!("Getting event statistics");

        let query = r#"
            SELECT 
                COUNT(*) as total_events,
                COUNT(CASE WHEN verification_status = 'verified' THEN 1 END) as verified_snapshots,
                COUNT(CASE WHEN verification_status = 'failed' THEN 1 END) as failed_verifications,
                MAX(epoch) as latest_epoch,
                MAX(ledger) as latest_ledger,
                COUNT(CASE WHEN created_at > datetime('now', '-1 day') THEN 1 END) as events_last_24h
            FROM contract_events
        "#;

        let row = sqlx::query(query)
            .fetch_one(self.db.pool())
            .await
            .context("Failed to get event statistics")?;

        let stats = EventStats {
            total_events: row.get("total_events"),
            verified_snapshots: row.get("verified_snapshots"),
            failed_verifications: row.get("failed_verifications"),
            latest_epoch: row.get::<Option<i64>, _>("latest_epoch").map(|e| e as u64),
            latest_ledger: row.get::<Option<i64>, _>("latest_ledger").map(|l| l as u64),
            events_last_24h: row.get("events_last_24h"),
        };

        debug!("Event stats: {:?}", stats);
        Ok(stats)
    }

    /// Get verification status summary for recent epochs
    pub async fn get_verification_summary(&self, epoch_count: i64) -> Result<Vec<VerificationSummary>> {
        debug!("Getting verification summary for last {} epochs", epoch_count);

        let query = r#"
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
        "#;

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
                verification_status: row.get("verification_status").unwrap_or("pending"),
                created_at: row.get("created_at"),
                transaction_hash: row.get("transaction_hash"),
            };
            summaries.push(summary);
        }

        Ok(summaries)
    }

    /// Search events by hash prefix
    pub async fn search_by_hash_prefix(&self, prefix: &str, limit: i64) -> Result<Vec<IndexedEvent>> {
        debug!("Searching events by hash prefix: {}", prefix);

        let query = r#"
            SELECT id, contract_id, event_type, epoch, hash, timestamp, 
                   ledger, transaction_hash, created_at, verification_status
            FROM contract_events
            WHERE hash LIKE ?
            ORDER BY created_at DESC
            LIMIT ?
        "#;

        let search_pattern = format!("{}%", prefix);

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

        debug!("Found {} events matching hash prefix {}", events.len(), prefix);
        Ok(events)
    }

    /// Clean up old events (retention policy)
    pub async fn cleanup_old_events(&self, days_to_keep: i64) -> Result<i64> {
        info!("Cleaning up events older than {} days", days_to_keep);

        let query = r#"
            DELETE FROM contract_events
            WHERE created_at < datetime('now', '-{} days')
        "#;

        let query = query.replace("{}", &days_to_keep.to_string());

        let result = sqlx::query(query)
            .execute(self.db.pool())
            .await
            .context("Failed to cleanup old events")?;

        let deleted_count = result.rows_affected();
        info!("Cleaned up {} old events", deleted_count);

        Ok(deleted_count as i64)
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
    use std::sync::Arc;

    #[tokio::test]
    async fn test_event_indexing() {
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let indexer = EventIndexer::new(db);

        let event = IndexedEvent {
            id: "test-event-1".to_string(),
            contract_id: "test-contract".to_string(),
            event_type: "SNAP_SUB".to_string(),
            epoch: Some(42),
            hash: Some("abcd1234".to_string()),
            timestamp: Some(1234567890),
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
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let indexer = EventIndexer::new(db);

        let event = IndexedEvent {
            id: "test-event-2".to_string(),
            contract_id: "test-contract".to_string(),
            event_type: "SNAP_SUB".to_string(),
            epoch: Some(43),
            hash: Some("efgh5678".to_string()),
            timestamp: Some(1234567891),
            ledger: 12346,
            transaction_hash: "tx-hash-2".to_string(),
            created_at: Utc::now(),
            verification_status: None,
        };

        indexer.index_event(event).await.unwrap();

        // Update verification status
        indexer.update_verification_status("test-event-2", "verified").await.unwrap();

        // Verify update
        let retrieved = indexer.get_event_by_id("test-event-2").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().verification_status, Some("verified".to_string()));
    }
}
