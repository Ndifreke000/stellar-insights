/// Admin cache management endpoints.
///
/// Provides an authenticated HTTP surface for operators to:
/// * View cache metrics
/// * Invalidate by pattern
/// * Flush the entire cache
/// * Trigger a manual warm-up

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::cache::{CacheInvalidationEvent, CacheManager, CacheMetrics};

// ────────────────────────────────────────────────────────────────
// Shared admin state (generic over value type V)
// ────────────────────────────────────────────────────────────────

/// Thin wrapper so Axum can extract the right cache from state.
#[derive(Clone)]
pub struct AdminCacheState<V: Clone + Send + Sync + 'static> {
    pub cache: Arc<CacheManager<V>>,
}

// ────────────────────────────────────────────────────────────────
// Request / response shapes
// ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InvalidatePatternQuery {
    /// Substring pattern.  All keys containing this string will be evicted.
    pub pattern: String,
}

#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub hits: u64,
    pub misses: u64,
    pub invalidations: u64,
    pub evictions: u64,
    pub warm_ups: u64,
    pub current_size: usize,
    pub hit_rate: f64,
}

impl From<CacheMetrics> for MetricsResponse {
    fn from(m: CacheMetrics) -> Self {
        let hit_rate = m.hit_rate();
        Self {
            hits: m.hits,
            misses: m.misses,
            invalidations: m.invalidations,
            evictions: m.evictions,
            warm_ups: m.warm_ups,
            current_size: m.current_size,
            hit_rate,
        }
    }
}

// ────────────────────────────────────────────────────────────────
// Handlers
// ────────────────────────────────────────────────────────────────

/// GET /admin/cache/metrics
pub async fn get_metrics<V: Clone + Send + Sync + 'static>(
    State(state): State<AdminCacheState<V>>,
) -> impl IntoResponse {
    let metrics: MetricsResponse = state.cache.metrics().await.into();
    (StatusCode::OK, Json(metrics))
}

/// DELETE /admin/cache/invalidate?pattern=<pattern>
pub async fn invalidate_by_pattern<V: Clone + Send + Sync + 'static>(
    Query(q): Query<InvalidatePatternQuery>,
    State(state): State<AdminCacheState<V>>,
) -> impl IntoResponse {
    state
        .cache
        .publish_event(CacheInvalidationEvent::AdminInvalidate {
            pattern: q.pattern.clone(),
        });
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "queued",
            "pattern": q.pattern
        })),
    )
}

/// DELETE /admin/cache/flush
pub async fn flush_cache<V: Clone + Send + Sync + 'static>(
    State(state): State<AdminCacheState<V>>,
) -> impl IntoResponse {
    state.cache.flush().await;
    (StatusCode::OK, Json(serde_json::json!({ "status": "flushed" })))
}

/// POST /admin/cache/evict-lru?target=<n>
#[derive(Debug, Deserialize)]
pub struct EvictLruQuery {
    /// Evict until the cache holds at most `target` entries.
    pub target: usize,
}

pub async fn evict_lru<V: Clone + Send + Sync + 'static>(
    Query(q): Query<EvictLruQuery>,
    State(state): State<AdminCacheState<V>>,
) -> impl IntoResponse {
    state
        .cache
        .publish_event(CacheInvalidationEvent::MemoryPressure {
            target_size: q.target,
        });
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "queued", "target_size": q.target })),
    )
}

// ────────────────────────────────────────────────────────────────
// Router
// ────────────────────────────────────────────────────────────────

pub fn admin_cache_router<V: Clone + Send + Sync + 'static>(
    state: AdminCacheState<V>,
) -> Router {
    Router::new()
        .route("/admin/cache/metrics", get(get_metrics::<V>))
        .route("/admin/cache/invalidate", delete(invalidate_by_pattern::<V>))
        .route("/admin/cache/flush", delete(flush_cache::<V>))
        .route("/admin/cache/evict-lru", post(evict_lru::<V>))
        .with_state(state)
}
