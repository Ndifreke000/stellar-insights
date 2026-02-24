//! State Builder
//!
//! Rebuilds application state from contract events in a deterministic manner.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use tracing::{debug, info};

use super::{ContractEvent, ProcessingResult};

/// Represents the application state at a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationState {
    /// State version/ledger
    pub ledger: u64,
    /// Snapshot data
    pub snapshots: HashMap<u64, SnapshotState>,
    /// Verification data
    pub verifications: HashMap<String, VerificationState>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ApplicationState {
    /// Create a new empty state
    pub fn new() -> Self {
        Self {
            ledger: 0,
            snapshots: HashMap::new(),
            verifications: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create state at a specific ledger
    pub fn at_ledger(ledger: u64) -> Self {
        Self {
            ledger,
            snapshots: HashMap::new(),
            verifications: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Serialize state to JSON
    pub fn to_json(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }

    /// Deserialize state from JSON
    pub fn from_json(value: &serde_json::Value) -> Result<Self> {
        Ok(serde_json::from_value(value.clone())?)
    }

    /// Compute state hash for verification
    pub fn compute_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let json = self.to_json().unwrap_or_default();
        let json_str = serde_json::to_string(&json).unwrap_or_default();
        let hash = Sha256::digest(json_str.as_bytes());
        hex::encode(hash)
    }
}

impl Default for ApplicationState {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotState {
    pub epoch: u64,
    pub hash: String,
    pub ledger: u64,
    pub transaction_hash: String,
}

/// Verification state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationState {
    pub verifier: String,
    pub epoch: u64,
    pub verified_at: chrono::DateTime<chrono::Utc>,
}

/// Builds application state from events
pub struct StateBuilder {
    pool: SqlitePool,
    state: ApplicationState,
}

impl StateBuilder {
    /// Create a new state builder
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            state: ApplicationState::new(),
        }
    }

    /// Create state builder with initial state
    pub fn with_state(pool: SqlitePool, state: ApplicationState) -> Self {
        Self { pool, state }
    }

    /// Get current state
    pub fn state(&self) -> &ApplicationState {
        &self.state
    }

    /// Apply an event to the state
    pub async fn apply_event(&mut self, event: &ContractEvent) -> Result<ProcessingResult> {
        debug!(
            "Applying event {} to state at ledger {}",
            event.unique_id(),
            self.state.ledger
        );

        // Update ledger
        if event.ledger_sequence > self.state.ledger {
            self.state.ledger = event.ledger_sequence;
        }

        // Process based on event type
        match event.event_type.as_str() {
            "snapshot_submitted" => self.apply_snapshot_submission(event).await,
            "snapshot_verified" => self.apply_snapshot_verification(event).await,
            _ => {
                debug!("Unknown event type: {}", event.event_type);
                Ok(ProcessingResult::success())
            }
        }
    }

    /// Apply snapshot submission event
    async fn apply_snapshot_submission(
        &mut self,
        event: &ContractEvent,
    ) -> Result<ProcessingResult> {
        let epoch = event
            .data
            .get("epoch")
            .and_then(|v| v.as_u64())
            .context("Missing epoch")?;

        let hash = event
            .data
            .get("hash")
            .and_then(|v| v.as_str())
            .context("Missing hash")?
            .to_string();

        // Check if already exists (idempotency)
        if self.state.snapshots.contains_key(&epoch) {
            return Ok(ProcessingResult::skipped());
        }

        // Add to state
        self.state.snapshots.insert(
            epoch,
            SnapshotState {
                epoch,
                hash,
                ledger: event.ledger_sequence,
                transaction_hash: event.transaction_hash.clone(),
            },
        );

        info!("Applied snapshot submission for epoch {}", epoch);
        Ok(ProcessingResult::success())
    }

    /// Apply snapshot verification event
    async fn apply_snapshot_verification(
        &mut self,
        event: &ContractEvent,
    ) -> Result<ProcessingResult> {
        let epoch = event
            .data
            .get("epoch")
            .and_then(|v| v.as_u64())
            .context("Missing epoch")?;

        let verifier = event
            .data
            .get("verifier")
            .and_then(|v| v.as_str())
            .context("Missing verifier")?
            .to_string();

        let key = format!("{}:{}", epoch, verifier);

        // Check if already exists (idempotency)
        if self.state.verifications.contains_key(&key) {
            return Ok(ProcessingResult::skipped());
        }

        // Add to state
        self.state.verifications.insert(
            key,
            VerificationState {
                verifier: verifier.clone(),
                epoch,
                verified_at: event.timestamp,
            },
        );

        info!(
            "Applied snapshot verification for epoch {} by {}",
            epoch, verifier
        );
        Ok(ProcessingResult::success())
    }

    /// Persist current state to database
    pub async fn persist_state(&self) -> Result<()> {
        info!("Persisting state at ledger {}", self.state.ledger);

        let state_json = self.state.to_json()?;
        let state_hash = self.state.compute_hash();

        sqlx::query(
            r#"
            INSERT INTO replay_state (ledger, state_json, state_hash, updated_at)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
            ON CONFLICT (ledger) DO UPDATE SET
                state_json = EXCLUDED.state_json,
                state_hash = EXCLUDED.state_hash,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(self.state.ledger as i64)
        .bind(serde_json::to_string(&state_json)?)
        .bind(&state_hash)
        .execute(&self.pool)
        .await
        .context("Failed to persist state")?;

        Ok(())
    }

    /// Load state from database
    pub async fn load_state(&mut self, ledger: u64) -> Result<bool> {
        debug!("Loading state at ledger {}", ledger);

        let row: Option<(String, String)> =
            sqlx::query_as("SELECT state_json, state_hash FROM replay_state WHERE ledger = $1")
                .bind(ledger as i64)
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some((state_json, state_hash)) => {
                let state_value: serde_json::Value = serde_json::from_str(&state_json)?;
                self.state = ApplicationState::from_json(&state_value)?;

                // Verify hash
                let computed_hash = self.state.compute_hash();
                if computed_hash != state_hash {
                    return Err(anyhow::anyhow!(
                        "State hash mismatch: expected {}, got {}",
                        state_hash,
                        computed_hash
                    ));
                }

                info!("Loaded state at ledger {} (hash: {})", ledger, state_hash);
                Ok(true)
            }
            None => {
                debug!("No state found at ledger {}", ledger);
                Ok(false)
            }
        }
    }

    /// Compare current state with database state
    pub async fn verify_state(&self, ledger: u64) -> Result<bool> {
        debug!("Verifying state at ledger {}", ledger);

        let row: Option<(String,)> =
            sqlx::query_as("SELECT state_hash FROM replay_state WHERE ledger = $1")
                .bind(ledger as i64)
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some((expected_hash,)) => {
                let actual_hash = self.state.compute_hash();
                let matches = actual_hash == expected_hash;

                if matches {
                    info!("State verification passed at ledger {}", ledger);
                } else {
                    info!(
                        "State verification failed at ledger {}: expected {}, got {}",
                        ledger, expected_hash, actual_hash
                    );
                }

                Ok(matches)
            }
            None => {
                debug!("No state to verify at ledger {}", ledger);
                Ok(false)
            }
        }
    }

    /// Reset state to empty
    pub fn reset(&mut self) {
        self.state = ApplicationState::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_state() {
        let state = ApplicationState::new();
        assert_eq!(state.ledger, 0);
        assert!(state.snapshots.is_empty());

        let hash = state.compute_hash();
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_state_serialization() {
        let state = ApplicationState::at_ledger(1000);
        let json = state.to_json().unwrap();
        let restored = ApplicationState::from_json(&json).unwrap();
        assert_eq!(restored.ledger, 1000);
    }
}
