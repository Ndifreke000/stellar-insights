//! Event Storage
//!
//! Provides storage and retrieval of contract events for replay.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{debug, info};

use super::{ContractEvent, EventFilter, ReplayMetadata, ReplayStatus};

/// Storage for contract events
pub struct EventStorage {
    pool: SqlitePool,
}

impl EventStorage {
    /// Create a new event storage
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Store a contract event
    pub async fn store_event(&self, event: &ContractEvent) -> Result<()> {
        let data_json = serde_json::to_string(&event.data)?;

        sqlx::query(
            r#"
            INSERT INTO contract_events (
                id, ledger_sequence, transaction_hash, contract_id,
                event_type, data, timestamp, network
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(&event.id)
        .bind(event.ledger_sequence as i64)
        .bind(&event.transaction_hash)
        .bind(&event.contract_id)
        .bind(&event.event_type)
        .bind(&data_json)
        .bind(event.timestamp)
        .bind(&event.network)
        .execute(&self.pool)
        .await
        .context("Failed to store event")?;

        Ok(())
    }

    /// Get events in a ledger range
    pub async fn get_events_in_range(
        &self,
        start_ledger: u64,
        end_ledger: u64,
        filter: &EventFilter,
        limit: Option<usize>,
    ) -> Result<Vec<ContractEvent>> {
        debug!(
            "Fetching events from ledger {} to {}",
            start_ledger, end_ledger
        );

        let mut query = String::from(
            r#"
            SELECT id, ledger_sequence, transaction_hash, contract_id,
                   event_type, data, timestamp, network
            FROM contract_events
            WHERE ledger_sequence >= $1 AND ledger_sequence <= $2
            "#,
        );

        // Apply filters
        let mut bind_index = 3;
        if filter.contract_ids.is_some() {
            query.push_str(&format!(" AND contract_id IN (${}) ", bind_index));
            bind_index += 1;
        }
        if filter.event_types.is_some() {
            query.push_str(&format!(" AND event_type IN (${}) ", bind_index));
            bind_index += 1;
        }
        if filter.network.is_some() {
            query.push_str(&format!(" AND network = ${} ", bind_index));
        }

        query.push_str(" ORDER BY ledger_sequence ASC, id ASC");

        if let Some(lim) = limit {
            query.push_str(&format!(" LIMIT {}", lim));
        }

        let mut query_builder = sqlx::query_as::<
            _,
            (
                String,
                i64,
                String,
                String,
                String,
                String,
                DateTime<Utc>,
                String,
            ),
        >(&query)
        .bind(start_ledger as i64)
        .bind(end_ledger as i64);

        // Bind filter parameters (simplified - in production, use proper parameter binding)
        let rows = query_builder.fetch_all(&self.pool).await?;

        let events = rows
            .into_iter()
            .filter_map(
                |(id, ledger, tx_hash, contract_id, event_type, data_json, timestamp, network)| {
                    let data = serde_json::from_str(&data_json).ok()?;
                    Some(ContractEvent {
                        id,
                        ledger_sequence: ledger as u64,
                        transaction_hash: tx_hash,
                        contract_id,
                        event_type,
                        data,
                        timestamp,
                        network,
                    })
                },
            )
            .collect();

        Ok(events)
    }

    /// Get total event count in range
    pub async fn count_events_in_range(
        &self,
        start_ledger: u64,
        end_ledger: u64,
        filter: &EventFilter,
    ) -> Result<u64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM contract_events
            WHERE ledger_sequence >= $1 AND ledger_sequence <= $2
            "#,
        )
        .bind(start_ledger as i64)
        .bind(end_ledger as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u64)
    }

    /// Get the latest ledger with events
    pub async fn get_latest_ledger(&self) -> Result<Option<u64>> {
        let ledger: Option<i64> =
            sqlx::query_scalar("SELECT MAX(ledger_sequence) FROM contract_events")
                .fetch_optional(&self.pool)
                .await?;

        Ok(ledger.map(|l| l as u64))
    }
}

/// Storage for replay metadata and state
pub struct ReplayStorage {
    pool: SqlitePool,
}

impl ReplayStorage {
    /// Create a new replay storage
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Save replay metadata
    pub async fn save_metadata(&self, metadata: &ReplayMetadata) -> Result<()> {
        info!("Saving replay metadata for session {}", metadata.session_id);

        let config_json = serde_json::to_string(&metadata.config)?;
        let status_json = serde_json::to_string(&metadata.status)?;
        let checkpoint_json = serde_json::to_string(&metadata.checkpoint)?;

        sqlx::query(
            r#"
            INSERT INTO replay_sessions (
                session_id, config, status, started_at, ended_at, checkpoint
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (session_id) DO UPDATE SET
                status = EXCLUDED.status,
                ended_at = EXCLUDED.ended_at,
                checkpoint = EXCLUDED.checkpoint
            "#,
        )
        .bind(&metadata.session_id)
        .bind(&config_json)
        .bind(&status_json)
        .bind(metadata.started_at)
        .bind(metadata.ended_at)
        .bind(&checkpoint_json)
        .execute(&self.pool)
        .await
        .context("Failed to save replay metadata")?;

        Ok(())
    }

    /// Load replay metadata
    pub async fn load_metadata(&self, session_id: &str) -> Result<Option<ReplayMetadata>> {
        let row: Option<(
            String,
            String,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            String,
        )> = sqlx::query_as(
            r#"
                SELECT session_id, config, status, started_at, ended_at, checkpoint
                FROM replay_sessions
                WHERE session_id = $1
                "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((session_id, config_json, status_json, started_at, ended_at, checkpoint_json)) => {
                let config = serde_json::from_str(&config_json)?;
                let status = serde_json::from_str(&status_json)?;
                let checkpoint = serde_json::from_str(&checkpoint_json)?;

                Ok(Some(ReplayMetadata {
                    session_id,
                    config,
                    status,
                    started_at,
                    ended_at,
                    checkpoint,
                }))
            }
            None => Ok(None),
        }
    }

    /// List all replay sessions
    pub async fn list_sessions(&self, limit: Option<usize>) -> Result<Vec<ReplayMetadata>> {
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();

        let query = format!(
            r#"
            SELECT session_id, config, status, started_at, ended_at, checkpoint
            FROM replay_sessions
            ORDER BY started_at DESC
            {}
            "#,
            limit_clause
        );

        let rows: Vec<(
            String,
            String,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            String,
        )> = sqlx::query_as(&query).fetch_all(&self.pool).await?;

        let sessions = rows
            .into_iter()
            .filter_map(
                |(session_id, config_json, status_json, started_at, ended_at, checkpoint_json)| {
                    let config = serde_json::from_str(&config_json).ok()?;
                    let status = serde_json::from_str(&status_json).ok()?;
                    let checkpoint = serde_json::from_str(&checkpoint_json).ok()?;

                    Some(ReplayMetadata {
                        session_id,
                        config,
                        status,
                        started_at,
                        ended_at,
                        checkpoint,
                    })
                },
            )
            .collect();

        Ok(sessions)
    }

    /// Delete replay session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        info!("Deleting replay session {}", session_id);

        sqlx::query("DELETE FROM replay_sessions WHERE session_id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_storage() {
        // This would require a test database setup
        // Placeholder for actual test implementation
    }
}
