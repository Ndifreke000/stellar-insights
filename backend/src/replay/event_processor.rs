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
use crate::replay::{LedgerProtocolVersion, PROTOCOL_V20};

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
pub struct CompositeEventProcessor {
    processors: Vec<Arc<dyn EventProcessor>>,
}

impl CompositeEventProcessor {
    /// Create a new composite processor
    #[must_use]
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    /// Add a processor
    pub fn add_processor(mut self, processor: Arc<dyn EventProcessor>) -> Self {
        self.processors.push(processor);
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
                    "Retrying event {} (attempt {}/{})",
                    event.unique_id(),
                    attempts,
                    max_retries
                );
                // Exponential backoff
                tokio::time::sleep(Duration::from_millis(100 * 2_u64.pow(attempts))).await;
            }

            match timeout(context.timeout, self.process_event_internal(event, context)).await {
                Ok(Ok(result)) => {
                    if result.success {
                        return Ok(result);
                    }
                    last_error = result.error.clone();
                }
                Ok(Err(e)) => {
                    last_error = Some(e.to_string());
                }
                Err(_) => {
                    last_error = Some("Processing timeout".to_string());
                }
            }

            attempts += 1;
        }

        Ok(ProcessingResult::failure(
            last_error.unwrap_or_else(|| "Unknown error".to_string()),
        ))
    }

    /// Internal processing logic
    async fn process_event_internal(
        &self,
        event: &ContractEvent,
        context: &ProcessingContext,
    ) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();

        // Find appropriate processor
        for processor in &self.processors {
            // Check idempotency
            if processor.is_processed(event).await? {
                debug!("Event {} already processed, skipping", event.unique_id());
                return Ok(ProcessingResult::skipped());
            }

            // Validate event
            processor.validate_event(event)?;

            // Process event
            match processor.process_event(event, context).await {
                Ok(mut result) => {
                    result.duration_ms = start.elapsed().as_millis() as u64;

                    // Mark as processed if not dry-run
                    if !context.dry_run && result.success {
                        processor.mark_processed(event).await?;
                    }

                    info!(
                        "Event {} processed by {} in {}ms (success: {}, skipped: {})",
                        event.unique_id(),
                        processor.name(),
                        result.duration_ms,
                        result.success,
                        result.skipped
                    );

                    return Ok(result);
                }
                Err(e) => {
                    warn!(
                        "Processor {} failed to process event {}: {}",
                        processor.name(),
                        event.unique_id(),
                        e
                    );
                }
            }
        }

        // No processor handled the event
        warn!("No processor found for event {}", event.unique_id());
        Ok(ProcessingResult::failure(
            "No processor found for event".to_string(),
        ))
    }
}

impl Default for CompositeEventProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Example processor for snapshot submission events
pub struct SnapshotEventProcessor {
    pool: sqlx::SqlitePool,
}

