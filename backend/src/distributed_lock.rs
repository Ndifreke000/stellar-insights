//! Redis-backed distributed locks for coordinating work across replicas.
//!
//! Each lock is a Redis key holding a random owner token, set with `NX` and a
//! TTL. Release and extension are compare-and-set Lua scripts, so an instance
//! can never delete or extend a lock that expired and was taken by another
//! instance.
//!
//! # Fallback policy
//!
//! - `REDIS_URL` **unset**: single-instance mode. Locks always succeed
//!   ("fail open") so local development works without Redis.
//! - `REDIS_URL` **set** but Redis unreachable: locks fail ("fail closed"). Other
//!   replicas may be running, and silently running the job everywhere would
//!   duplicate work (double webhook deliveries, double backups).
//!
//! # Usage
//!
//! ```ignore
//! let lock = DistributedLock::shared().await;
//! if let Some(guard) = lock.try_acquire("job:corridor-refresh", Duration::from_secs(290)).await {
//!     run_job().await;
//!     guard.release().await;
//! }
//! ```

use redis::aio::ConnectionManager;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

/// Delete the key only if we still own it.
const RELEASE_SCRIPT: &str = r#"
if redis.call("GET", KEYS[1]) == ARGV[1] then
    return redis.call("DEL", KEYS[1])
else
    return 0
end
"#;

/// Extend the TTL only if we still own the key.
const EXTEND_SCRIPT: &str = r#"
if redis.call("GET", KEYS[1]) == ARGV[1] then
    return redis.call("PEXPIRE", KEYS[1], ARGV[2])
else
    return 0
end
"#;

/// Stable identifier for this process, used in lock tokens and logs.
///
/// Uses `POD_NAME` (set from the Downward API in Kubernetes) or `HOSTNAME`,
/// falling back to a random UUID.
#[must_use]
pub fn instance_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| {
        std::env::var("POD_NAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| Uuid::new_v4().to_string())
    })
}

enum Backend {
    /// No Redis configured: single-instance mode, every lock succeeds.
    SingleInstance,
    /// Redis configured. `client` is `None` if `REDIS_URL` is invalid. The
    /// connection is established lazily and retried on each use until it
    /// succeeds, so a Redis outage at startup doesn't disable locking forever.
    Redis {
        client: Option<redis::Client>,
        conn: tokio::sync::OnceCell<ConnectionManager>,
    },
}

pub struct DistributedLock {
    backend: Backend,
}

/// Proof of lock ownership. Call [`LockGuard::release`] when done; if dropped
/// without releasing, the lock simply expires after its TTL.
pub struct LockGuard {
    lock: Arc<DistributedLock>,
    key: String,
    /// `None` in single-instance mode (nothing to release).
    token: Option<String>,
}

impl DistributedLock {
    /// Process-wide shared instance (one Redis connection for all locks).
    pub async fn shared() -> Arc<Self> {
        static SHARED: tokio::sync::OnceCell<Arc<DistributedLock>> =
            tokio::sync::OnceCell::const_new();
        Arc::clone(SHARED.get_or_init(Self::from_env).await)
    }

    /// Build from `REDIS_URL`. Never fails; see the module docs for the fallback policy.
    pub async fn from_env() -> Arc<Self> {
        let Ok(url) = std::env::var("REDIS_URL") else {
            info!("DistributedLock: REDIS_URL not set, running in single-instance mode");
            return Arc::new(Self {
                backend: Backend::SingleInstance,
            });
        };
        let client = redis::Client::open(url.as_str())
            .map_err(|e| {
                warn!(
                    "DistributedLock: invalid REDIS_URL ({}); locks will fail closed",
                    e
                );
            })
            .ok();
        let lock = Arc::new(Self {
            backend: Backend::Redis {
                client,
                conn: tokio::sync::OnceCell::new(),
            },
        });
        // Connect eagerly so problems show up in startup logs.
        lock.connection().await;
        lock
    }

    /// Whether locks are coordinated through Redis (i.e. multi-instance safe).
    #[must_use]
    pub fn is_distributed(&self) -> bool {
        matches!(self.backend, Backend::Redis { .. })
    }

    /// The Redis connection, connecting on first use. `None` in single-instance
    /// mode or while Redis is unreachable (callers then fail closed).
    async fn connection(&self) -> Option<ConnectionManager> {
        let Backend::Redis { client, conn } = &self.backend else {
            return None;
        };
        let client = client.as_ref()?;
        // ConnectionManager transparently reconnects after the first success.
        conn.get_or_try_init(|| ConnectionManager::new(client.clone()))
            .await
            .map_err(|e| {
                warn!(
                    "DistributedLock: Redis connect failed ({}); locks will fail closed",
                    e
                );
            })
            .ok()
            .cloned()
    }

    /// Try to acquire `key` for `ttl`. Returns `None` if another instance holds
    /// it, or if Redis is configured but unavailable.
    pub async fn try_acquire(self: &Arc<Self>, key: &str, ttl: Duration) -> Option<LockGuard> {
        if matches!(self.backend, Backend::SingleInstance) {
            return Some(LockGuard {
                lock: Arc::clone(self),
                key: key.to_string(),
                token: None,
            });
        }
        let mut conn = self.connection().await?;

        let token = format!("{}:{}", instance_id(), Uuid::new_v4());
        let result: redis::RedisResult<Option<String>> = redis::cmd("SET")
            .arg(key)
            .arg(&token)
            .arg("NX")
            .arg("PX")
            .arg(ttl_millis(ttl))
            .query_async(&mut conn)
            .await;

        match result {
            Ok(Some(_)) => Some(LockGuard {
                lock: Arc::clone(self),
                key: key.to_string(),
                token: Some(token),
            }),
            Ok(None) => None,
            Err(e) => {
                warn!(key, "DistributedLock: SET NX failed ({}); not acquiring", e);
                None
            }
        }
    }

    async fn run_script(&self, script: &str, key: &str, token: &str, ttl: Option<Duration>) -> bool {
        if matches!(self.backend, Backend::SingleInstance) {
            return true;
        }
        let Some(mut conn) = self.connection().await else {
            return false;
        };
        let script = redis::Script::new(script);
        let mut invocation = script.key(key);
        invocation.arg(token);
        if let Some(ttl) = ttl {
            invocation.arg(ttl_millis(ttl));
        }
        match invocation.invoke_async::<i64>(&mut conn).await {
            Ok(n) => n == 1,
            Err(e) => {
                warn!(key, "DistributedLock: script failed ({})", e);
                false
            }
        }
    }
}

impl LockGuard {
    /// Extend the lock's TTL. Returns `false` if ownership was lost (the lock
    /// expired and possibly moved to another instance) — stop the work then.
    pub async fn extend(&self, ttl: Duration) -> bool {
        match &self.token {
            None => true,
            Some(token) => {
                self.lock
                    .run_script(EXTEND_SCRIPT, &self.key, token, Some(ttl))
                    .await
            }
        }
    }

    /// Release the lock if still owned.
    pub async fn release(self) {
        if let Some(token) = &self.token {
            self.lock
                .run_script(RELEASE_SCRIPT, &self.key, token, None)
                .await;
        }
    }

    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
}

fn ttl_millis(ttl: Duration) -> u64 {
    u64::try_from(ttl.as_millis()).unwrap_or(u64::MAX).max(1)
}
