//! Graceful shutdown handling for the backend server
//!
//! This module provides utilities for handling shutdown signals (SIGTERM, SIGINT)
//! and coordinating graceful shutdown of server components.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tracing::{info, warn};

/// Cheaply-clonable, level-triggered shutdown signal.
///
/// Unlike `broadcast::Receiver` (edge-triggered — a receiver that subscribes
/// after `trigger_shutdown()` has already fired will never see it), this can
/// be polled at any time from any loop to ask "has shutdown started yet?".
///
/// Long-running ingestion loops (e.g. `ingestion::ledger::LedgerIngestionService`,
/// which fetches and persists ledgers one at a time) hold a clone and check
/// `is_set()` between units of work — specifically *before* starting the next
/// ledger, never mid-ledger — so a SIGTERM stops ingestion at a ledger
/// boundary instead of leaving a partially-written ledger behind (#2655).
#[derive(Clone, Default)]
pub struct ShutdownFlag(Arc<AtomicBool>);

impl ShutdownFlag {
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Marks shutdown as started. Idempotent.
    pub fn set(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether shutdown has been requested.
    #[must_use]
    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Ordered shutdown phases.
///
/// Shutdown proceeds strictly in this order so that each phase can rely on
/// the previous one having completed (or timed out):
///
/// 1. `StopAccepting` — stop taking new HTTP connections / work.
/// 2. `DrainHttp` — let in-flight HTTP requests finish.
/// 3. `CloseWebSockets` — close WS clients with code 1012 (Service Restart).
/// 4. `FinishBatch` — let the in-flight ingestion batch commit or roll back.
/// 5. `FlushOutbox` — flush any pending outbox entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownPhase {
    StopAccepting,
    DrainHttp,
    CloseWebSockets,
    FinishBatch,
    FlushOutbox,
}

impl ShutdownPhase {
    /// All phases in execution order.
    pub const ALL: [ShutdownPhase; 5] = [
        ShutdownPhase::StopAccepting,
        ShutdownPhase::DrainHttp,
        ShutdownPhase::CloseWebSockets,
        ShutdownPhase::FinishBatch,
        ShutdownPhase::FlushOutbox,
    ];

    /// Human-readable label used in logs.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            ShutdownPhase::StopAccepting => "stop-accepting",
            ShutdownPhase::DrainHttp => "drain-http",
            ShutdownPhase::CloseWebSockets => "close-websockets",
            ShutdownPhase::FinishBatch => "finish-batch",
            ShutdownPhase::FlushOutbox => "flush-outbox",
        }
    }
}

/// WebSocket close code used when the server is restarting (RFC 6455 §7.4.1).
pub const WS_CLOSE_SERVICE_RESTART: u16 = 1012;

/// Configuration for graceful shutdown behavior
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Maximum time to wait for in-flight requests to complete
    pub graceful_timeout: Duration,
    /// Maximum time to wait for background tasks to complete
    pub background_task_timeout: Duration,
    /// Maximum time to wait for database connections to close
    pub db_close_timeout: Duration,
    /// Maximum time to wait for WebSocket clients to acknowledge close
    pub websocket_close_timeout: Duration,
    /// Maximum time to wait for the in-flight ingestion batch to finish
    pub batch_finish_timeout: Duration,
    /// Maximum time to wait for the outbox to flush
    pub outbox_flush_timeout: Duration,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            graceful_timeout: Duration::from_secs(30),
            background_task_timeout: Duration::from_secs(10),
            db_close_timeout: Duration::from_secs(5),
            websocket_close_timeout: Duration::from_secs(5),
            batch_finish_timeout: Duration::from_secs(15),
            outbox_flush_timeout: Duration::from_secs(10),
        }
    }
}

