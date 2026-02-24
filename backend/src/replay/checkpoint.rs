//! Checkpoint Management
//!
//! Provides checkpoint functionality for saving and resuming replay progress.
//! Checkpoints enable recovery from failures and allow pausing/resuming replays.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use tracing::{debug, info};

/// Represents a checkpoint in the replay process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Checkpoint {
    /// Unique checkpoint identifier
    pub id: String,
    /// Replay session ID
    pub session_id: String,
    /// Last processed ledger
    pub last_ledger: u64,
    /// Total events processed
    pub events_processed: u64,
    /// Events failed
    pub events_failed: u64,
    /// State snapshot (JSON-encoded)
    pub state_snapshot: serde_json::Value,
    /// Checkpoint metadata
    pub metadata: HashMap<String, String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(session_id: String, last_ledger: u64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            last_ledger,
            events_processed: 0,
            events_failed: 0,
            state_snapshot: serde_json::json!({}),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Add metadata to checkpoint
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set state snapshot
    pub fn with_state(mut self, state: serde_json::Value) -> Self {
        self.state_snapshot = state;
        self
    }

    /// Set processing statistics
    pub fn with_stats(mut self, processed: u64, failed: u64) -> Self {
        self.events_processed = processed;
        self.events_failed = failed;
        self
    }
}

/// Manages checkpoint storage and retrieval
pub struct CheckpointManager {
    pool: SqlitePool,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Save a checkpoint
    pub async fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        info!(
            "Saving checkpoint {} for session {} at ledger {}",
            checkpoint.id, checkpoint.session_id, checkpoint.last_ledger
        );

        let metadata_json = serde_json::to_string(&checkpoint.metadata)?;
        let state_json = serde_json::to_string(&checkpoint.state_snapshot)?;

        sqlx::query(
            r#"
            INSERT INTO replay_checkpoints (
                id, session_id, last_ledger, events_processed, events_failed,
                state_snapshot, metadata, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                last_ledger = EXCLUDED.last_ledger,
                events_processed = EXCLUDED.events_processed,
                events_failed = EXCLUDED.events_failed,
                state_snapshot = EXCLUDED.state_snapshot,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&checkpoint.id)
        .bind(&checkpoint.session_id)
        .bind(checkpoint.last_ledger as i64)
        .bind(checkpoint.events_processed as i64)
        .bind(checkpoint.events_failed as i64)
        .bind(&state_json)
        .bind(&metadata_json)
        .bind(checkpoint.created_at)
        .execute(&self.pool)
        .await
        .context("Failed to save checkpoint")?;

        debug!("Checkpoint {} saved successfully", checkpoint.id);
        Ok(())
    }

