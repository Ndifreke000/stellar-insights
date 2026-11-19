//! Replay Engine
//!
//! Orchestrates the replay process, managing event fetching, processing,
//! checkpointing, and state building.

use anyhow::{Context, Result};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use super::{
    checkpoint::{Checkpoint, CheckpointManager},
    config::{ReplayConfig, ReplayMode},
    event_processor::{CompositeEventProcessor, ProcessingContext},
    ledger_headers::ProtocolVersionSource,
    state_builder::StateBuilder,
    storage::{EventStorage, ReplayStorage},
    ContractEvent, LedgerProtocolVersion, ReplayError, ReplayMetadata, ReplayResult, ReplayStatus,
};
use std::collections::HashMap;

/// Main replay engine
pub struct ReplayEngine {
    config: ReplayConfig,
    event_storage: Arc<EventStorage>,
    replay_storage: Arc<ReplayStorage>,
    checkpoint_manager: Arc<CheckpointManager>,
    processor: Arc<CompositeEventProcessor>,
    state_builder: Arc<RwLock<StateBuilder>>,
    session_id: String,
    /// Per-ledger protocol versions recorded at ingestion (#2768).
    protocol_source: Option<Arc<dyn ProtocolVersionSource>>,
}

/// Resolve the protocol version active for `ledger` on `network`.
///
/// 1. A non-zero `override_version` (from `ReplayConfig`) wins.
/// 2. Otherwise the version is looked up in the ledger headers recorded at
///    ingestion.
/// 3. A missing header is an explicit [`ReplayError::MissingLedgerHeader`],
///    never a silent `0`.
pub async fn resolve_protocol_version(
    override_version: u32,
    source: Option<&dyn ProtocolVersionSource>,
    network: &str,
    ledger: u64,
) -> ReplayResult<u32> {
    if override_version != 0 {
        return Ok(override_version);
    }
    let missing = || ReplayError::MissingLedgerHeader {
        network: network.to_string(),
        ledger,
    };
    let source = source.ok_or_else(missing)?;
    source
        .protocol_version(network, ledger)
        .await?
        .ok_or_else(missing)
}

impl ReplayEngine {
    /// Create a new replay engine
    pub fn new(
        config: ReplayConfig,
        event_storage: Arc<EventStorage>,
        replay_storage: Arc<ReplayStorage>,
        checkpoint_manager: Arc<CheckpointManager>,
        processor: Arc<CompositeEventProcessor>,
        state_builder: Arc<RwLock<StateBuilder>>,
    ) -> Result<Self> {
        // Validate configuration
        config
            .validate()
            .map_err(|e| ReplayError::ConfigError(e.to_string()))?;

        let session_id = uuid::Uuid::new_v4().to_string();

        Ok(Self {
            config,
            event_storage,
            replay_storage,
            checkpoint_manager,
            processor,
            state_builder,
            session_id,
            protocol_source: None,
        })
    }

    /// Attach the per-ledger protocol version source (normally
    /// [`super::LedgerHeaderStore`]).
    #[must_use]
    pub fn with_protocol_source(mut self, source: Arc<dyn ProtocolVersionSource>) -> Self {
        self.protocol_source = Some(source);
        self
    }

