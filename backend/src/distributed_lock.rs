/// Distributed lock using Redis SETNX with TTL.
///
/// Acquires a lock by setting a key with NX (only if not exists) and a TTL.
/// Returns `true` if the lock was acquired, `false` if another instance holds it.
/// The lock is released by deleting the key.
///
/// Usage in job scheduler:
/// ```ignore
/// if DistributedLock::try_acquire(&redis_url, "job:corridor-refresh", 290).await {
///     run_job().await;
/// }
/// ```
pub struct DistributedLock;

impl DistributedLock {
    /// Try to acquire a lock named `key` for `ttl_seconds`.
    /// Returns `true` if acquired (this instance should run the job).
    /// Returns `false` if another instance already holds the lock.
    /// Falls back to `true` (always run) if Redis is unavailable, preserving
    /// existing single-instance behaviour.
    pub async fn try_acquire(redis_url: &str, key: &str, ttl_seconds: u64) -> bool {
        let client = match redis::Client::open(redis_url) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("DistributedLock: Redis unavailable ({}), running job anyway", e);
                return true;
            }
        };
        let mut conn = match client.get_multiplexed_tokio_connection().await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("DistributedLock: Redis connect failed ({}), running job anyway", e);
                return true;
            }
        };

        let result: redis::RedisResult<bool> = redis::cmd("SET")
            .arg(key)
            .arg(1u8)
            .arg("NX")
            .arg("EX")
            .arg(ttl_seconds)
            .query_async(&mut conn)
            .await;

        match result {
            Ok(acquired) => acquired,
            Err(e) => {
                tracing::warn!("DistributedLock: SET NX failed ({}), running job anyway", e);
                true
            }
        }
    }
}