impl ShutdownConfig {
    /// Create a new shutdown configuration from environment variables
    pub fn from_env() -> Self {
        let graceful_timeout = std::env::var("SHUTDOWN_GRACEFUL_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .map_or(Duration::from_secs(30), Duration::from_secs);

        let background_task_timeout = std::env::var("SHUTDOWN_BACKGROUND_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .map_or(Duration::from_secs(10), Duration::from_secs);

        let db_close_timeout = std::env::var("SHUTDOWN_DB_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .map_or(Duration::from_secs(5), Duration::from_secs);

        let websocket_close_timeout = std::env::var("SHUTDOWN_WS_CLOSE_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .map_or(Duration::from_secs(5), Duration::from_secs);

        let batch_finish_timeout = std::env::var("SHUTDOWN_BATCH_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .map_or(Duration::from_secs(15), Duration::from_secs);

        let outbox_flush_timeout = std::env::var("SHUTDOWN_OUTBOX_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .map_or(Duration::from_secs(10), Duration::from_secs);

        Self {
            graceful_timeout,
            background_task_timeout,
            db_close_timeout,
            websocket_close_timeout,
            batch_finish_timeout,
            outbox_flush_timeout,
        }
    }

    /// Deadline for a given shutdown phase.
    #[must_use]
    pub const fn phase_timeout(&self, phase: ShutdownPhase) -> Duration {
        match phase {
            ShutdownPhase::StopAccepting => self.graceful_timeout,
            ShutdownPhase::DrainHttp => self.graceful_timeout,
            ShutdownPhase::CloseWebSockets => self.websocket_close_timeout,
            ShutdownPhase::FinishBatch => self.batch_finish_timeout,
            ShutdownPhase::FlushOutbox => self.outbox_flush_timeout,
        }
    }
}

/// Shutdown coordinator that manages the graceful shutdown process
pub struct ShutdownCoordinator {
    config: ShutdownConfig,
    shutdown_tx: broadcast::Sender<()>,
    shutdown_flag: ShutdownFlag,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator with the given configuration
    #[must_use]
    pub fn new(config: ShutdownConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            config,
            shutdown_tx,
            shutdown_flag: ShutdownFlag::new(),
        }
    }

    /// Get a receiver for shutdown notifications
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Get a cheaply-clonable, level-triggered handle for the shutdown
    /// state. Hand a clone to any long-running loop (e.g. ledger ingestion)
    /// that needs to check "has shutdown started?" between units of work
    /// rather than reacting to a one-shot broadcast.
    #[must_use]
    pub fn shutdown_flag(&self) -> ShutdownFlag {
        self.shutdown_flag.clone()
    }

    /// Trigger shutdown and notify all subscribers
    pub fn trigger_shutdown(&self) {
        info!("Triggering graceful shutdown");
        self.shutdown_flag.set();
        let _ = self.shutdown_tx.send(());
    }

    /// Get the graceful timeout duration
    #[must_use]
    pub const fn graceful_timeout(&self) -> Duration {
        self.config.graceful_timeout
    }

    /// Get the background task timeout duration
    #[must_use]
    pub const fn background_task_timeout(&self) -> Duration {
        self.config.background_task_timeout
    }

    /// Get the database close timeout duration
    #[must_use]
    pub const fn db_close_timeout(&self) -> Duration {
        self.config.db_close_timeout
    }

    /// Get the configured deadline for a specific shutdown phase.
    #[must_use]
    pub const fn phase_timeout(&self, phase: ShutdownPhase) -> Duration {
        self.config.phase_timeout(phase)
    }

    /// Run a single shutdown phase with its configured deadline.
    ///
    /// The supplied future is awaited up to the phase deadline. If it does not
    /// complete in time the phase is logged as timed out and `false` is
    /// returned; otherwise `true`. This never panics, so a stuck phase cannot
    /// prevent the remaining phases from running.
    pub async fn run_phase<F>(&self, phase: ShutdownPhase, fut: F) -> bool
    where
        F: std::future::Future<Output = ()>,
    {
        let deadline = self.phase_timeout(phase);
        info!(phase = phase.label(), ?deadline, "Starting shutdown phase");
        match timeout(deadline, fut).await {
            Ok(()) => {
                info!(phase = phase.label(), "Shutdown phase completed");
                true
            }
            Err(_) => {
                warn!(
                    phase = phase.label(),
                    ?deadline,
                    "Shutdown phase timed out; continuing"
                );
                false
            }
        }
    }

    /// Run all shutdown phases in order, each bounded by its own deadline.
    ///
    /// `phases` supplies the future for each phase. Phases run strictly in
    /// `ShutdownPhase::ALL` order; a phase that times out does not abort the
    /// remaining phases. Returns the list of phases that completed in time.
    pub async fn run_ordered<F, Fut>(&self, mut phases: F) -> Vec<ShutdownPhase>
    where
        F: FnMut(ShutdownPhase) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let mut completed = Vec::new();
        for phase in ShutdownPhase::ALL {
            if self.run_phase(phase, phases(phase)).await {
                completed.push(phase);
            }
        }
        completed
    }
}

/// Wait for shutdown signals (SIGTERM, SIGINT/Ctrl+C)
///
/// This function will block until either SIGTERM or SIGINT is received.
/// On Windows, only Ctrl+C is supported.
pub async fn wait_for_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM signal");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT signal");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C signal");
    }
}

/// Shutdown signal future suitable for use with `axum::serve().with_graceful_shutdown()`.
///
/// Resolves when SIGTERM or SIGINT (Ctrl+C) is received. Errors installing
/// signal handlers are logged as warnings rather than panicking, so the server
/// continues to run and can still be stopped via other means.
pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let sigterm = signal(SignalKind::terminate());
        let sigint = signal(SignalKind::interrupt());