    /// Start the replay process
    pub async fn start(&self) -> ReplayResult<ReplayMetadata> {
        info!(
            "Starting replay session {} with mode: {}",
            self.session_id, self.config.mode
        );

        // Create initial metadata
        let mut metadata = ReplayMetadata {
            session_id: self.session_id.clone(),
            config: self.config.clone(),
            status: ReplayStatus::Pending,
            started_at: Utc::now(),
            ended_at: None,
            checkpoint: None,
        };

        // Save initial metadata
        self.replay_storage
            .save_metadata(&metadata)
            .await
            .map_err(ReplayError::StorageError)?;

        // Determine start and end ledgers
        let (start_ledger, end_ledger) = self.determine_ledger_range().await?;

        info!("Replay range: ledger {} to {}", start_ledger, end_ledger);

        // Update status to in progress
        metadata.status = ReplayStatus::InProgress {
            current_ledger: start_ledger,
            events_processed: 0,
            events_failed: 0,
        };
        self.replay_storage
            .save_metadata(&metadata)
            .await
            .map_err(ReplayError::StorageError)?;

        // Execute replay
        let start_time = Instant::now();
        match self
            .execute_replay(start_ledger, end_ledger, &mut metadata)
            .await
        {
            Ok((processed, failed)) => {
                let duration = start_time.elapsed().as_secs();
                metadata.status = ReplayStatus::Completed {
                    events_processed: processed,
                    events_failed: failed,
                    duration_secs: duration,
                };
                metadata.ended_at = Some(Utc::now());

                info!(
                    "Replay completed: {} events processed, {} failed in {}s",
                    processed, failed, duration
                );
            }
            Err(e) => {
                error!("Replay failed: {}", e);
                metadata.status = ReplayStatus::Failed {
                    error: e.to_string(),
                    last_ledger: self.get_current_ledger(&metadata),
                };
                metadata.ended_at = Some(Utc::now());
            }
        }

        // Save final metadata
        self.replay_storage
            .save_metadata(&metadata)
            .await
            .map_err(ReplayError::StorageError)?;

        Ok(metadata)
    }

    /// Execute the replay process
    async fn execute_replay(
        &self,
        start_ledger: u64,
        end_ledger: u64,
        metadata: &mut ReplayMetadata,
    ) -> Result<(u64, u64)> {
        let mut current_ledger = start_ledger;
        let mut total_processed = 0u64;
        let mut total_failed = 0u64;

        // Create base processing context.  The protocol version will be
        // resolved per batch and applied via `with_protocol_version` before
        // each event is processed.
        let base_context =
            ProcessingContext::for_replay(self.session_id.clone(), self.config.dry_run);

        while current_ledger <= end_ledger {
            // Fetch batch of events
            let batch_end = (current_ledger + self.config.batch_size as u64 - 1).min(end_ledger);

            info!(
                "Processing ledgers {} to {} (batch size: {})",
                current_ledger, batch_end, self.config.batch_size
            );

            let events = self
                .event_storage
                .get_events_in_range(
                    current_ledger,
                    batch_end,
                    &self.config.filter,
                    Some(self.config.batch_size),
                )
                .await
                .context("Failed to fetch events")?;

            info!("Fetched {} events in batch", events.len());

            // Resolve the protocol version per event ledger (#2768): a batch
            // can straddle a protocol upgrade, so each event is decoded with
            // the rules active when its own ledger closed. Lookups are
            // cached per ledger within the batch.
            let mut versions: HashMap<(String, u64), u32> = HashMap::new();

            for event in &events {
                let key = (event.network.clone(), event.ledger_sequence);
                let resolved_protocol = if let Some(v) = versions.get(&key) {
                    *v
                } else {
                    let v = self
                        .resolve_protocol_version(&event.network, event.ledger_sequence)
                        .await?;
                    versions.insert(key, v);
                    v
                };

                // Soroban not active for this ledger: nothing to replay.
                if !LedgerProtocolVersion(resolved_protocol).supports_soroban() {
                    info!(
                        ledger = event.ledger_sequence,
                        protocol = resolved_protocol,
                        "Skipping event — Soroban not active for this protocol version"
                    );
                    continue;
                }

                let mut context = base_context
                    .clone()
                    .with_protocol_version(resolved_protocol);
                context.current_ledger = event.ledger_sequence;

                match self.process_event(event, &context).await {
                    Ok(result) => {
                        if result.success {
                            total_processed += 1;

                            // Apply to state builder
                            if self.config.mode == ReplayMode::Full
                                || self.config.mode == ReplayMode::Verification
                            {
                                let mut state_builder = self.state_builder.write().await;
                                state_builder.apply_event(event, resolved_protocol).await?;
                            }
                        } else {
                            total_failed += 1;
                            warn!("Event {} failed: {:?}", event.unique_id(), result.error);
                        }
                    }
                    Err(e) => {
                        total_failed += 1;
                        error!("Error processing event {}: {}", event.unique_id(), e);
                    }
                }
            }

            // Update current ledger
            current_ledger = batch_end + 1;

            // Update metadata
            metadata.status = ReplayStatus::InProgress {
                current_ledger,
                events_processed: total_processed,
                events_failed: total_failed,
            };

            // Checkpoint if needed
            if current_ledger.is_multiple_of(self.config.checkpoint_interval) {
                self.create_checkpoint(current_ledger, total_processed, total_failed, metadata)
                    .await?;
            }

            // Save metadata periodically
            self.replay_storage.save_metadata(metadata).await?;
        }

        // Final checkpoint
        self.create_checkpoint(end_ledger, total_processed, total_failed, metadata)
            .await?;

        // Persist final state
        if !self.config.dry_run {
            let state_builder = self.state_builder.read().await;
            state_builder.persist_state().await?;
        }

        Ok((total_processed, total_failed))
    }

