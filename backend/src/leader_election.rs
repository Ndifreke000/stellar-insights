//! Leader election for singleton background tasks.
//!
//! Some work must run on exactly one replica at a time: long-running loops
//! (the webhook dispatcher) and scheduled jobs (backups). Every replica runs a
//! [`LeaderElection`] for a given role; the one holding the Redis lock is the
//! leader and runs the task. The leader renews its lease every `ttl / 3`. If
//! it dies, the lease expires and another replica takes over within about one
//! TTL.
//!
//! In single-instance mode (no `REDIS_URL`) every process is the leader.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::distributed_lock::{instance_id, DistributedLock, LockGuard};

/// Default lease length. Failover takes at most about this long.
pub const DEFAULT_LEASE_TTL: Duration = Duration::from_secs(30);

pub struct LeaderElection {
    role: String,
    is_leader: watch::Receiver<bool>,
    handle: JoinHandle<()>,
}

impl LeaderElection {
    /// Start campaigning for `role` (lock key `leader:<role>`).
    #[must_use]
    pub fn start(lock: Arc<DistributedLock>, role: &str, ttl: Duration) -> Self {
        let (tx, rx) = watch::channel(false);
        let key = format!("leader:{role}");
        let role_name = role.to_string();
        let renew_every = (ttl / 3).max(Duration::from_millis(500));

        let handle = tokio::spawn(async move {
            let mut lease: Option<LockGuard> = None;
            let mut ticker = tokio::time::interval(renew_every);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            loop {
                ticker.tick().await;

                lease = match lease.take() {
                    Some(guard) => {
                        if guard.extend(ttl).await {
                            Some(guard)
                        } else {
                            warn!(role = %role_name, instance = instance_id(), "Lost leadership");
                            None
                        }
                    }
                    None => {
                        let acquired = lock.try_acquire(&key, ttl).await;
                        if acquired.is_some() {
                            info!(role = %role_name, instance = instance_id(), "Acquired leadership");
                        }
                        acquired
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
