//! Event Processor
//!
//! Provides the core event processing logic that is shared between
//! live event handling and replay mode to ensure consistency.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use super::ContractEvent;
use crate::replay::LedgerProtocolVersion;

/// Version of the contract event schema this backend understands.
///
/// Mirrors `contracts/payraider/src/events.rs::EVENT_SCHEMA_VERSION`. Bump
/// both together whenever a topic/data shape changes so decoders can reject
/// or migrate events produced by an incompatible contract build.
pub const EVENT_SCHEMA_VERSION: u32 = 1;

/// Versioned schema descriptor for a single contract event kind.
///
/// The contract crate is the source of truth for these shapes; this registry
/// is the backend's mirror so decoders no longer hard-code topic/data layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventSchema {
    /// Event kind name (matches the contract's event symbol).
    pub name: &'static str,
    /// Schema version this descriptor describes.
    pub version: u32,
    /// Ordered topic symbols (excluding the leading event-name topic).
    pub topics: &'static [&'static str],
    /// Ordered data field names.
    pub data_fields: &'static [&'static str],
}

/// Registry of known event schemas, keyed by event name.
///
/// Keep in sync with `contracts/payraider/src/events.rs`. Adding a field is a
/// backwards-compatible change (decoders tolerate unknown trailing fields);
/// removing or reordering fields requires a version bump.
pub const EVENT_SCHEMAS: &[EventSchema] = &[
    EventSchema {
        name: "init",
        version: 1,
        topics: &["admin"],
        data_fields: &["fee_bps"],
    },
    EventSchema {
        name: "payment",
        version: 1,
        topics: &["from", "to"],
        data_fields: &["amount", "asset", "memo"],
    },
    EventSchema {
        name: "refund",
        version: 1,
        topics: &["from", "to"],
        data_fields: &["amount", "asset"],
    },
];

/// Look up the schema for an event name at the current schema version.
#[must_use]
pub fn schema_for(name: &str) -> Option<&'static EventSchema> {
    EVENT_SCHEMAS
        .iter()
        .find(|s| s.name == name && s.version == EVENT_SCHEMA_VERSION)
}

/// Error returned when an event cannot be decoded against the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// No schema is registered for the event name.
    UnknownEvent(String),
    /// The event was produced by a newer schema version than we support.
    UnsupportedVersion { name: String, version: u32 },
    /// A required topic or data field is missing.
    MissingField { name: String, field: String },
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownEvent(name) => write!(f, "unknown event schema: {name}"),
            Self::UnsupportedVersion { name, version } => write!(
                f,
                "event {name} uses unsupported schema version {version} (backend supports {EVENT_SCHEMA_VERSION})"
            ),
            Self::MissingField { name, field } => {
                write!(f, "event {name} missing required field {field}")
            }
        }
    }
}

impl std::error::Error for SchemaError {}

/// Decode an event's topic/data payload against the versioned registry.
///
/// Returns the ordered topic values and a map of data fields. Unknown trailing
/// fields are tolerated (forward-compatible additions); missing required
/// fields are rejected so silent decoder breakage becomes a loud error.
pub fn decode_event(
    event: &ContractEvent,
) -> std::result::Result<(Vec<String>, serde_json::Map<String, serde_json::Value>), SchemaError> {
    let schema = schema_for(&event.event_type)
        .ok_or_else(|| SchemaError::UnknownEvent(event.event_type.clone()))?;

    let topics = event.topics.clone();
    if topics.len() < schema.topics.len() {
        let field = schema.topics[topics.len()].to_string();
        return Err(SchemaError::MissingField {
            name: event.event_type.clone(),
            field,
        });
    }

    let data = match &event.data {
        serde_json::Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };

    for field in schema.data_fields {
        if !data.contains_key(*field) {
            return Err(SchemaError::MissingField {
                name: event.event_type.clone(),
                field: (*field).to_string(),
            });
        }
    }

    Ok((topics, data))
}

/// Context provided to event processors
#[derive(Debug, Clone)]
pub struct ProcessingContext {
    /// Replay session ID (None for live processing)
    pub session_id: Option<String>,
    /// Whether this is a dry-run
    pub dry_run: bool,
    /// Current ledger being processed
    pub current_ledger: u64,
    /// Total events processed in this session
    pub events_processed: u64,
    /// Processing timeout
    pub timeout: Duration,
    /// Stellar protocol version active for the current ledger.
    ///
    /// Set to the protocol version that was active when `current_ledger` was
    /// closed so that processors can branch on semantics that changed across
    /// protocol upgrades.  `0` means the version has not been resolved yet
    /// (treated as "use legacy / most-conservative path").
    pub protocol_version: u32,
}

impl ProcessingContext {
    /// Create a new processing context
    #[must_use]
    pub const fn new() -> Self {
        Self {
            session_id: None,
            dry_run: false,
            current_ledger: 0,
            events_processed: 0,
            timeout: Duration::from_secs(30),
            protocol_version: 0,
        }
    }

    /// Create context for replay
    #[must_use]
    pub const fn for_replay(session_id: String, dry_run: bool) -> Self {
        Self {
            session_id: Some(session_id),
            dry_run,
            current_ledger: 0,
            events_processed: 0,
            timeout: Duration::from_secs(30),
            protocol_version: 0,
        }
    }

