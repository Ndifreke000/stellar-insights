//! Tests for cache invalidation logic and CacheInvalidationService.
//!
//! The CacheManager gracefully degrades when Redis is unavailable (returns
//! Ok(()) from all mutating methods), which allows unit-level testing without
//! a live Redis instance.

use std::sync::Arc;
use stellar_insights_backend::cache::{keys, CacheConfig, CacheManager, CacheStats};
use stellar_insights_backend::cache_invalidation::CacheInvalidationService;

// ── key-builder helpers ───────────────────────────────────────────────────────

#[test]
fn test_cache_key_anchor_list_format() {
    assert_eq!(keys::anchor_list(50, 0), "anchor:list:50:0");
    assert_eq!(keys::anchor_list(10, 100), "anchor:list:10:100");
    assert_eq!(keys::anchor_list(0, 0), "anchor:list:0:0");
}

#[test]
fn test_cache_key_anchor_detail_format() {
    assert_eq!(keys::anchor_detail("abc-123"), "anchor:detail:abc-123");
    assert_eq!(keys::anchor_detail(""), "anchor:detail:");
}

#[test]
fn test_cache_key_anchor_by_account_format() {
    assert_eq!(
        keys::anchor_by_account("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN"),
        "anchor:account:GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN"
    );
}

#[test]
fn test_cache_key_anchor_assets_format() {
    assert_eq!(keys::anchor_assets("anchor-99"), "anchor:assets:anchor-99");
}

#[test]
fn test_cache_key_corridor_list_format() {
    assert_eq!(
        keys::corridor_list(20, 40, "active"),
        "corridor:list:20:40:active"
    );
}

#[test]
fn test_cache_key_corridor_detail_format() {
    assert_eq!(
        keys::corridor_detail("USDC->XLM"),
        "corridor:detail:USDC->XLM"
    );
}

#[test]
fn test_cache_key_dashboard_stats_format() {
    assert_eq!(keys::dashboard_stats(), "dashboard:stats");
}

#[test]
fn test_cache_key_metrics_overview_format() {
    assert_eq!(keys::metrics_overview(), "metrics:overview");
}

#[test]
fn test_cache_key_patterns() {
    assert_eq!(keys::anchor_pattern(), "anchor:*");
    assert_eq!(keys::corridor_pattern(), "corridor:*");
    assert_eq!(keys::dashboard_pattern(), "dashboard:*");
}

// ── CacheConfig TTL logic ─────────────────────────────────────────────────────

#[test]
fn test_cache_config_default_ttls() {
    let config = CacheConfig::default();
    assert_eq!(config.corridor_metrics_ttl, 300); // 5 minutes
    assert_eq!(config.anchor_data_ttl, 600); // 10 minutes
    assert_eq!(config.dashboard_stats_ttl, 60); // 1 minute
}

#[test]
fn test_cache_config_get_ttl_known_keys() {
    let config = CacheConfig::default();
    assert_eq!(config.get_ttl("corridor"), 300);
    assert_eq!(config.get_ttl("anchor"), 600);
    assert_eq!(config.get_ttl("dashboard"), 60);
}

#[test]
fn test_cache_config_get_ttl_unknown_key_returns_default() {
    let config = CacheConfig::default();
    assert_eq!(config.get_ttl("unknown"), 300);
    assert_eq!(config.get_ttl(""), 300);
}

// ── CacheStats hit-rate computation ──────────────────────────────────────────

#[test]
fn test_cache_stats_hit_rate_zero_when_empty() {
    let stats = CacheStats {
        hits: 0,
        misses: 0,
        invalidations: 0,
    };
    assert_eq!(stats.hit_rate(), 0.0);
}

#[test]
fn test_cache_stats_hit_rate_100_percent() {
    let stats = CacheStats {
        hits: 200,
        misses: 0,
        invalidations: 0,
    };
    assert_eq!(stats.hit_rate(), 100.0);
}

#[test]
fn test_cache_stats_hit_rate_50_percent() {
    let stats = CacheStats {
        hits: 50,
        misses: 50,
        invalidations: 10,
    };
    assert_eq!(stats.hit_rate(), 50.0);
}

#[test]
fn test_cache_stats_hit_rate_80_percent() {
    let stats = CacheStats {
        hits: 80,
        misses: 20,
        invalidations: 5,
    };
    assert_eq!(stats.hit_rate(), 80.0);
}

#[test]
fn test_cache_stats_hit_rate_0_percent_no_hits() {
    let stats = CacheStats {
        hits: 0,
        misses: 100,
        invalidations: 3,
    };
    assert_eq!(stats.hit_rate(), 0.0);
}

// ── CacheInvalidationService (no-Redis, graceful-degradation path) ───────────