        match (sigterm, sigint) {
            (Ok(mut term), Ok(mut int)) => {
                tokio::select! {
                    _ = term.recv() => info!("Received SIGTERM, shutting down"),
                    _ = int.recv()  => info!("Received SIGINT, shutting down"),
                }
            }
            (Err(e), _) | (_, Err(e)) => {
                warn!(
                    "Failed to install signal handler: {}; falling back to Ctrl+C",
                    e
                );
                let _ = tokio::signal::ctrl_c().await;
                info!("Received Ctrl+C, shutting down");
            }
        }
    }

    #[cfg(not(unix))]
    {
        match tokio::signal::ctrl_c().await {
            Ok(()) => info!("Received Ctrl+C, shutting down"),
            Err(e) => warn!("Failed to listen for Ctrl+C: {}", e),
        }
    }
}

/// Gracefully shutdown background tasks
///
/// Waits for the given task handles to complete within the timeout period.
/// If the timeout is exceeded, tasks are forcefully aborted.
pub async fn shutdown_background_tasks(
    tasks: Vec<tokio::task::JoinHandle<()>>,
    timeout_duration: Duration,
) {
    info!("Shutting down {} background tasks", tasks.len());

    let shutdown_future = async {
        for (idx, task) in tasks.into_iter().enumerate() {
            match task.await {
                Ok(()) => info!("Background task {} completed successfully", idx),
                Err(e) if e.is_cancelled() => {
                    info!("Background task {} was cancelled", idx);
                }
                Err(e) => warn!("Background task {} failed: {}", idx, e),
            }
        }
    };

    if timeout(timeout_duration, shutdown_future).await.is_err() {
        warn!(
            "Background tasks did not complete within {:?}, forcing shutdown",
            timeout_duration
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn fast_config() -> ShutdownConfig {
        ShutdownConfig {
            graceful_timeout: Duration::from_millis(200),
            background_task_timeout: Duration::from_millis(200),
            db_close_timeout: Duration::from_millis(200),
            websocket_close_timeout: Duration::from_millis(200),
            batch_finish_timeout: Duration::from_millis(200),
            outbox_flush_timeout: Duration::from_millis(200),
        }
    }

    #[test]
    fn phases_are_ordered() {
        assert_eq!(
            ShutdownPhase::ALL,
            [
                ShutdownPhase::StopAccepting,
                ShutdownPhase::DrainHttp,
                ShutdownPhase::CloseWebSockets,
                ShutdownPhase::FinishBatch,
                ShutdownPhase::FlushOutbox,
            ]
        );
    }

    #[test]
    fn ws_close_code_is_service_restart() {
        assert_eq!(WS_CLOSE_SERVICE_RESTART, 1012);
    }

    #[tokio::test]
    async fn run_phase_completes_within_deadline() {
        let coord = ShutdownCoordinator::new(fast_config());
        let done = coord
            .run_phase(ShutdownPhase::DrainHttp, async {})
            .await;
        assert!(done);
    }

    #[tokio::test]
    async fn run_phase_times_out_but_does_not_panic() {
        let coord = ShutdownCoordinator::new(fast_config());
        let done = coord
            .run_phase(ShutdownPhase::FinishBatch, async {
                tokio::time::sleep(Duration::from_secs(5)).await;
            })
            .await;
        assert!(!done);
    }

    #[tokio::test]
    async fn run_ordered_executes_every_phase_in_order() {
        let coord = ShutdownCoordinator::new(fast_config());
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen_clone = Arc::clone(&seen);

        let completed = coord
            .run_ordered(|phase| {
                let seen = Arc::clone(&seen_clone);
                async move {
                    seen.lock().unwrap().push(phase);
                }
            })
            .await;

        assert_eq!(completed, ShutdownPhase::ALL.to_vec());
        assert_eq!(*seen.lock().unwrap(), ShutdownPhase::ALL.to_vec());
    }

    #[tokio::test]
    async fn run_ordered_continues_after_timeout() {
        let coord = ShutdownCoordinator::new(fast_config());
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&count);

        let completed = coord
            .run_ordered(|phase| {
                let count = Arc::clone(&count_clone);
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    if phase == ShutdownPhase::FinishBatch {
                        // Simulate a stuck batch that exceeds its deadline.
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            })
            .await;

        // Every phase was attempted even though FinishBatch timed out.
        assert_eq!(count.load(Ordering::SeqCst), ShutdownPhase::ALL.len());
        assert!(!completed.contains(&ShutdownPhase::FinishBatch));
        assert!(completed.contains(&ShutdownPhase::FlushOutbox));
    }

    #[tokio::test]
    async fn sigterm_during_batch_finishes_batch_phase() {
        // Simulates SIGTERM arriving while a batch is in flight: the batch
        // future is allowed to complete within its deadline, so the batch is
        // committed/rolled back cleanly rather than being dropped.
        let coord = ShutdownCoordinator::new(fast_config());
        let committed = Arc::new(AtomicBool::new(false));
        let committed_clone = Arc::clone(&committed);

        coord.trigger_shutdown();
        assert!(coord.shutdown_flag().is_set());

        let done = coord
            .run_phase(ShutdownPhase::FinishBatch, async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                committed_clone.store(true, Ordering::SeqCst);
            })
            .await;

        assert!(done);
        assert!(committed.load(Ordering::SeqCst));
    }
}
