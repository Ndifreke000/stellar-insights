//! Leader election integration tests (#2656).
//!
//! `leader_election.rs` exists and is wired into ingestion (`main.rs` uses
//! `spawn_leader_task` for the corridor performance monitor's role), but
//! there was no test actually running two instances against the same lock
//! and checking (a) only one of them is ever leader and (b) when the leader
//! disappears without a clean shutdown, the other takes over within one
//! lease TTL — which is what actually prevents double-ingestion across
//! replicas and is what the acceptance criteria ("failover within 30s") are
//! about.
//!
//! These tests require a real Redis instance at `REDIS_URL` (default
//! `redis://127.0.0.1:6379`), the same as `rate_limit_redis_integration.rs`,
//! since `DistributedLock`'s single-instance fallback (no `REDIS_URL`) makes
//! every instance "leader" unconditionally and can't exercise the mutual
//! exclusion this is meant to verify.
//!
//! Locally:
//! ```bash
//! REDIS_URL=redis://127.0.0.1:6379 cargo test --test leader_election_redis_integration -- --ignored
//! ```

use payraider_backend::distributed_lock::DistributedLock;
use payraider_backend::leader_election::LeaderElection;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn unique_suffix() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos}-{counter}")
}

/// Two independent `DistributedLock`s against the same Redis instance,
/// standing in for two separate replica processes.
async fn two_locks() -> (Arc<DistributedLock>, Arc<DistributedLock>) {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    std::env::set_var("REDIS_URL", &redis_url);

    let a = DistributedLock::from_env().await;
    let b = DistributedLock::from_env().await;
    assert!(
        a.is_distributed() && b.is_distributed(),
        "expected Redis at {redis_url}; start Redis or set REDIS_URL"
    );
    (a, b)
}

#[tokio::test]
#[ignore = "requires real Redis; run via leader-election-redis CI job"]
async fn only_one_replica_is_leader_at_a_time() {
    let (lock_a, lock_b) = two_locks().await;
    let role = format!("test-mutex-{}", unique_suffix());
    let ttl = Duration::from_secs(2);

    let election_a = LeaderElection::start(lock_a, &role, ttl);
    let election_b = LeaderElection::start(lock_b, &role, ttl);

    // Let both campaigns settle; the first SET NX wins the Redis key and the
    // other observes it's already held.
    tokio::time::sleep(Duration::from_millis(900)).await;

    let a_leader = election_a.is_leader();
    let b_leader = election_b.is_leader();
    assert_ne!(
        a_leader, b_leader,
        "exactly one of the two replicas should hold leadership for role {role:?}, got a={a_leader} b={b_leader}"
    );

    election_a.stop();
    election_b.stop();
}

#[tokio::test]
#[ignore = "requires real Redis; run via leader-election-redis CI job"]
async fn failover_happens_within_one_lease_ttl() {
    let (lock_a, lock_b) = two_locks().await;
    let role = format!("test-failover-{}", unique_suffix());
    let ttl = Duration::from_secs(2);

    let election_a = LeaderElection::start(lock_a, &role, ttl);
    let election_b = LeaderElection::start(lock_b, &role, ttl);

    tokio::time::sleep(Duration::from_millis(900)).await;

    let (leader, standby) = if election_a.is_leader() {
        (&election_a, &election_b)
    } else {
        (&election_b, &election_a)
    };
    assert!(leader.is_leader(), "setup: one side must be leader first");
    assert!(!standby.is_leader(), "setup: the other side must be standby");

    // Simulate the leader's process dying ungracefully (crash, OOM-kill,
    // network partition) rather than shutting down cleanly: `stop()` aborts
    // the campaigning task directly, which skips the `release()` call that
    // only runs when the task loop exits on its own. The lease is left to
    // expire on its TTL, exactly like a real crash.
    leader.stop();

    // The standby must pick up leadership within about one lease TTL (plus a
    // little slack for its own renewal tick) — this is the same bound the
    // issue's "failover within 30s" acceptance criterion imposes on
    // `DEFAULT_LEASE_TTL`, just scaled down so the test runs quickly.
    let deadline = Instant::now() + ttl + Duration::from_millis(1500);
    let mut took_over = false;
    while Instant::now() < deadline {
        if standby.is_leader() {
            took_over = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        took_over,
        "standby did not take over leadership within one lease TTL of the leader dying"
    );

    standby.stop();
}

#[tokio::test]
#[ignore = "requires real Redis; run via leader-election-redis CI job"]
async fn dropping_the_leader_also_fails_over_within_one_lease_ttl() {
    // `Drop for LeaderElection` aborts the campaigning task the same way
    // `stop()` does (see the module docs: "dropping the election stops
    // campaigning; a held lease expires after its TTL") — neither path
    // releases the lease early, so this should behave identically to the
    // explicit `stop()` case above and still land within one TTL.
    let (lock_a, lock_b) = two_locks().await;
    let role = format!("test-drop-failover-{}", unique_suffix());
    let ttl = Duration::from_secs(2);

    let election_a = LeaderElection::start(lock_a, &role, ttl);
    let election_b = LeaderElection::start(lock_b, &role, ttl);

    tokio::time::sleep(Duration::from_millis(900)).await;

    let (leader, standby) = if election_a.is_leader() {
        (election_a, election_b)
    } else {
        (election_b, election_a)
    };
    assert!(leader.is_leader());
    assert!(!standby.is_leader());

    drop(leader);

    let deadline = Instant::now() + ttl + Duration::from_millis(1500);
    let mut took_over = false;
    while Instant::now() < deadline {
        if standby.is_leader() {
            took_over = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        took_over,
        "standby did not take over leadership within one lease TTL of the leader being dropped"
    );

    standby.stop();
}
