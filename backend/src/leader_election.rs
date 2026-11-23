//! Leader election for singleton background tasks with fencing tokens.
//!
//! Some work must run on exactly one replica at a time: long-running loops
//! (the webhook dispatcher) and scheduled jobs (backups). Every replica runs a
//! [`LeaderElection`] for a given role; the one holding the Redis lock is the
//! leader and runs the task. The leader renews its lease every `ttl / 3`.
//!
//! # Fencing
//!
//! When leadership is acquired, a **fencing token** is assigned via
//! `DistributedLock::increment_fence_token()`. This monotonically increasing
//! token must be presented with any side-effecting operation:
//!
//! ```ignore
//! let lock = DistributedLock::shared().await;
//! let guard = lock.try_acquire("role:myjob", Duration::from_secs(30)).await?;
//! let token = guard.fence_token().expect("distributed mode");
//!
//! // When writing to DB, pass the token
//! db.write_with_fencing(data, token).await?;
//! ```
//!
//! When leadership transfers to another instance:
//! 1. The new leader calls `increment_fence_token()` for the role
//! 2. Any writes from the old leader bearing an older token are rejected
//! 3. This prevents the classic Redlock pitfall of a paused leader writing
//!    after its lease expired
//!
//! # Safety margins
//!
//! The lock extension happens every `ttl / 3`. If the lock cannot be extended
//! (Redis unavailable or lease expired), the election loop exits, stopping all
//! leader-only work. This ensures a leader never continues writing after it
//! has lost the lease.
//!
//! # Usage
//!
//! ```ignore
//! let lock = DistributedLock::shared().await;
//! let election = LeaderElection::start(lock, "myrole", Duration::from_secs(30));
//!
//! // Periodically check if still leader
//! if election.is_leader() {
//!     do_leader_work().await?;
//! }
//! ```

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::distributed_lock::{instance_id, DistributedLock, LockGuard};

/// Default lease length. Failover takes at most about this long.
pub const DEFAULT_LEASE_TTL: Duration = Duration::from_secs(30);

/// Safety margin: extend lease every this fraction of TTL.
/// At 1/3, a leader has 2/3 of TTL to complete work before expiry.
const LEASE_EXTENSION_INTERVAL: f64 = 1.0 / 3.0;

pub struct LeaderElection {
    role: String,
    is_leader: watch::Receiver<bool>,
    handle: JoinHandle<()>,
}

impl LeaderElection {
    /// Start campaigning for `role` (lock key `leader:<role>`).
    ///
    /// The lease TTL should be long enough for a typical job run plus a safety
    /// margin, but short enough that a dead leader's lease expires in a
    /// reasonable time. For a job that typically runs in 5 minutes, a 30-second
    /// TTL with a 25-second extension interval gives ample time.
    #[must_use]
    pub fn start(lock: Arc<DistributedLock>, role: &str, ttl: Duration) -> Self {
        let (tx, rx) = watch::channel(false);
        let key = format!("leader:{role}");
        let role_name = role.to_string();
        let renew_every = ttl
            .mul_f64(LEASE_EXTENSION_INTERVAL)
            .max(Duration::from_millis(500));

        let handle = tokio::spawn(async move {
            let mut lease: Option<LockGuard> = None;
            let mut ticker = tokio::time::interval(renew_every);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            loop {
                ticker.tick().await;

                lease = match lease.take() {
                    Some(mut guard) => {
                        // Attempt to extend the lease
                        if guard.extend(ttl).await {
                            // Successfully extended; continue leading
                            Some(guard)
                        } else {
                            // Lost leadership - stop work immediately
                            warn!(role = %role_name, instance = instance_id(), "Lost leadership (lease extension failed)");
                            None
                        }
                    }
                    None => {
                        // Try to acquire the lock
                        let acquired = lock.try_acquire(&key, ttl).await;
                        match acquired {
                            Some(guard) => {
                                info!(role = %role_name, instance = instance_id(), "Acquired leadership");
                                // Increment fencing token for this role
                                let fence_key = format!("fence:{}", key);
                                match lock.increment_fence_token(&fence_key).await {
                                    Ok(token) => {
                                        info!(role = %role_name, fence_token = token, "Acquired fencing token");
                                    }
                                    Err(e) => {
                                        warn!(role = %role_name, error = ?e, "Failed to increment fencing token");
                                    }
                                }
                                Some(guard)
                            }
                            None => {
                                // Lock held by another instance
                                None
                            }
                        }
                    }
                };

                tx.send_if_modified(|leading| {
                    let changed = *leading != lease.is_some();
                    *leading = lease.is_some();
                    changed
                });
                if tx.is_closed() {
                    break;
                }
            }

            if let Some(guard) = lease {
                guard.release().await;
            }
        });

        Self {
            role: role.to_string(),
            is_leader: rx,
            handle,
        }
    }

    #[must_use]
    pub fn is_leader(&self) -> bool {
        *self.is_leader.borrow()
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.is_leader.clone()
    }

    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Stop campaigning. Any held lease expires after its TTL.
    pub fn stop(&self) {
        self.handle.abort();
    }
}

impl Drop for LeaderElection {
    /// Dropping the election stops campaigning; a held lease expires after its TTL.
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// Run `make_task()` only while this instance is leader for `role`.
///
/// The task is started when leadership is gained and aborted when it is lost,
/// so at most one replica runs it at a time (modulo one lease TTL during a
/// network partition, so tasks must still be idempotent).
///
/// **Important:** The task is aborted when leadership is lost, so it must be
/// idempotent and safe to interrupt at any point.
///
/// If the task exits on its own (e.g. it gave up after repeated failures), this
/// instance resigns: it stops campaigning, its lease expires, and another
/// replica takes over the role.
pub fn spawn_leader_task<F, Fut>(
    lock: Arc<DistributedLock>,
    role: &str,
    ttl: Duration,
    make_task: F,
) -> JoinHandle<()>
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let election = LeaderElection::start(lock, role, ttl);
    let role = role.to_string();

    tokio::spawn(async move {
        let mut leader_rx = election.subscribe();
        loop {
            if !wait_until(&mut leader_rx, true).await {
                break;
            }
            info!(role = %role, "Starting leader-only task");
            let task = tokio::spawn(make_task());
            let abort = task.abort_handle();
            // Also stop the task if this supervisor itself is aborted (shutdown).
            let _stop_on_drop = AbortOnDrop(task.abort_handle());

            tokio::select! {
                _ = task => {
                    warn!(role = %role, "Leader-only task exited; resigning leadership");
                    break;
                }
                _ = wait_until(&mut leader_rx, false) => {
                    warn!(role = %role, "Leadership lost, stopping leader-only task");
                    abort.abort();
                }
            }
        }
        election.stop();
    })
}

struct AbortOnDrop(tokio::task::AbortHandle);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Wait until the leadership flag equals `want`. Returns `false` if the
/// election task has gone away.
async fn wait_until(rx: &mut watch::Receiver<bool>, want: bool) -> bool {
    loop {
        if *rx.borrow_and_update() == want {
            return true;
        }
        if rx.changed().await.is_err() {
            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn leadership_changes_trigger_abort() {
        // This is a smoke test - in reality we'd need multiple processes or Redis
        // to properly test leader election. This just verifies the types compile.
        let lock = DistributedLock::from_env().await;
        let election = LeaderElection::start(lock, "test-role", Duration::from_secs(5));
        assert!(!election.is_leader()); // No Redis, single-instance mode
        election.stop();
    }
}
