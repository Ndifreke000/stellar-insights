//! Redis-backed rate limiter integration tests (#1869).
//!
//! These tests require a real Redis instance at `REDIS_URL` (default
//! `redis://127.0.0.1:6379`). They are run explicitly in CI via the
//! `rate-limit-redis` workflow with a Redis service container.
//!
//! Locally:
//! ```bash
//! REDIS_URL=redis://127.0.0.1:6379 cargo test --test rate_limit_redis_integration -- --ignored
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use stellar_insights_backend::rate_limit::{
    ClientIdentifier, ClientRateLimits, ClientTier, RateLimitConfig, RateLimiter,
};

fn unique_suffix() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos}-{counter}")
}

async fn redis_limiter() -> RateLimiter {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    std::env::set_var("REDIS_URL", &redis_url);

    let limiter = RateLimiter::new()
        .await
        .expect("rate limiter should construct");
    assert!(
        limiter.is_using_redis().await,
        "expected Redis at {redis_url}; start Redis or set REDIS_URL"
    );
    limiter
}

fn memory_only_limiter() -> RateLimiter {
    RateLimiter::new_memory_only(None)
}

async fn assert_limit_behavior(limiter: &RateLimiter, endpoint: &str, ip: &str, limit: u32) {
    limiter
        .register_endpoint(
            endpoint.to_string(),
            RateLimitConfig {
                requests_per_minute: limit,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: limit,
                    premium: limit,
                    anonymous: limit,
                }),
            },
        )
        .await;

    let client = ClientIdentifier::IpAddress(ip.to_string());

    for _ in 0..limit {
        let (allowed, info) = limiter
            .check_rate_limit_for_client(&client, endpoint, ip)
            .await;
        assert!(allowed, "request within limit should be allowed");
        assert!(info.remaining < limit);
    }

    let (allowed, info) = limiter
        .check_rate_limit_for_client(&client, endpoint, ip)
        .await;
    assert!(!allowed, "request over limit should be denied");
    assert_eq!(info.remaining, 0);
}

#[tokio::test]
#[ignore = "requires real Redis; run via rate-limit-redis CI job"]
async fn redis_and_memory_agree_on_limit_reached() {
    let suffix = unique_suffix();
    let limit = 3u32;

    let redis = redis_limiter().await;
    let memory = memory_only_limiter();

    assert_limit_behavior(
        &redis,
        &format!("/redis-limit-{suffix}"),
        &format!("10.0.0.{}", (suffix.len() % 200) + 1),
        limit,
    )
    .await;
    assert_limit_behavior(
        &memory,
        &format!("/mem-limit-{suffix}"),
        &format!("10.0.1.{}", (suffix.len() % 200) + 1),
        limit,
    )
    .await;
}

#[tokio::test]
#[ignore = "requires real Redis; run via rate-limit-redis CI job"]
async fn redis_and_memory_agree_on_independent_per_key() {
    let suffix = unique_suffix();
    let limit = 2u32;

    let redis = redis_limiter().await;
    let memory = memory_only_limiter();

    for (label, limiter) in [("redis", &redis), ("memory", &memory)] {
        let key_a = format!("{label}-key-a-{suffix}");
        let key_b = format!("{label}-key-b-{suffix}");

        for _ in 0..limit {
            let (allowed, _) = limiter.rate_limit_api_key(&key_a, limit).await;
            assert!(allowed, "{label}: key_a within limit");
        }
        let (allowed, info) = limiter.rate_limit_api_key(&key_a, limit).await;
        assert!(!allowed, "{label}: key_a should be exhausted");
        assert_eq!(info.remaining, 0);

        let (allowed, _) = limiter.rate_limit_api_key(&key_b, limit).await;
        assert!(allowed, "{label}: key_b must be independent of key_a");
    }
}

#[tokio::test]
#[ignore = "requires real Redis; run via rate-limit-redis CI job"]
async fn redis_resets_after_window_expiry() {
    let suffix = unique_suffix();
    let limit = 2u32;
    let limiter = redis_limiter().await;
    let api_key = format!("reset-key-{suffix}");

    for _ in 0..limit {
        let (allowed, _) = limiter.rate_limit_api_key(&api_key, limit).await;
        assert!(allowed);
    }
    let (allowed, _) = limiter.rate_limit_api_key(&api_key, limit).await;
    assert!(!allowed, "should be blocked before window reset");

    // Simulate window expiry by deleting the Redis key (same effect as TTL elapsing).
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let client = redis::Client::open(redis_url).expect("valid redis url");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("redis connection");
    let key = format!("apikey_ratelimit:{api_key}");
    redis::cmd("DEL")
        .arg(&key)
        .query_async::<()>(&mut conn)
        .await
        .expect("del key");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let (allowed, info) = limiter.rate_limit_api_key(&api_key, limit).await;
    assert!(allowed, "should allow again after window reset");
    assert!(info.remaining < limit);
}

#[tokio::test]
#[ignore = "requires real Redis; run via rate-limit-redis CI job"]
async fn redis_client_tier_helpers_still_work() {
    assert_eq!(
        ClientIdentifier::ApiKey("k".into()).tier(),
        ClientTier::Authenticated
    );
    assert_eq!(
        ClientIdentifier::IpAddress("1.2.3.4".into()).tier(),
        ClientTier::Anonymous
    );
}
