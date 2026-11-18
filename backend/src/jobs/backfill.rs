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
//! - **RPC retention window** (#2770): Stellar RPC only keeps events for a
//!   limited window (~7 days by default). Before backfilling, the job queries
//!   `getHealth` for `oldestLedger`/`latestLedger` and splits each requested
//!   range into an in-window part (backfilled from RPC) and an out-of-window
//!   part. Out-of-window ranges are **never** marked as covered: they are
//!   recorded as `unrecoverable_via_rpc`, an alert is raised, and they are
//!   handed to a configured [`HistoricalEventSource`] (e.g. a Galexie/S3
//!   ledger-meta lake) if one is set. They are surfaced on
//!   `GET /admin/backfill/health`.
//! - **Rate limiting**: a configurable delay between RPC page requests prevents
//!   overwhelming the upstream node.
//! - **Idempotency**: events are inserted with `INSERT … ON CONFLICT DO NOTHING`
//!   on the `(network, ledger, tx_hash, op_index, event_index)` identity key and
//!   the checkpoint only moves forward, so re-running a range is safe (#2769).

use crate::observability::job_metrics::JobMetricsCollector;
use crate::rpc::StellarRpcClient;
use crate::services::event_indexer::{EventIndexer, IndexedEvent};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

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
    /// Ranges from this run that fell outside the RPC retention window and
    /// were not backfilled (#2770).
    #[serde(default)]
    pub unrecoverable_ranges: Vec<UnrecoverableRange>,
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
            unrecoverable_ranges: Vec::new(),
        }
    }
}

/// Why a range could not be backfilled.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnrecoverableReason {
    /// The range is older than the RPC's `oldestLedger`.
    UnrecoverableViaRpc,
}

/// A ledger range that RPC can no longer serve (#2770).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnrecoverableRange {
    pub start: u64,
    pub end: u64,
    pub reason: UnrecoverableReason,
    /// `oldestLedger` reported by RPC when the range was evaluated.
    pub rpc_oldest_ledger: u64,
    pub detected_at: DateTime<Utc>,
}

/// Result of splitting a requested range against the RPC retention window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RetentionSplit {
    /// Part of the range RPC can still serve.
    pub in_window: Option<(u64, u64)>,
    /// Part older than `oldest_ledger` that RPC can no longer serve.
    pub out_of_window: Option<(u64, u64)>,
}

/// Split the inclusive range `[start, end]` by the RPC retention window
/// `[oldest, latest]`. Ledgers newer than `latest` don't exist yet and are
/// dropped from the in-window part (they'll be picked up by tailing).
#[must_use]
pub fn split_by_retention(start: u64, end: u64, oldest: u64, latest: u64) -> RetentionSplit {
    if start > end {
        return RetentionSplit::default();
    }
    let out_of_window = (start < oldest).then(|| (start, end.min(oldest.saturating_sub(1))));
    let lo = start.max(oldest);
    let hi = end.min(latest);
    let in_window = (lo <= hi).then_some((lo, hi));
    RetentionSplit {
        in_window,
        out_of_window,
    }
}

/// A full-history event source used for ranges outside RPC retention
/// (e.g. Galexie ledger-meta in S3/GCS, or Hubble).
#[async_trait::async_trait]
pub trait HistoricalEventSource: Send + Sync {
    /// Human-readable name for logs/health output.
    fn name(&self) -> &str;

    /// Fetch events for the inclusive ledger range `[start, end]`.
    async fn fetch_events(
        &self,
        start: u64,
        end: u64,
        contract_id: Option<&str>,
    ) -> Result<Vec<IndexedEvent>>;
}

/// Stub for a Galexie / S3 ledger-meta source. Configured with the bucket
/// URL; decoding `LedgerCloseMeta` batches is not implemented yet, so every
/// call fails and the range stays flagged as unrecoverable (never covered).
#[derive(Debug, Clone)]
pub struct GalexieS3Source {
    pub bucket_url: String,
}

#[async_trait::async_trait]
impl HistoricalEventSource for GalexieS3Source {
    fn name(&self) -> &str {
        "galexie-s3"
    }

