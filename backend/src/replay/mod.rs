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

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Protocol version thresholds
// ---------------------------------------------------------------------------
// These constants represent the Stellar protocol version at which specific
// on-chain semantic changes were introduced.  Replay logic branches on these
// values so that events from older ledgers are processed with the rules that
// were active at the time, preserving determinism.
//
// Convention: `PROTOCOL_V<N>` is the first protocol version where the change
// takes effect.  All versions *below* the constant use the legacy path; all
// versions *at or above* use the new path.

/// Protocol 20 introduced Soroban smart contracts (Soroban / CAP-0046).
/// Before this version there are no contract events to replay; the replay
/// engine skips ledgers whose active protocol is below this threshold.
pub const PROTOCOL_V20: u32 = 20;

/// Protocol 21 tightened Soroban fee semantics (CAP-0052 / CAP-0054).
/// The snapshot-submission handler uses different fee/reserve accounting
/// depending on whether the ledger was closed under protocol 20 or 21+.
pub const PROTOCOL_V21: u32 = 21;

/// A resolved protocol version for a specific ledger.
///
/// Wraps the raw `u32` so callers can call the helper predicates instead of
/// writing bare numeric comparisons throughout the replay code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LedgerProtocolVersion(pub u32);

impl LedgerProtocolVersion {
    /// Return the raw protocol version number.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns `true` when Soroban contract events are valid for this ledger.
    /// Ledgers closed under a protocol version below 20 pre-date Soroban and
    /// produce no contract events; replaying them is a no-op.
    #[must_use]
    pub const fn supports_soroban(self) -> bool {
        self.0 >= PROTOCOL_V20
    }

    /// Returns `true` when the protocol-21 fee semantics are active.
    /// Used by `SnapshotEventProcessor` and `StateBuilder` to choose the
    /// correct accounting path for snapshot-submission events.
    #[must_use]
    pub const fn has_v21_fee_semantics(self) -> bool {
        self.0 >= PROTOCOL_V21
    }
}

impl Default for LedgerProtocolVersion {
    /// Default to protocol 20 (first Soroban-capable version) so that replay
    /// sessions that do not supply an explicit version still process events
    /// correctly.  Callers that know the exact version should always supply it.
    fn default() -> Self {
        Self(PROTOCOL_V20)
    }
}

impl fmt::Display for LedgerProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "protocol/{}", self.0)
    }
}

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
    #[must_use]
    pub fn unique_id(&self) -> String {
        format!(
            "{}:{}:{}",
            self.ledger_sequence, self.transaction_hash, self.event_type
        )
    }

    /// Check if event matches a filter
    #[must_use]
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
                "In Progress (ledger: {current_ledger}, processed: {events_processed}, failed: {events_failed})"
            ),
            Self::Completed {
                events_processed,
                events_failed,
                duration_secs,
            } => write!(
                f,
                "Completed (processed: {events_processed}, failed: {events_failed}, duration: {duration_secs}s)"
            ),
            Self::Failed { error, last_ledger } => {
                write!(f, "Failed: {error} (last ledger: {last_ledger:?})")
            }
            Self::Paused {
                last_ledger,
                events_processed,
            } => write!(
                f,
                "Paused (last ledger: {last_ledger}, processed: {events_processed})"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ledger_protocol_version_soroban_support() {
        // Versions below 20 pre-date Soroban — no contract events.
        assert!(!LedgerProtocolVersion(0).supports_soroban());
        assert!(!LedgerProtocolVersion(19).supports_soroban());
        // Protocol 20 introduced Soroban.
        assert!(LedgerProtocolVersion(20).supports_soroban());
        assert!(LedgerProtocolVersion(21).supports_soroban());
        assert!(LedgerProtocolVersion(22).supports_soroban());
    }

    #[test]
    fn test_ledger_protocol_version_v21_fee_semantics() {
        assert!(!LedgerProtocolVersion(20).has_v21_fee_semantics());
        assert!(LedgerProtocolVersion(21).has_v21_fee_semantics());
        assert!(LedgerProtocolVersion(22).has_v21_fee_semantics());
    }

    #[test]
    fn test_ledger_protocol_version_default() {
        // Default should be protocol 20 (first Soroban-capable version).
        let v = LedgerProtocolVersion::default();
        assert_eq!(v.as_u32(), PROTOCOL_V20);
        assert!(v.supports_soroban());
    }

    #[test]
    fn test_ledger_protocol_version_ordering() {
        assert!(LedgerProtocolVersion(19) < LedgerProtocolVersion(20));
        assert!(LedgerProtocolVersion(20) < LedgerProtocolVersion(21));
        assert_eq!(LedgerProtocolVersion(21), LedgerProtocolVersion(21));
    }

    #[test]
    fn test_ledger_protocol_version_display() {
        assert_eq!(LedgerProtocolVersion(20).to_string(), "protocol/20");
        assert_eq!(LedgerProtocolVersion(21).to_string(), "protocol/21");
    }
}