impl SnapshotEventProcessor {
    #[must_use]
    pub const fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    async fn process_snapshot_submission(
        &self,
        event: &ContractEvent,
        context: &ProcessingContext,
    ) -> Result<ProcessingResult> {
        let proto = LedgerProtocolVersion(context.protocol_version);

        // Ledgers closed before Soroban was introduced (protocol < 20) cannot
        // contain snapshot-submission contract events.  Guard here in case the
        // caller supplies an explicit protocol version that is too old.
        if !proto.supports_soroban() {
            debug!(
                ledger = event.ledger_sequence,
                protocol = context.protocol_version,
                "Skipping snapshot submission — Soroban not active for this protocol version"
            );
            return Ok(ProcessingResult::skipped());
        }

        // Extract snapshot data from event
        let epoch = event
            .data
            .get("epoch")
            .and_then(serde_json::Value::as_u64)
            .context("Missing epoch in event data")?;

        let hash = event
            .data
            .get("hash")
            .and_then(|v| v.as_str())
            .context("Missing hash in event data")?;

        debug!(
            "Processing snapshot submission: epoch={}, hash={}, protocol={}",
            epoch, hash, context.protocol_version
        );

        // Check if already exists (idempotency)
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM snapshots WHERE epoch = $1 AND hash = $2)",
        )
        .bind(epoch as i64)
        .bind(hash)
        .fetch_one(&self.pool)
        .await?;

        if exists {
            return Ok(ProcessingResult::skipped());
        }

        // Insert snapshot record (if not dry-run).
        //
        // Protocol 21+ introduced stricter fee semantics (CAP-0052/CAP-0054).
        // The snapshot record itself has the same shape, but we tag it with the
        // active protocol so downstream analytics can distinguish pre-/post-
        // upgrade submissions without re-querying the ledger.
        if !context.dry_run {
            if proto.has_v21_fee_semantics() {
                // Protocol 21+: store with explicit protocol tag so the reader
                // can apply the correct fee-accounting rules.
                sqlx::query(
                    r"
                    INSERT INTO snapshots (
                        epoch, hash, ledger_sequence, transaction_hash,
                        protocol_version, created_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (epoch) DO NOTHING
                    ",
                )
                .bind(epoch as i64)
                .bind(hash)
                .bind(event.ledger_sequence as i64)
                .bind(&event.transaction_hash)
                .bind(context.protocol_version as i64)
                .bind(event.timestamp)
                .execute(&self.pool)
                .await?;
            } else {
                // Protocol 20 (original Soroban): legacy insert without the
                // protocol_version column so old schema deployments still work.
                sqlx::query(
                    r"
                    INSERT INTO snapshots (epoch, hash, ledger_sequence, transaction_hash, created_at)
                    VALUES ($1, $2, $3, $4, $5)
                    ON CONFLICT (epoch) DO NOTHING
                    ",
                )
                .bind(epoch as i64)
                .bind(hash)
                .bind(event.ledger_sequence as i64)
                .bind(&event.transaction_hash)
                .bind(event.timestamp)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(ProcessingResult::success().with_change(StateChange {
            change_type: "insert".to_string(),
            entity_type: "snapshot".to_string(),
            entity_id: epoch.to_string(),
            previous_value: None,
            new_value: Some(serde_json::json!({
                "epoch": epoch,
                "hash": hash,
                "ledger": event.ledger_sequence,
                "protocol_version": context.protocol_version,
            })),
        }))
    }
}

#[async_trait]
impl EventProcessor for SnapshotEventProcessor {
    async fn process_event(
        &self,
        event: &ContractEvent,
        context: &ProcessingContext,
    ) -> Result<ProcessingResult> {
        match event.event_type.as_str() {
            "snapshot_submitted" => self.process_snapshot_submission(event, context).await,
            _ => Ok(ProcessingResult::failure(format!(
                "Unknown event type: {}",
                event.event_type
            ))),
        }
    }

    async fn is_processed(&self, event: &ContractEvent) -> Result<bool> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM processed_events WHERE event_id = $1)")
                .bind(event.unique_id())
                .fetch_one(&self.pool)
                .await?;

        Ok(exists)
    }

    async fn mark_processed(&self, event: &ContractEvent) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO processed_events (event_id, ledger_sequence, processed_at)
            VALUES ($1, $2, CURRENT_TIMESTAMP)
            ON CONFLICT (event_id) DO NOTHING
            ",
        )
        .bind(event.unique_id())
        .bind(event.ledger_sequence as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        "SnapshotEventProcessor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_context() {
        let ctx = ProcessingContext::new();
        assert!(!ctx.is_replay());
        assert_eq!(ctx.protocol_version, 0);

        let replay_ctx = ProcessingContext::for_replay("session-1".to_string(), false);
        assert!(replay_ctx.is_replay());
        assert_eq!(replay_ctx.protocol_version, 0);

        let versioned = replay_ctx.with_protocol_version(21);
        assert_eq!(versioned.protocol_version, 21);
    }

    #[test]
    fn test_processing_result() {
        let success = ProcessingResult::success();
        assert!(success.success);
        assert!(!success.skipped);

        let skipped = ProcessingResult::skipped();
        assert!(skipped.success);
        assert!(skipped.skipped);

        let failed = ProcessingResult::failure("error".to_string());
        assert!(!failed.success);
        assert_eq!(failed.error, Some("error".to_string()));
    }
}