    /// Load a checkpoint by ID
    pub async fn load(&self, checkpoint_id: &str) -> Result<Option<Checkpoint>> {
        debug!("Loading checkpoint {}", checkpoint_id);

        let row: Option<(String, String, i64, i64, i64, String, String, DateTime<Utc>)> =
            sqlx::query_as(
                r#"
            SELECT id, session_id, last_ledger, events_processed, events_failed,
                   state_snapshot, metadata, created_at
            FROM replay_checkpoints
            WHERE id = $1
            "#,
            )
            .bind(checkpoint_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to load checkpoint")?;

        match row {
            Some((
                id,
                session_id,
                last_ledger,
                events_processed,
                events_failed,
                state_json,
                metadata_json,
                created_at,
            )) => {
                let state_snapshot: serde_json::Value = serde_json::from_str(&state_json)?;
                let metadata: HashMap<String, String> = serde_json::from_str(&metadata_json)?;

                Ok(Some(Checkpoint {
                    id,
                    session_id,
                    last_ledger: last_ledger as u64,
                    events_processed: events_processed as u64,
                    events_failed: events_failed as u64,
                    state_snapshot,
                    metadata,
                    created_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get the latest checkpoint for a session
    pub async fn get_latest(&self, session_id: &str) -> Result<Option<Checkpoint>> {
        debug!("Getting latest checkpoint for session {}", session_id);

        let row: Option<(String, String, i64, i64, i64, String, String, DateTime<Utc>)> =
            sqlx::query_as(
                r#"
            SELECT id, session_id, last_ledger, events_processed, events_failed,
                   state_snapshot, metadata, created_at
            FROM replay_checkpoints
            WHERE session_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            )
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to get latest checkpoint")?;

        match row {
            Some((
                id,
                session_id,
                last_ledger,
                events_processed,
                events_failed,
                state_json,
                metadata_json,
                created_at,
            )) => {
                let state_snapshot: serde_json::Value = serde_json::from_str(&state_json)?;
                let metadata: HashMap<String, String> = serde_json::from_str(&metadata_json)?;

                Ok(Some(Checkpoint {
                    id,
                    session_id,
                    last_ledger: last_ledger as u64,
                    events_processed: events_processed as u64,
                    events_failed: events_failed as u64,
                    state_snapshot,
                    metadata,
                    created_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// List all checkpoints for a session
    pub async fn list_for_session(&self, session_id: &str) -> Result<Vec<Checkpoint>> {
        debug!("Listing checkpoints for session {}", session_id);

        let rows: Vec<(String, String, i64, i64, i64, String, String, DateTime<Utc>)> =
            sqlx::query_as(
                r#"
            SELECT id, session_id, last_ledger, events_processed, events_failed,
                   state_snapshot, metadata, created_at
            FROM replay_checkpoints
            WHERE session_id = $1
            ORDER BY created_at DESC
            "#,
            )
            .bind(session_id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list checkpoints")?;

        let checkpoints = rows
            .into_iter()
            .filter_map(
                |(
                    id,
                    session_id,
                    last_ledger,
                    events_processed,
                    events_failed,
                    state_json,
                    metadata_json,
                    created_at,
                )| {
                    let state_snapshot = serde_json::from_str(&state_json).ok()?;
                    let metadata = serde_json::from_str(&metadata_json).ok()?;

                    Some(Checkpoint {
                        id,
                        session_id,
                        last_ledger: last_ledger as u64,
                        events_processed: events_processed as u64,
                        events_failed: events_failed as u64,
                        state_snapshot,
                        metadata,
                        created_at,
                    })
                },
            )
            .collect();

        Ok(checkpoints)
    }

    /// Delete a checkpoint
    pub async fn delete(&self, checkpoint_id: &str) -> Result<()> {
        info!("Deleting checkpoint {}", checkpoint_id);

        sqlx::query("DELETE FROM replay_checkpoints WHERE id = $1")
            .bind(checkpoint_id)
            .execute(&self.pool)
            .await
            .context("Failed to delete checkpoint")?;

        Ok(())
    }

    /// Delete all checkpoints for a session
    pub async fn delete_for_session(&self, session_id: &str) -> Result<()> {
        info!("Deleting all checkpoints for session {}", session_id);

        sqlx::query("DELETE FROM replay_checkpoints WHERE session_id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .context("Failed to delete checkpoints")?;

        Ok(())
    }

    /// Clean up old checkpoints (older than specified days)
    pub async fn cleanup_old(&self, days: i64) -> Result<u64> {
        info!("Cleaning up checkpoints older than {} days", days);

        let cutoff = Utc::now() - chrono::Duration::days(days);

        let result = sqlx::query("DELETE FROM replay_checkpoints WHERE created_at < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .context("Failed to cleanup old checkpoints")?;

        let deleted = result.rows_affected();
        info!("Deleted {} old checkpoints", deleted);

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_creation() {
        let checkpoint = Checkpoint::new("session-1".to_string(), 1000)
            .with_metadata("key".to_string(), "value".to_string())
            .with_stats(100, 5);

        assert_eq!(checkpoint.session_id, "session-1");
        assert_eq!(checkpoint.last_ledger, 1000);
        assert_eq!(checkpoint.events_processed, 100);
        assert_eq!(checkpoint.events_failed, 5);
        assert_eq!(checkpoint.metadata.get("key"), Some(&"value".to_string()));
    }
}
