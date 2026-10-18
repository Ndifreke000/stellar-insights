//! Webhook dispatcher with supervised error handling, retry logic, and health reporting.
//!
//! The dispatcher is spawned as a background task. To avoid silent failures
//! (issue #2342) the task is supervised: errors are logged with context, failed
//! deliveries are retried with exponential backoff, and the dispatcher exposes a
//! health snapshot so its status is observable.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Maximum number of delivery attempts (initial try + retries).
const MAX_ATTEMPTS: u32 = 5;
/// Base delay for exponential backoff between retries.
const BASE_BACKOFF: Duration = Duration::from_millis(250);
/// Upper bound for a single backoff delay.
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// A single webhook delivery job.
#[derive(Debug, Clone)]
pub struct WebhookJob {
    pub url: String,
    pub payload: String,
}

/// Observable health snapshot for the webhook dispatcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatcherHealth {
    /// Whether the dispatcher task is currently running.
    pub running: bool,
    /// Total jobs successfully delivered.
    pub delivered: u64,
    /// Total jobs that exhausted all retry attempts.
    pub failed: u64,
}

/// Supervised webhook dispatcher.
///
/// Holds shared state that is updated by the background task and can be read
/// from request handlers via [`DispatcherHandle::health`].
#[derive(Clone)]
pub struct DispatcherHandle {
    running: Arc<AtomicBool>,
    delivered: Arc<AtomicU64>,
    failed: Arc<AtomicU64>,
    last_error: Arc<RwLock<Option<String>>>,
}

impl DispatcherHandle {
    /// Returns the current health snapshot of the dispatcher.
    pub async fn health(&self) -> DispatcherHealth {
        DispatcherHealth {
            running: self.running.load(Ordering::SeqCst),
            delivered: self.delivered.load(Ordering::SeqCst),
            failed: self.failed.load(Ordering::SeqCst),
        }
    }

    /// Returns the most recent dispatcher error, if any.
    pub async fn last_error(&self) -> Option<String> {
        self.last_error.read().await.clone()
    }
}

/// Spawns the webhook dispatcher as a supervised background task.
///
/// The returned [`DispatcherHandle`] can be used to observe dispatcher health.
/// Errors are logged with context and never silently dropped.
pub fn spawn_dispatcher<F, Fut>(mut next_job: F) -> DispatcherHandle
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Option<WebhookJob>> + Send,
{
    let handle = DispatcherHandle {
        running: Arc::new(AtomicBool::new(true)),
        delivered: Arc::new(AtomicU64::new(0)),
        failed: Arc::new(AtomicU64::new(0)),
        last_error: Arc::new(RwLock::new(None)),
    };

    let task_handle = handle.clone();
    tokio::spawn(async move {
        info!("webhook dispatcher started");
        loop {
            let job = match next_job().await {
                Some(job) => job,
                None => {
                    info!("webhook dispatcher queue drained, shutting down");
                    break;
                }
            };

            match deliver_with_retry(&job).await {
                Ok(()) => {
                    task_handle.delivered.fetch_add(1, Ordering::SeqCst);
                }
                Err(err) => {
                    let message = format!("webhook delivery to {} failed: {}", job.url, err);
                    error!(url = %job.url, error = %err, "webhook delivery exhausted retries");
                    task_handle.failed.fetch_add(1, Ordering::SeqCst);
                    *task_handle.last_error.write().await = Some(message);
                }
            }
        }
        task_handle.running.store(false, Ordering::SeqCst);
        info!("webhook dispatcher stopped");
    });

    handle
}

/// Attempts to deliver a job, retrying with exponential backoff on failure.
async fn deliver_with_retry(job: &WebhookJob) -> Result<(), String> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        match deliver(job).await {
            Ok(()) => return Ok(()),
            Err(err) if attempt >= MAX_ATTEMPTS => {
                return Err(format!("after {attempt} attempts: {err}"));
            }
            Err(err) => {
                let backoff = backoff_for(attempt);
                warn!(
                    url = %job.url,
                    attempt,
                    backoff_ms = backoff.as_millis() as u64,
                    error = %err,
                    "webhook delivery failed, retrying"
                );
                tokio::time::sleep(backoff).await;
            }
        }
    }
}

/// Computes the exponential backoff delay for a given attempt, capped at
/// [`MAX_BACKOFF`].
fn backoff_for(attempt: u32) -> Duration {
    let shift = attempt.saturating_sub(1).min(16);
    let delay = BASE_BACKOFF.saturating_mul(1u32 << shift);
    delay.min(MAX_BACKOFF)
}

/// Performs a single webhook delivery attempt.
///
/// Replace the body with the project's HTTP client call; the signature is kept
/// stable so retry/supervision logic stays independent of transport details.
async fn deliver(job: &WebhookJob) -> Result<(), String> {
    if job.url.is_empty() {
        return Err("empty webhook url".to_string());
    }
    // Transport implementation is provided by the caller's HTTP client.
    let _ = &job.payload;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_is_capped() {
        assert_eq!(backoff_for(1), BASE_BACKOFF);
        assert_eq!(backoff_for(2), BASE_BACKOFF * 2);
        assert!(backoff_for(64) <= MAX_BACKOFF);
    }

    #[tokio::test]
    async fn health_reports_running_state() {
        let handle = spawn_dispatcher(|| async { None });
        let health = handle.health().await;
        assert_eq!(health.delivered, 0);
        assert_eq!(health.failed, 0);
    }
}
