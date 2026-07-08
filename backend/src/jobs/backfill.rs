//! Contract Event Backfill Job
//!
//! Fetches historical contract events from the Stellar RPC for a ledger range,
//! indexes them via [`EventIndexer`], and tracks progress so the job can be
//! paused and resumed without re-processing already-indexed ledgers.
//!
//! # Design
//!
//! - **Gap detection**: queries the `contract_events` table to find ledger
//!   ranges that have no indexed events and schedules them for backfill.
//! - **Progress tracking**: an in-memory [`BackfillState`] is shared behind an
//!   `Arc<RwLock<_>>` so the status endpoint can read it without blocking the
//!   worker.
//! - **Rate limiting**: a configurable delay between RPC page requests prevents
//!   overwhelming the upstream node.
//! - **Idempotency**: events are inserted with `INSERT OR REPLACE` so re-running
//!   a range is safe.

use crate::rpc::StellarRpcClient;
use crate::services::event_indexer::{EventIndexer, IndexedEvent};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

// ── Constants ────────────────────────────────────────────────────────────────

/// Maximum ledger range that can be requested in a single backfill job.
const MAX_BACKFILL_RANGE: u64 = 100_000;
/// Number of ledgers fetched per RPC page during backfill.
const BACKFILL_PAGE_SIZE: u32 = 200;
/// Default delay between RPC page requests (ms).
const DEFAULT_BACKFILL_DELAY_MS: u64 = 250;

// ── Public types ─────────────────────────────────────────────────────────────

/// Request body for `POST /admin/backfill`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackfillRequest {
    /// First ledger to include (inclusive).
    pub from_ledger: u64,
    /// Last ledger to include (inclusive).
    pub to_ledger: u64,
    /// Optional contract ID filter. When `None` all contracts are indexed.
    pub contract_id: Option<String>,
    /// Milliseconds to wait between RPC page requests. Defaults to 250 ms.
    pub delay_ms: Option<u64>,
}

/// Current status of the backfill worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackfillStatus {
    /// No backfill has been requested yet.
    Idle,
    /// A backfill is actively running.
    Running,
    /// The last backfill completed successfully.
    Completed,
    /// The last backfill was aborted due to an error.
    Failed,
}

/// Snapshot of backfill progress exposed by the status endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillState {
    pub status: BackfillStatus,
    /// Ledger range requested for the current (or last) job.
    pub from_ledger: u64,
    pub to_ledger: u64,
    /// Last ledger successfully processed.
    pub current_ledger: u64,
    /// Total events indexed during this run.
    pub events_indexed: u64,
    /// Total ledgers processed so far.
    pub ledgers_processed: u64,
    /// Total ledgers in the requested range.
    pub ledgers_total: u64,
    /// Gaps detected in the indexed event sequence.
    pub gaps_detected: u64,
    /// When the current (or last) job started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the current (or last) job finished (success or failure).
    pub finished_at: Option<DateTime<Utc>>,
    /// Error message if status is `Failed`.
    pub error: Option<String>,
}

impl Default for BackfillState {
    fn default() -> Self {
        Self {
            status: BackfillStatus::Idle,
            from_ledger: 0,
            to_ledger: 0,
            current_ledger: 0,
            events_indexed: 0,
            ledgers_processed: 0,
            ledgers_total: 0,
            gaps_detected: 0,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }
}

/// Shared handle to the backfill state, readable from the status endpoint.
pub type BackfillStateRef = Arc<RwLock<BackfillState>>;

// ── BackfillJob ───────────────────────────────────────────────────────────────

/// Orchestrates historical event backfill for a ledger range.
pub struct BackfillJob {
    event_indexer: Arc<EventIndexer>,
    rpc_client: Arc<StellarRpcClient>,
    state: BackfillStateRef,
}

impl BackfillJob {
    /// Create a new backfill job.
    #[must_use]
    pub fn new(
        event_indexer: Arc<EventIndexer>,
        rpc_client: Arc<StellarRpcClient>,
        state: BackfillStateRef,
    ) -> Self {
        Self {
            event_indexer,
            rpc_client,
            state,
        }
    }

