//! Replay Configuration
//!
//! Defines configuration options for replay operations including
//! network selection, block ranges, and processing parameters.

use serde::{Deserialize, Serialize};

use super::EventFilter;

/// Configuration for a replay operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayConfig {
    /// Replay mode
    pub mode: ReplayMode,
    /// Range of ledgers to replay
    pub range: ReplayRange,
    /// Event filter
    pub filter: EventFilter,
    /// Batch size for processing
    pub batch_size: usize,
    /// Maximum concurrent workers
    pub max_workers: usize,
    /// Enable dry-run mode (no state changes)
    pub dry_run: bool,
    /// Enable verbose logging
    pub verbose: bool,
    /// Checkpoint interval (save state every N ledgers)
    pub checkpoint_interval: u64,
    /// Timeout for processing a single event (seconds)
    pub event_timeout_secs: u64,
    /// Maximum retries for failed events
    pub max_retries: u32,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            mode: ReplayMode::Full,
            range: ReplayRange::All,
            filter: EventFilter::default(),
            batch_size: 100,
            max_workers: 4,
            dry_run: false,
            verbose: false,
            checkpoint_interval: 1000,
            event_timeout_secs: 30,
            max_retries: 3,
        }
    }
}

impl ReplayConfig {
    /// Create a new replay config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set replay mode
    pub fn with_mode(mut self, mode: ReplayMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set ledger range
    pub fn with_range(mut self, range: ReplayRange) -> Self {
        self.range = range;
        self
    }

    /// Set event filter
    pub fn with_filter(mut self, filter: EventFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Enable dry-run mode
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Enable verbose logging
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.batch_size == 0 {
            return Err("Batch size must be greater than 0".to_string());
        }

        if self.max_workers == 0 {
            return Err("Max workers must be greater than 0".to_string());
        }

        if self.checkpoint_interval == 0 {
            return Err("Checkpoint interval must be greater than 0".to_string());
        }

        if self.event_timeout_secs == 0 {
            return Err("Event timeout must be greater than 0".to_string());
        }

        match &self.range {
            ReplayRange::FromTo { start, end } => {
                if start > end {
                    return Err(format!("Invalid range: start ({}) > end ({})", start, end));
                }
            }
            ReplayRange::FromCheckpoint { checkpoint_id } => {
                if checkpoint_id.is_empty() {
                    return Err("Checkpoint ID cannot be empty".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// Replay mode determines how events are processed
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplayMode {
    /// Full replay - rebuild entire state from scratch
    Full,
    /// Incremental - only process new events since last checkpoint
    Incremental,
    /// Verification - replay and compare with existing state
    Verification,
    /// Debug - replay with detailed logging and no state changes
    Debug,
}

impl std::fmt::Display for ReplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "Full"),
            Self::Incremental => write!(f, "Incremental"),
            Self::Verification => write!(f, "Verification"),
            Self::Debug => write!(f, "Debug"),
        }
    }
}

/// Range of ledgers to replay
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplayRange {
    /// Replay all available events
    All,
    /// Replay from a specific ledger to the latest
    From { start: u64 },
    /// Replay up to a specific ledger
    To { end: u64 },
    /// Replay a specific range
    FromTo { start: u64, end: u64 },
    /// Replay from a checkpoint
    FromCheckpoint { checkpoint_id: String },
    /// Replay last N ledgers
    Last { count: u64 },
}

impl ReplayRange {
    /// Get the start ledger for this range
    pub fn start_ledger(&self, latest: u64, checkpoint_ledger: Option<u64>) -> Option<u64> {
        match self {
            Self::All => Some(0),
            Self::From { start } => Some(*start),
            Self::To { .. } => Some(0),
            Self::FromTo { start, .. } => Some(*start),
            Self::FromCheckpoint { .. } => checkpoint_ledger,
            Self::Last { count } => Some(latest.saturating_sub(*count)),
        }
    }

    /// Get the end ledger for this range
    pub fn end_ledger(&self, latest: u64) -> Option<u64> {
        match self {
            Self::All => Some(latest),
            Self::From { .. } => Some(latest),
            Self::To { end } => Some(*end),
            Self::FromTo { end, .. } => Some(*end),
            Self::FromCheckpoint { .. } => Some(latest),
            Self::Last { .. } => Some(latest),
        }
    }

    /// Check if a ledger is within this range
    pub fn contains(&self, ledger: u64, latest: u64, checkpoint_ledger: Option<u64>) -> bool {
        let start = self.start_ledger(latest, checkpoint_ledger).unwrap_or(0);
        let end = self.end_ledger(latest).unwrap_or(u64::MAX);
        ledger >= start && ledger <= end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_config_validation() {
        let config = ReplayConfig::default();
        assert!(config.validate().is_ok());

        let invalid_config = ReplayConfig {
            batch_size: 0,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_replay_range_contains() {
        let range = ReplayRange::FromTo {
            start: 100,
            end: 200,
        };
        assert!(range.contains(150, 1000, None));
        assert!(!range.contains(50, 1000, None));
        assert!(!range.contains(250, 1000, None));
    }

    #[test]
    fn test_replay_range_last() {
        let range = ReplayRange::Last { count: 100 };
        assert_eq!(range.start_ledger(1000, None), Some(900));
        assert_eq!(range.end_ledger(1000), Some(1000));
    }
}