    async fn fetch_events(
        &self,
        start: u64,
        end: u64,
        _contract_id: Option<&str>,
    ) -> Result<Vec<IndexedEvent>> {
        anyhow::bail!(
            "Galexie source at {} cannot decode ledger-meta yet (range {start}-{end})",
            self.bucket_url
        )
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
    historical_source: Option<Arc<dyn HistoricalEventSource>>,
    /// All unrecoverable ranges seen since startup, for the health endpoint.
    unrecoverable: Arc<RwLock<Vec<UnrecoverableRange>>>,
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
            historical_source: None,
            unrecoverable: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Use `source` to fill ranges that fall outside RPC retention.
    #[must_use]
    pub fn with_historical_source(mut self, source: Arc<dyn HistoricalEventSource>) -> Self {
        self.historical_source = Some(source);
        self
    }

    /// Every range flagged `unrecoverable_via_rpc` since startup.
    pub async fn unrecoverable_ranges(&self) -> Vec<UnrecoverableRange> {
        self.unrecoverable.read().await.clone()
    }

    /// Split `[from, to]` against RPC retention, record and alert on the
    /// out-of-window part, and return the in-window part (if any).
    async fn apply_retention(&self, from: u64, to: u64) -> Result<Option<(u64, u64)>> {
        let health = self
            .rpc_client
            .check_health()
            .await
            .map_err(|e| anyhow::anyhow!("RPC getHealth failed before backfill: {e}"))?;
        let split = split_by_retention(from, to, health.oldest_ledger, health.latest_ledger);

        if let Some((start, end)) = split.out_of_window {
            let range = UnrecoverableRange {
                start,
                end,
                reason: UnrecoverableReason::UnrecoverableViaRpc,
                rpc_oldest_ledger: health.oldest_ledger,
                detected_at: Utc::now(),
            };
            crate::observability::metrics::record_backfill_unrecoverable_range();
            error!(
                alert = "backfill_unrecoverable_via_rpc",
                start,
                end,
                rpc_oldest_ledger = health.oldest_ledger,
                "Backfill range is outside the RPC retention window; not marking it covered"
            );

            let recovered = match &self.historical_source {
                Some(src) => match src.fetch_events(start, end, None).await {
                    Ok(events) => {
                        self.event_indexer
                            .index_events_with_checkpoint(events, end)
                            .await?;
                        info!(source = src.name(), start, end, "Recovered range from historical source");
                        true
                    }
                    Err(e) => {
                        warn!(source = src.name(), start, end, error = %e, "Historical source failed");
                        false
                    }
                },
                None => false,
            };

            if !recovered {
                self.unrecoverable.write().await.push(range.clone());
                self.state.write().await.unrecoverable_ranges.push(range);
            }
        }
        Ok(split.in_window)
    }

    /// The last ledger the indexer has durably processed (its persisted
    /// `last_processed_ledger` cursor), or 0 when there is no checkpoint yet.
    ///
    /// A read failure is logged and also reported as 0. The resume endpoint
    /// then sees a gap wider than the retention window and asks for a manual
    /// backfill instead of guessing a starting point.
    pub async fn last_ledger(&self) -> u32 {
        match self.event_indexer.get_last_processed_ledger().await {
            Ok(Some(ledger)) => u32::try_from(ledger).unwrap_or(u32::MAX),
            Ok(None) => 0,
            Err(e) => {
                warn!(error = %e, "Failed to read the indexer checkpoint");
                0
            }
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

        // Split against the RPC retention window (#2770). Anything older than
        // `oldestLedger` is flagged and alerted on, never walked via RPC.
        let mut req = req;
        match self.apply_retention(req.from_ledger, req.to_ledger).await {
            Ok(Some((from, to))) => {
                req.from_ledger = from;
                req.to_ledger = to;
            }
            Ok(None) => {
                let mut state = self.state.write().await;
                state.status = BackfillStatus::Completed;
                state.finished_at = Some(Utc::now());
                warn!("Entire backfill range is outside RPC retention; nothing to fetch");
                return Ok(());
            }
            Err(e) => {
                let mut state = self.state.write().await;
                state.status = BackfillStatus::Failed;
                state.error = Some(e.to_string());
                state.finished_at = Some(Utc::now());
                return Err(e);
            }
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
            let metrics = JobMetricsCollector::new("event-backfill");
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
                    metrics.complete_success();
                }
                Err(e) => {
                    state.status = BackfillStatus::Failed;
                    state.error = Some(e.to_string());
                    error!(error = %e, "Backfill failed");
                    metrics.complete_failure(&e.to_string());
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
                events_buffer.extend(events);
                chunk_ledgers += 1;

                if chunk_ledgers >= 1000 {
                    if let Err(e) = indexer
                        .index_events_with_checkpoint(
                            std::mem::take(&mut events_buffer),
                            ledger.sequence,
                        )
                        .await
                    {
                        warn!(ledger = ledger.sequence, error = %e, "Failed to index event chunk — aborting");
                        return Err(e);
                    }
                    chunk_ledgers = 0;
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

        // Commit any remaining events in the buffer for this gap.
        // `current` has already been advanced past the last processed ledger,
        // so use the state's `current_ledger` as the checkpoint target.
        if !events_buffer.is_empty() {
            let current_ledger = state_ref.read().await.current_ledger;
            if let Err(e) = indexer
                .index_events_with_checkpoint(std::mem::take(&mut events_buffer), current_ledger)
                .await
            {
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
        // Deterministic id: a retried page must map to the same row (#2769).
        id: format!("backfill-{}", ledger.sequence),
        contract_id,
        event_type: "LEDGER_CHECKPOINT".to_string(),
        epoch: None,
        hash: Some(ledger.hash.clone()),
        timestamp: Some(timestamp.timestamp() as u64),
        ledger: ledger.sequence,
        transaction_hash: ledger.hash.clone(),
        created_at: timestamp,
        verification_status: Some("backfilled".to_string()),
        op_index: 0,
        event_index: 0,
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
        let json = serde_json::to_string(&req).expect("BackfillRequest should serialize");
        assert!(json.contains("from_ledger"));
        assert!(json.contains("1000"));
    }

    #[tokio::test]
    async fn start_rejects_inverted_range() {
        let _guard = crate::lock_env_test();
        use crate::database::Database;
        use crate::db::schema::Schema;

        let pool = sqlx::SqlitePool::connect(":memory:")
            .await
            .expect("in-memory sqlite pool should connect");
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS)
            .execute(&pool)
            .await
            .expect("contract_events table creation should succeed");
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS_INDEXES)
            .execute(&pool)
            .await
            .expect("contract_events indexes creation should succeed");
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
        assert!(result
            .expect_err("start should reject an inverted range")
            .to_string()
            .contains("must be <="));
    }

    #[tokio::test]
    async fn start_rejects_oversized_range() {
        let _guard = crate::lock_env_test();
        use crate::database::Database;
        use crate::db::schema::Schema;

        let pool = sqlx::SqlitePool::connect(":memory:")
            .await
            .expect("in-memory sqlite pool should connect");
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS)
            .execute(&pool)
            .await
            .expect("contract_events table creation should succeed");
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS_INDEXES)
            .execute(&pool)
            .await
            .expect("contract_events indexes creation should succeed");
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
        assert!(result
            .expect_err("start should reject an oversized range")
            .to_string()
            .contains("exceeds maximum"));
    }

    #[tokio::test]
    async fn start_rejects_concurrent_run() {
        let _guard = crate::lock_env_test();
        use crate::database::Database;
        use crate::db::schema::Schema;

        let pool = sqlx::SqlitePool::connect(":memory:")
            .await
            .expect("in-memory sqlite pool should connect");
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS)
            .execute(&pool)
            .await
            .expect("contract_events table creation should succeed");
        sqlx::query(Schema::CREATE_CONTRACT_EVENTS_INDEXES)
            .execute(&pool)
            .await
            .expect("contract_events indexes creation should succeed");
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

    #[test]
    fn split_fully_in_window() {
        let s = split_by_retention(150, 180, 100, 200);
        assert_eq!(s.in_window, Some((150, 180)));
        assert_eq!(s.out_of_window, None);
    }

    #[test]
    fn split_straddles_oldest_ledger() {
        let s = split_by_retention(50, 150, 100, 200);
        assert_eq!(s.out_of_window, Some((50, 99)));
        assert_eq!(s.in_window, Some((100, 150)));
    }

    #[test]
    fn split_fully_out_of_window() {
        let s = split_by_retention(10, 20, 100, 200);
        assert_eq!(s.out_of_window, Some((10, 20)));
        assert_eq!(s.in_window, None);
    }

    #[test]
    fn split_clamps_future_ledgers_and_rejects_inverted() {
        let s = split_by_retention(190, 500, 100, 200);
        assert_eq!(s.in_window, Some((190, 200)));
        assert_eq!(split_by_retention(5, 1, 0, 10), RetentionSplit::default());
    }

    async fn job_with_db() -> (BackfillJob, Arc<crate::database::Database>) {
        use crate::database::Database;
        use crate::db::schema::Schema;
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await
            .expect("pool");
        for q in [
            Schema::CREATE_CONTRACT_EVENTS,
            Schema::CREATE_CONTRACT_EVENTS_INDEXES,
            Schema::CREATE_BACKFILL_STATE,
        ] {
            sqlx::query(q).execute(&pool).await.expect("schema");
        }
        let db = Arc::new(Database::new(pool));
        let indexer = Arc::new(EventIndexer::new(Arc::clone(&db)));
        let rpc = Arc::new(StellarRpcClient::new_with_defaults(true));
        let state = Arc::new(RwLock::new(BackfillState::default()));
        (BackfillJob::new(indexer, rpc, state), db)
    }

    #[tokio::test]
    async fn out_of_window_range_is_flagged_and_not_covered() {
        use crate::rpc::mock_stellar::{MOCK_LATEST_LEDGER, MOCK_OLDEST_LEDGER};
        let _guard = crate::lock_env_test();
        let (job, db) = job_with_db().await;

        // Mock RPC reports oldestLedger > requested start.
        let start = MOCK_OLDEST_LEDGER - 100;
        let in_window = job
            .apply_retention(start, MOCK_LATEST_LEDGER)
            .await
            .expect("retention");
        assert_eq!(in_window, Some((MOCK_OLDEST_LEDGER, MOCK_LATEST_LEDGER)));

        let flagged = job.unrecoverable_ranges().await;
        assert_eq!(flagged.len(), 1);
        assert_eq!(flagged[0].start, start);
        assert_eq!(flagged[0].end, MOCK_OLDEST_LEDGER - 1);
        assert_eq!(flagged[0].reason, UnrecoverableReason::UnrecoverableViaRpc);
        assert_eq!(job.status().await.unrecoverable_ranges.len(), 1);

        // Nothing was written for the out-of-window part.
        let covered: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contract_events WHERE ledger < ?")
            .bind(MOCK_OLDEST_LEDGER as i64)
            .fetch_one(db.pool())
            .await
            .expect("count");
        assert_eq!(covered, 0);
    }

    #[tokio::test]
    async fn in_window_range_is_not_flagged() {
        use crate::rpc::mock_stellar::{MOCK_LATEST_LEDGER, MOCK_OLDEST_LEDGER};
        let _guard = crate::lock_env_test();
        let (job, _db) = job_with_db().await;
        let in_window = job
            .apply_retention(MOCK_OLDEST_LEDGER, MOCK_LATEST_LEDGER)
            .await
            .expect("retention");
        assert_eq!(in_window, Some((MOCK_OLDEST_LEDGER, MOCK_LATEST_LEDGER)));
        assert!(job.unrecoverable_ranges().await.is_empty());
    }

    #[tokio::test]
    async fn failing_historical_source_keeps_range_flagged() {
        use crate::rpc::mock_stellar::MOCK_OLDEST_LEDGER;
        let _guard = crate::lock_env_test();
        let (job, _db) = job_with_db().await;
        let job = job.with_historical_source(Arc::new(GalexieS3Source {
            bucket_url: "s3://example/ledgers".into(),
        }));
        let in_window = job
            .apply_retention(1, MOCK_OLDEST_LEDGER - 1)
            .await
            .expect("retention");
        assert_eq!(in_window, None);
        assert_eq!(job.unrecoverable_ranges().await.len(), 1);
    }
}