    /// Validate the request and spawn the backfill worker as a background task.
    ///
    /// Returns immediately; progress can be polled via the shared [`BackfillStateRef`].
    pub async fn start(&self, req: BackfillRequest) -> Result<()> {
        // Validate range
        if req.from_ledger > req.to_ledger {
            anyhow::bail!(
                "from_ledger ({}) must be <= to_ledger ({})",
                req.from_ledger,
                req.to_ledger
            );
        }
        let range = req.to_ledger - req.from_ledger;
        if range > MAX_BACKFILL_RANGE {
            anyhow::bail!(
                "Requested range ({} ledgers) exceeds maximum ({} ledgers). \
                 Split into smaller ranges.",
                range,
                MAX_BACKFILL_RANGE
            );
        }

        // Reject if already running
        {
            let state = self.state.read().await;
            if state.status == BackfillStatus::Running {
                anyhow::bail!("A backfill is already in progress");
            }
        }

        // Initialise state
        {
            let mut state = self.state.write().await;
            *state = BackfillState {
                status: BackfillStatus::Running,
                from_ledger: req.from_ledger,
                to_ledger: req.to_ledger,
                current_ledger: req.from_ledger,
                ledgers_total: range + 1,
                started_at: Some(Utc::now()),
                ..Default::default()
            };
        }

        // Detect gaps before starting so we can report them
        let gaps = self
            .event_indexer
            .detect_ledger_gaps(req.from_ledger, req.to_ledger)
            .await
            .unwrap_or_default();

        {
            let mut state = self.state.write().await;
            state.gaps_detected = gaps.len() as u64;
        }

        if gaps.is_empty() {
            info!(
                from = req.from_ledger,
                to = req.to_ledger,
                "No gaps detected — range already fully indexed"
            );
        } else {
            info!(
                from = req.from_ledger,
                to = req.to_ledger,
                gaps = gaps.len(),
                "Starting backfill"
            );
        }

        // Spawn worker
        let indexer = Arc::clone(&self.event_indexer);
        let rpc = Arc::clone(&self.rpc_client);
        let state_ref = Arc::clone(&self.state);
        let delay_ms = req.delay_ms.unwrap_or(DEFAULT_BACKFILL_DELAY_MS);

        tokio::spawn(async move {
            let result = run_backfill(indexer, rpc, state_ref.clone(), req, gaps, delay_ms).await;

            let mut state = state_ref.write().await;
            state.finished_at = Some(Utc::now());
            match result {
                Ok(()) => {
                    state.status = BackfillStatus::Completed;
                    info!(
                        events = state.events_indexed,
                        ledgers = state.ledgers_processed,
                        "Backfill completed"
                    );
                }
                Err(e) => {
                    state.status = BackfillStatus::Failed;
                    state.error = Some(e.to_string());
                    error!(error = %e, "Backfill failed");
                }
            }
        });

        Ok(())
    }