    /// Resolve the Stellar protocol version for `current_ledger` from the
    /// ledger headers stored at ingestion. See [`resolve_protocol_version`].
    async fn resolve_protocol_version(
        &self,
        network: &str,
        current_ledger: u64,
    ) -> ReplayResult<u32> {
        resolve_protocol_version(
            self.config.protocol_version,
            self.protocol_source.as_deref(),
            network,
            current_ledger,
        )
        .await
    }

    /// Process a single event
    async fn process_event(
        &self,
        event: &ContractEvent,
        context: &ProcessingContext,
    ) -> Result<super::event_processor::ProcessingResult> {
        self.processor
            .process_with_retry(event, context, self.config.max_retries)
            .await
    }

    /// Create a checkpoint
    async fn create_checkpoint(
        &self,
        ledger: u64,
        processed: u64,
        failed: u64,
        metadata: &mut ReplayMetadata,
    ) -> Result<()> {
        info!("Creating checkpoint at ledger {}", ledger);

        // Get current state
        let state_builder = self.state_builder.read().await;
        let state_json = state_builder.state().to_json()?;

        // Create checkpoint, tagging it with the active protocol version so
        // that a resumed session can restore the correct processing context.
        let checkpoint = Checkpoint::new(self.session_id.clone(), ledger)
            .with_stats(processed, failed)
            .with_state(state_json)
            .with_metadata("mode".to_string(), self.config.mode.to_string())
            .with_metadata(
                "protocol_version".to_string(),
                self.config.protocol_version.to_string(),
            );

        // Save checkpoint
        self.checkpoint_manager.save(&checkpoint).await?;

        // Update metadata
        metadata.checkpoint = Some(checkpoint);

        Ok(())
    }

    /// Determine the ledger range for replay
    async fn determine_ledger_range(&self) -> Result<(u64, u64)> {
        let latest_ledger = self.event_storage.get_latest_ledger().await?.unwrap_or(0);

        let checkpoint_ledger = if let Some(checkpoint_id) = self.get_checkpoint_id() {
            self.checkpoint_manager
                .load(&checkpoint_id)
                .await?
                .map(|c| c.last_ledger)
        } else {
            None
        };

        let start = self
            .config
            .range
            .start_ledger(latest_ledger, checkpoint_ledger)
            .unwrap_or(0);
        let end = self
            .config
            .range
            .end_ledger(latest_ledger)
            .unwrap_or(latest_ledger);

        Ok((start, end))
    }

    /// Get checkpoint ID from config if resuming
    fn get_checkpoint_id(&self) -> Option<String> {
        match &self.config.range {
            super::config::ReplayRange::FromCheckpoint { checkpoint_id } => {
                Some(checkpoint_id.clone())
            }
            _ => None,
        }
    }

    /// Get current ledger from metadata
    fn get_current_ledger(&self, metadata: &ReplayMetadata) -> Option<u64> {
        match &metadata.status {
            ReplayStatus::InProgress { current_ledger, .. } => Some(*current_ledger),
            ReplayStatus::Paused { last_ledger, .. } => Some(*last_ledger),
            _ => None,
        }
    }

