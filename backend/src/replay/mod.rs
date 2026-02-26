//! Contract Event Replay System
//!
//! This module provides a reliable system for replaying contract events to rebuild
//! application state and support debugging. It ensures deterministic replay with
//! idempotency guarantees and comprehensive error handling.
//!
//! ## Features
//! - Deterministic event replay from historical data
//! - Idempotent processing (safe to replay multiple times)
//! - Checkpoint and resume capability
//! - Structured logging and tracing
//! - Network and contract filtering
//! - Shared processing logic with live event handling
//! - Performance optimized for large datasets

pub mod checkpoint;
pub mod config;
pub mod engine;
pub mod event_processor;
pub mod state_builder;
pub mod storage;

pub use checkpoint::{Checkpoint, CheckpointManager};
pub use config::{ReplayConfig, ReplayMode, ReplayRange};
pub use engine::ReplayEngine;
pub use event_processor::{EventProcessor, ProcessingContext, ProcessingResult};
pub use state_builder::StateBuilder;
pub use storage::{EventStorage, ReplayStorage};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a contract event from the blockchain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractEvent {
    /// Unique event identifier
    pub id: String,
    /// Ledger sequence number
    pub ledger_sequence: u64,
    /// Transaction hash
    pub transaction_hash: String,
    /// Contract ID that emitted the event
    pub contract_id: String,
    /// Event type/topic
    pub event_type: String,
    /// Event data (JSON-encoded)
    pub data: serde_json::Value,
    /// Timestamp when event occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Network identifier (testnet, mainnet, etc.)
    pub network: String,
}

impl ContractEvent {
    /// Create a unique identifier for this event
    pub fn unique_id(&self) -> String {
        format!(
            "{}:{}:{}",
            self.ledger_sequence, self.transaction_hash, self.event_type
        )
    }

    /// Check if event matches a filter
    pub fn matches_filter(&self, filter: &EventFilter) -> bool {
        if let Some(ref contract_ids) = filter.contract_ids {
            if !contract_ids.contains(&self.contract_id) {
                return false;
            }
        }

        if let Some(ref event_types) = filter.event_types {
            if !event_types.contains(&self.event_type) {
                return false;
            }
        }

        if let Some(ref network) = filter.network {
            if &self.network != network {
                return false;
            }
        }

        true
    }
}

/// Filter for selecting events to replay
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventFilter {
    /// Filter by contract IDs
    pub contract_ids: Option<Vec<String>>,
    /// Filter by event types
    pub event_types: Option<Vec<String>>,
    /// Filter by network
    pub network: Option<String>,
}

/// Status of a replay operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplayStatus {
    /// Replay is pending
    Pending,
    /// Replay is in progress
    InProgress {
        /// Current ledger being processed
        current_ledger: u64,
        /// Total events processed
        events_processed: u64,
        /// Events failed
        events_failed: u64,
    },
    /// Replay completed successfully
    Completed {
        /// Total events processed
        events_processed: u64,
        /// Events failed
        events_failed: u64,
        /// Duration in seconds
        duration_secs: u64,
    },
    /// Replay failed
    Failed {
        /// Error message
        error: String,
        /// Last successful ledger
        last_ledger: Option<u64>,
    },
    /// Replay paused (can be resumed)
    Paused {
        /// Last processed ledger
        last_ledger: u64,
        /// Events processed so far
        events_processed: u64,
    },
}

impl fmt::Display for ReplayStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::InProgress {
                current_ledger,
                events_processed,
                events_failed,
            } => write!(
                f,
                "In Progress (ledger: {}, processed: {}, failed: {})",
                current_ledger, events_processed, events_failed
            ),
            Self::Completed {
                events_processed,
                events_failed,
                duration_secs,
            } => write!(
                f,
                "Completed (processed: {}, failed: {}, duration: {}s)",
                events_processed, events_failed, duration_secs
            ),
            Self::Failed { error, last_ledger } => {
                write!(f, "Failed: {} (last ledger: {:?})", error, last_ledger)
            }
            Self::Paused {
                last_ledger,
                events_processed,
            } => write!(
                f,
                "Paused (last ledger: {}, processed: {})",
                last_ledger, events_processed
            ),
        }
    }
}

/// Metadata about a replay session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayMetadata {
    /// Unique replay session ID
    pub session_id: String,
    /// Replay configuration
    pub config: ReplayConfig,
    /// Current status
    pub status: ReplayStatus,
    /// Start time
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// End time (if completed or failed)
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Checkpoint information
    pub checkpoint: Option<Checkpoint>,
}

/// Error types specific to replay operations
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("Event not found: {0}")]
    EventNotFound(String),

    #[error("Invalid checkpoint: {0}")]
    InvalidCheckpoint(String),

    #[error("Replay already in progress: {0}")]
    AlreadyInProgress(String),

    #[error("Storage error: {0}")]
    StorageError(#[from] anyhow::Error),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("State corruption detected: {0}")]
    StateCorruption(String),
}

pub type ReplayResult<T> = std::result::Result<T, ReplayError>;