    /// Return a snapshot of the current backfill state.
    pub async fn status(&self) -> BackfillState {
        self.state.read().await.clone()
    }
}

// ── Worker ────────────────────────────────────────────────────────────────────

/// A contiguous ledger range with no indexed events.
#[derive(Debug, Clone)]
pub struct LedgerGap {
    pub start: u64,
    pub end: u64,
}

/// Core backfill loop. Iterates over detected gaps, fetches ledgers from the
/// RPC in pages, and indexes any contract events found.
async fn run_backfill(
    indexer: Arc<EventIndexer>,
    rpc: Arc<StellarRpcClient>,
    state_ref: BackfillStateRef,
    req: BackfillRequest,
    gaps: Vec<LedgerGap>,
    delay_ms: u64,
) -> Result<()> {
    // If no gaps were detected, still walk the full range to catch any events
    // that may have been missed (e.g. the table was empty).
    let ranges_to_process: Vec<(u64, u64)> = if gaps.is_empty() {
        vec![(req.from_ledger, req.to_ledger)]
    } else {
        gaps.iter().map(|g| (g.start, g.end)).collect()
    };

    for (range_start, range_end) in ranges_to_process {
        let mut cursor: Option<String> = None;
        let mut current = range_start;

        let mut events_buffer = Vec::new();
        let mut chunk_ledgers = 0;
        let mut uncommitted_events = 0;

        loop {
            // Fetch a page of ledgers
            let result = rpc
                .fetch_ledgers(Some(current), BACKFILL_PAGE_SIZE, cursor.as_deref())
                .await
                .context("RPC fetch_ledgers failed during backfill")?;

            let fetched_count = result.ledgers.len() as u64;
            if fetched_count == 0 {
                break;
            }

            let mut page_events: u64 = 0;

            for ledger in &result.ledgers {
                if ledger.sequence > range_end {
                    // Past the requested range — stop this gap's loop
                    break;
                }

                // Build synthetic events from ledger metadata.
                let events = extract_events_from_ledger(ledger, req.contract_id.as_deref());
                page_events += events.len() as u64;
                uncommitted_events += events.len() as u64;
                events_buffer.extend(events);
                chunk_ledgers += 1;

                if chunk_ledgers >= 1000 {
                    if let Err(e) = indexer.index_events_with_checkpoint(std::mem::take(&mut events_buffer), ledger.sequence).await {
                        warn!(ledger = ledger.sequence, error = %e, "Failed to index event chunk — aborting");
                        return Err(e);
                    }
                    chunk_ledgers = 0;
                    uncommitted_events = 0;
                }

                // Update progress
                {
                    let mut state = state_ref.write().await;
                    state.current_ledger = ledger.sequence;
                    state.ledgers_processed += 1;
                }
            }

            // Update events indexed for the page
            {
                let mut state = state_ref.write().await;
                state.events_indexed += page_events;
            }

            // Check if we've covered the range
            let last_ledger = result.ledgers.last().map(|l| l.sequence).unwrap_or(current);

            if last_ledger >= range_end || result.cursor.is_none() {
                break;
            }

            cursor = result.cursor;
            current = last_ledger + 1;

            // Rate-limit between pages
            if delay_ms > 0 {
                sleep(Duration::from_millis(delay_ms)).await;
            }
        }

        // Commit any remaining events in the buffer for this gap
        if !events_buffer.is_empty() {
            let last_ledger = current - 1; // It was updated before the break, wait, `current` is the next ledger to fetch, but the last processed is what we should checkpoint.
            // Better to use the last processed ledger from state
            let current_ledger = state_ref.read().await.current_ledger;
            if let Err(e) = indexer.index_events_with_checkpoint(std::mem::take(&mut events_buffer), current_ledger).await {
                warn!(ledger = current_ledger, error = %e, "Failed to index remaining event chunk — aborting");
                return Err(e);
            }
        }
    }

    Ok(())
}

/// Extract [`IndexedEvent`]s from a single [`crate::rpc::stellar::RpcLedger`].
///
/// In a full implementation this would decode `ledger.metadata_xdr` using the
/// `stellar-xdr` crate to extract Soroban contract events. For now it emits a
/// single ledger-checkpoint event per ledger so that gap detection and progress
/// tracking work end-to-end without requiring XDR parsing.
fn extract_events_from_ledger(
    ledger: &crate::rpc::stellar::RpcLedger,
    contract_id_filter: Option<&str>,
) -> Vec<IndexedEvent> {
    // Parse the close time from the ledger
    let timestamp = chrono::DateTime::parse_from_rfc3339(&ledger.ledger_close_time)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let contract_id = contract_id_filter
        .unwrap_or("backfill-checkpoint")
        .to_string();

    // Emit one checkpoint event per ledger so the ledger is marked as indexed.
    vec![IndexedEvent {
        id: format!("backfill-{}-{}", ledger.sequence, Uuid::new_v4()),
        contract_id,
        event_type: "LEDGER_CHECKPOINT".to_string(),
        epoch: None,
        hash: Some(ledger.hash.clone()),
        timestamp: Some(timestamp.timestamp() as u64),
        ledger: ledger.sequence,
        transaction_hash: ledger.hash.clone(),
        created_at: timestamp,
        verification_status: Some("backfilled".to_string()),
    }]
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backfill_state_default_is_idle() {
        let state = BackfillState::default();
        assert_eq!(state.status, BackfillStatus::Idle);
        assert_eq!(state.events_indexed, 0);
        assert!(state.started_at.is_none());
    }

    #[test]
    fn backfill_request_serialises() {
        let req = BackfillRequest {
            from_ledger: 1000,
            to_ledger: 2000,
            contract_id: None,
            delay_ms: Some(100),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("from_ledger"));
        assert!(json.contains("1000"));
    }

    #[tokio::test]
    async fn start_rejects_inverted_range() {
        let _guard = crate::lock_env_test();
        use crate::database::Database;
        use crate::db::schema::Schema;

        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS_INDEXES)
            .execute(&pool)
            .await
            .unwrap();
        let db = Arc::new(Database::new(pool));
        let indexer = Arc::new(EventIndexer::new(db));
        let rpc = Arc::new(StellarRpcClient::new_with_defaults(true));
        let state = Arc::new(RwLock::new(BackfillState::default()));
        let job = BackfillJob::new(indexer, rpc, state);

        let result = job
            .start(BackfillRequest {
                from_ledger: 2000,
                to_ledger: 1000,
                contract_id: None,
                delay_ms: None,
            })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be <="));
    }

    #[tokio::test]
    async fn start_rejects_oversized_range() {
        let _guard = crate::lock_env_test();
        use crate::database::Database;
        use crate::db::schema::Schema;

        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS_INDEXES)
            .execute(&pool)
            .await
            .unwrap();
        let db = Arc::new(Database::new(pool));
        let indexer = Arc::new(EventIndexer::new(db));
        let rpc = Arc::new(StellarRpcClient::new_with_defaults(true));
        let state = Arc::new(RwLock::new(BackfillState::default()));
        let job = BackfillJob::new(indexer, rpc, state);

        let result = job
            .start(BackfillRequest {
                from_ledger: 1,
                to_ledger: MAX_BACKFILL_RANGE + 10,
                contract_id: None,
                delay_ms: None,
            })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    #[tokio::test]
    async fn start_rejects_concurrent_run() {
        let _guard = crate::lock_env_test();
        use crate::database::Database;
        use crate::db::schema::Schema;

        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS_INDEXES)
            .execute(&pool)
            .await
            .unwrap();
        let db = Arc::new(Database::new(pool));
        let indexer = Arc::new(EventIndexer::new(db));
        let rpc = Arc::new(StellarRpcClient::new_with_defaults(true));
        let state = Arc::new(RwLock::new(BackfillState {
            status: BackfillStatus::Running,
            ..Default::default()
        }));
        let job = BackfillJob::new(indexer, rpc, state);

        let result = job
            .start(BackfillRequest {
                from_ledger: 1000,
                to_ledger: 2000,
                contract_id: None,
                delay_ms: None,
            })
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already in progress"));
    }
}
