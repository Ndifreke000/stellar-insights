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
    state_builder::StateBuilder,
    storage::{EventStorage, ReplayStorage},
    ContractEvent, EventFilter, ReplayError, ReplayMetadata, ReplayResult, ReplayStatus,
};

/// Main replay engine
pub struct ReplayEngine {
    config: ReplayConfig,
    event_storage: Arc<EventStorage>,
    replay_storage: Arc<ReplayStorage>,
    checkpoint_manager: Arc<CheckpointManager>,
    processor: Arc<CompositeEventProcessor>,
    state_builder: Arc<RwLock<StateBuilder>>,
    session_id: String,
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
        config.validate().map_err(|e| ReplayError::ConfigError(e))?;

        let session_id = uuid::Uuid::new_v4().to_string();

        Ok(Self {
            config,
            event_storage,
            replay_storage,
            checkpoint_manager,
            processor,
            state_builder,
            session_id,
        })
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
            .map_err(|e| ReplayError::StorageError(e))?;

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
            .map_err(|e| ReplayError::StorageError(e))?;

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
                    last_ledger: self.get_current_ledger(&metadata).await,
                };
                metadata.ended_at = Some(Utc::now());
            }
        }

        // Save final metadata
        self.replay_storage
            .save_metadata(&metadata)
            .await
            .map_err(|e| ReplayError::StorageError(e))?;

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

        // Create processing context
        let context = ProcessingContext::for_replay(self.session_id.clone(), self.config.dry_run);

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

            // Process events
            for event in &events {
                match self.process_event(event, &context).await {
                    Ok(result) => {
                        if result.success {
                            total_processed += 1;

                            // Apply to state builder
                            if self.config.mode == ReplayMode::Full
                                || self.config.mode == ReplayMode::Verification
                            {
                                let mut state_builder = self.state_builder.write().await;
                                state_builder.apply_event(event).await?;
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
            if current_ledger % self.config.checkpoint_interval == 0 {
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

        // Create checkpoint
        let checkpoint = Checkpoint::new(self.session_id.clone(), ledger)
            .with_stats(processed, failed)
            .with_state(state_json)
            .with_metadata("mode".to_string(), self.config.mode.to_string());

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
    async fn get_current_ledger(&self, metadata: &ReplayMetadata) -> Option<u64> {
        match &metadata.status {
            ReplayStatus::InProgress { current_ledger, .. } => Some(*current_ledger),
            ReplayStatus::Paused { last_ledger, .. } => Some(*last_ledger),
            _ => None,
        }
    }

    /// Pause the replay
    pub async fn pause(&self) -> Result<()> {
        info!("Pausing replay session {}", self.session_id);
        // Implementation would set a flag that the execute loop checks
        Ok(())
    }

    /// Resume a paused replay
    pub async fn resume(&self) -> Result<()> {
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

    #[test]
    fn test_replay_engine_creation() {
        // Placeholder for actual tests
        // Would require mock implementations of dependencies
    }
}