    /// Pause the replay
    pub fn pause(&self) -> Result<()> {
        info!("Pausing replay session {}", self.session_id);
        // Implementation would set a flag that the execute loop checks
        Ok(())
    }

    /// Resume a paused replay
    pub fn resume(&self) -> Result<()> {
        info!("Resuming replay session {}", self.session_id);
        // Implementation would clear the pause flag
        Ok(())
    }

    /// Get replay status
    pub async fn get_status(&self) -> Result<ReplayMetadata> {
        self.replay_storage
            .load_metadata(&self.session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Replay session not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::event_processor::{
        CompositeEventProcessor, EventProcessor, ProcessingContext, ProcessingResult,
    };
    use crate::replay::ledger_headers::InMemoryProtocolVersions;
    use async_trait::async_trait;

    #[tokio::test]
    async fn missing_header_is_an_explicit_error() {
        let source = InMemoryProtocolVersions::default();
        let err = resolve_protocol_version(0, Some(&source), "mainnet", 42)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ReplayError::MissingLedgerHeader { ledger: 42, .. }
        ));
        // No source at all is also an error, not 0.
        assert!(resolve_protocol_version(0, None, "mainnet", 42)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn override_wins_and_lookup_is_per_network() {
        let source = InMemoryProtocolVersions::default()
            .with("mainnet", 10, 22)
            .with("testnet", 10, 23);
        assert_eq!(
            resolve_protocol_version(21, Some(&source), "mainnet", 10)
                .await
                .unwrap(),
            21
        );
        assert_eq!(
            resolve_protocol_version(0, Some(&source), "mainnet", 10)
                .await
                .unwrap(),
            22
        );
        assert_eq!(
            resolve_protocol_version(0, Some(&source), "testnet", 10)
                .await
                .unwrap(),
            23
        );
    }

    struct Tagging(&'static str);

    #[async_trait]
    impl EventProcessor for Tagging {
        async fn process_event(
            &self,
            _event: &ContractEvent,
            _context: &ProcessingContext,
        ) -> anyhow::Result<ProcessingResult> {
            Ok(ProcessingResult::success().with_change(
                crate::replay::event_processor::StateChange {
                    change_type: "decoded".into(),
                    entity_type: "event".into(),
                    entity_id: self.0.into(),
                    previous_value: None,
                    new_value: None,
                },
            ))
        }
        async fn is_processed(&self, _event: &ContractEvent) -> anyhow::Result<bool> {
            Ok(false)
        }
        async fn mark_processed(&self, _event: &ContractEvent) -> anyhow::Result<()> {
            Ok(())
        }
        fn name(&self) -> &str {
            self.0
        }
    }

    fn event_at(ledger: u64) -> ContractEvent {
        ContractEvent {
            id: format!("e{ledger}"),
            ledger_sequence: ledger,
            transaction_hash: "tx".into(),
            contract_id: "c".into(),
            event_type: "t".into(),
            data: serde_json::json!({}),
            timestamp: Utc::now(),
            network: "mainnet".into(),
        }
    }

    /// Fixture spanning the protocol 22 -> 23 (CAP-67) boundary at ledger
    /// 1000: each side must be decoded by its own handler.
    #[tokio::test]
    async fn replay_across_protocol_boundary_selects_handler_per_ledger() {
        let source = InMemoryProtocolVersions::default()
            .with("mainnet", 999, 22)
            .with("mainnet", 1000, 23);
        let processor = CompositeEventProcessor::new()
            .add_versioned_processor(20, Some(22), Arc::new(Tagging("legacy")))
            .add_versioned_processor(23, None, Arc::new(Tagging("cap67")));

        let mut seen = Vec::new();
        for ev in [event_at(999), event_at(1000)] {
            let v = resolve_protocol_version(0, Some(&source), &ev.network, ev.ledger_sequence)
                .await
                .unwrap();
            let ctx = ProcessingContext::for_replay("s".into(), true).with_protocol_version(v);
            let result = processor.process_with_retry(&ev, &ctx, 0).await.unwrap();
            seen.push(result.state_changes[0].entity_id.clone());
        }
        assert_eq!(seen, vec!["legacy", "cap67"]);
    }
}