/// Create a CacheManager that has no Redis connection (offline / test mode).
/// Because the pool is not initialised, every operation returns Ok(()) without
/// actually touching Redis – which is fine for unit tests focused on the
/// service's control-flow rather than Redis commands.
async fn make_offline_cache() -> Arc<CacheManager> {
    // Point at a deliberately unreachable Redis URL so the connection fails and
    // CacheManager initialises in offline/disconnected mode.
    std::env::set_var("REDIS_URL", "redis://127.0.0.1:1"); // port 1 is never open
    let manager = CacheManager::new(CacheConfig::default())
        .await
        .expect("CacheManager::new must not fail even if Redis is unreachable");
    Arc::new(manager)
}

#[tokio::test]
async fn test_invalidate_anchors_succeeds_without_redis() {
    let cache = make_offline_cache().await;
    let svc = CacheInvalidationService::new(Arc::clone(&cache));
    svc.invalidate_anchors().await.expect("should succeed");
}

#[tokio::test]
async fn test_invalidate_anchor_succeeds_without_redis() {
    let cache = make_offline_cache().await;
    let svc = CacheInvalidationService::new(Arc::clone(&cache));
    svc.invalidate_anchor("anchor-001")
        .await
        .expect("should succeed");
}

#[tokio::test]
async fn test_invalidate_anchor_by_account_succeeds_without_redis() {
    let cache = make_offline_cache().await;
    let svc = CacheInvalidationService::new(Arc::clone(&cache));
    svc.invalidate_anchor_by_account("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN")
        .await
        .expect("should succeed");
}

#[tokio::test]
async fn test_invalidate_corridors_succeeds_without_redis() {
    let cache = make_offline_cache().await;
    let svc = CacheInvalidationService::new(Arc::clone(&cache));
    svc.invalidate_corridors().await.expect("should succeed");
}

#[tokio::test]
async fn test_invalidate_corridor_succeeds_without_redis() {
    let cache = make_offline_cache().await;
    let svc = CacheInvalidationService::new(Arc::clone(&cache));
    svc.invalidate_corridor("USDC:issuer->XLM:native")
        .await
        .expect("should succeed");
}

#[tokio::test]
async fn test_invalidate_dashboard_succeeds_without_redis() {
    let cache = make_offline_cache().await;
    let svc = CacheInvalidationService::new(Arc::clone(&cache));
    svc.invalidate_dashboard().await.expect("should succeed");
}

#[tokio::test]
async fn test_invalidate_metrics_succeeds_without_redis() {
    let cache = make_offline_cache().await;
    let svc = CacheInvalidationService::new(Arc::clone(&cache));
    svc.invalidate_metrics().await.expect("should succeed");
}

#[tokio::test]
async fn test_invalidate_all_succeeds_without_redis() {
    let cache = make_offline_cache().await;
    let svc = CacheInvalidationService::new(Arc::clone(&cache));
    svc.invalidate_all().await.expect("should succeed");
}

// ── CacheManager stat tracking ────────────────────────────────────────────────

#[tokio::test]
async fn test_cache_manager_initial_stats_are_zero() {
    let cache = make_offline_cache().await;
    let stats = cache.get_stats();
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.invalidations, 0);
}

#[tokio::test]
async fn test_cache_manager_get_increments_miss_when_offline() {
    let cache = make_offline_cache().await;
    let _: Option<String> = cache.get("some-key").await.expect("should not error");
    let stats = cache.get_stats();
    // When Redis is unavailable, get() counts as a cache miss
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hits, 0);
}

#[tokio::test]
async fn test_cache_manager_set_does_not_error_when_offline() {
    let cache = make_offline_cache().await;
    // set() should silently succeed even with no Redis
    cache
        .set("corridor:detail:USDC->XLM", &"test_value", 300)
        .await
        .expect("should not error when offline");
}

#[tokio::test]
async fn test_cache_manager_delete_does_not_error_when_offline() {
    let cache = make_offline_cache().await;
    cache
        .delete("anchor:detail:xyz")
        .await
        .expect("should not error when offline");
}

#[tokio::test]
async fn test_cache_manager_reset_stats() {
    let cache = make_offline_cache().await;
    // Trigger some misses
    let _: Option<String> = cache.get("k1").await.unwrap();
    let _: Option<String> = cache.get("k2").await.unwrap();
    assert_eq!(cache.get_stats().misses, 2);

    cache.reset_stats();
    let stats = cache.get_stats();
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.invalidations, 0);
}

#[tokio::test]
async fn test_cache_manager_multiple_gets_accumulate_misses() {
    let cache = make_offline_cache().await;
    for i in 0..5u32 {
        let _: Option<String> = cache.get(&format!("key-{i}")).await.unwrap();
    }
    assert_eq!(cache.get_stats().misses, 5);
}