    /// Return a copy of this context with the given protocol version applied.
    ///
    /// Called by the engine once per ledger batch so that each processor
    /// receives the version that was active when the ledger was closed.
    #[must_use]
    pub const fn with_protocol_version(mut self, version: u32) -> Self {
        self.protocol_version = version;
        self
    }

    /// Check if this is a replay context
    #[must_use]
    pub const fn is_replay(&self) -> bool {
        self.session_id.is_some()
    }
}

impl Default for ProcessingContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of processing an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    /// Whether processing was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// State changes made (for verification)
    pub state_changes: Vec<StateChange>,
    /// Processing duration in milliseconds
    pub duration_ms: u64,
    /// Whether the event was skipped (idempotency)
    pub skipped: bool,
}

impl ProcessingResult {
    /// Create a successful result
    #[must_use]
    pub const fn success() -> Self {
        Self {
            success: true,
            error: None,
            state_changes: Vec::new(),
            duration_ms: 0,
            skipped: false,
        }
    }

    /// Create a failed result
    #[must_use]
    pub const fn failure(error: String) -> Self {
        Self {
            success: false,
            error: Some(error),
            state_changes: Vec::new(),
            duration_ms: 0,
            skipped: false,
        }
    }

    /// Create a skipped result (idempotency)
    #[must_use]
    pub const fn skipped() -> Self {
        Self {
            success: true,
            error: None,
            state_changes: Vec::new(),
            duration_ms: 0,
            skipped: true,
        }
    }

    /// Add a state change
    #[must_use]
    pub fn with_change(mut self, change: StateChange) -> Self {
        self.state_changes.push(change);
        self
    }

    /// Set duration
    #[must_use]
    pub const fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Represents a state change made by processing an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    /// Type of change (insert, update, delete)
    pub change_type: String,
    /// Entity type affected
    pub entity_type: String,
    /// Entity ID
    pub entity_id: String,
    /// Previous value (for verification)
    pub previous_value: Option<serde_json::Value>,
    /// New value
    pub new_value: Option<serde_json::Value>,
}

/// Trait for processing contract events
#[async_trait]
pub trait EventProcessor: Send + Sync {
    /// Process a single event
    async fn process_event(
        &self,
        event: &ContractEvent,
        context: &ProcessingContext,
    ) -> Result<ProcessingResult>;

    /// Check if an event has already been processed (idempotency)
    async fn is_processed(&self, event: &ContractEvent) -> Result<bool>;

    /// Mark an event as processed
    async fn mark_processed(&self, event: &ContractEvent) -> Result<()>;

    /// Validate event data
    fn validate_event(&self, event: &ContractEvent) -> Result<()> {
        if event.contract_id.is_empty() {
            return Err(anyhow::anyhow!("Contract ID is empty"));
        }
        if event.event_type.is_empty() {
            return Err(anyhow::anyhow!("Event type is empty"));
        }
        Ok(())
    }

    /// Get processor name for logging
    fn name(&self) -> &str;
}

/// Composite processor that delegates to specific processors based on event type
///
/// Processors are registered for a protocol-version range (#2768) and only
/// those whose range contains `ProcessingContext::protocol_version` are
/// tried, so event decoding rules can change across upgrades (e.g. CAP-67
/// in protocol 23) without one handler branching on every version.
pub struct CompositeEventProcessor {
    processors: Vec<VersionedProcessor>,
}

/// Inclusive protocol range a processor handles; `max: None` is open-ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolRange {
    pub min: u32,
    pub max: Option<u32>,
}

impl ProtocolRange {
    pub const ALL: Self = Self { min: 0, max: None };

    #[must_use]
    pub fn contains(self, version: u32) -> bool {
        version >= self.min && self.max.is_none_or(|m| version <= m)
    }
}

struct VersionedProcessor {
    range: ProtocolRange,
    processor: Arc<dyn EventProcessor>,
}

impl CompositeEventProcessor {
    /// Create a new composite processor
    #[must_use]
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    /// Add a processor that handles every protocol version.
    pub fn add_processor(mut self, processor: Arc<dyn EventProcessor>) -> Self {
        self.processors.push(VersionedProcessor {
            range: ProtocolRange::ALL,
            processor,
        });
        self
    }

    /// Add a processor that only handles ledgers closed under a protocol
    /// version in `[min, max]` (`max: None` = no upper bound).
    pub fn add_versioned_processor(
        mut self,
        min: u32,
        max: Option<u32>,
        processor: Arc<dyn EventProcessor>,
    ) -> Self {
        self.processors.push(VersionedProcessor {
            range: ProtocolRange { min, max },
            processor,
        });
        self
    }

    /// Process an event with timeout and retry logic
    pub async fn process_with_retry(
        &self,
        event: &ContractEvent,
        context: &ProcessingContext,
        max_retries: u32,
    ) -> Result<ProcessingResult> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= max_retries {
            if attempts > 0 {
                debug!(
                    "Retrying event {}

/* … truncated 10507 chars — edit only what you need near the top … */
