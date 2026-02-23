/// Cached corridor API handlers.
///
/// Wraps corridor data fetches with the shared `CacheManager`, using
/// event-driven invalidation so that stale corridor data is evicted the
/// moment a new payment is detected.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::cache::{CacheInvalidationEvent, CacheManager, CacheMetrics, DEFAULT_TTL};

// ────────────────────────────────────────────────────────────────
// Domain types (stubs – replace with your real models)
// ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorData {
    pub id: String,
    pub from_asset: String,
    pub to_asset: String,
    pub rate: f64,
    pub fee_bps: u32,
}

// ────────────────────────────────────────────────────────────────
// App state
// ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CorridorState {
    pub cache: Arc<CacheManager<CorridorData>>,
    // pub db: Arc<Database>,
    // pub rpc: Arc<StellarRpcClient>,
}

fn cache_key(corridor_id: &str) -> String {
    format!("corridor:{}", corridor_id)
}

// ────────────────────────────────────────────────────────────────
// Handlers
// ────────────────────────────────────────────────────────────────

/// GET /corridors/:id
/// Returns corridor data, serving from cache when available.
pub async fn get_corridor(
    Path(id): Path<String>,
    State(state): State<CorridorState>,
) -> impl IntoResponse {
    let key = cache_key(&id);

    if let Some(cached) = state.cache.get(&key).await {
        info!("Cache HIT for corridor '{}'", id);
        return (StatusCode::OK, Json(cached)).into_response();
    }

    info!("Cache MISS for corridor '{}' – fetching from source", id);

    // ── Replace this stub with a real DB/RPC call ──────────────
    let data = CorridorData {
        id: id.clone(),
        from_asset: "USDC".into(),
        to_asset: "XLM".into(),
        rate: 1.0,
        fee_bps: 30,
    };
    // ───────────────────────────────────────────────────────────

    state.cache.set(key, data.clone(), DEFAULT_TTL).await;

    (StatusCode::OK, Json(data)).into_response()
}

/// POST /corridors/:id/payment
/// Simulates a payment event and triggers cache invalidation for the corridor.
pub async fn on_payment_detected(
    Path(id): Path<String>,
    State(state): State<CorridorState>,
) -> impl IntoResponse {
    info!("Payment detected for corridor '{}' – invalidating cache", id);
    state
        .cache
        .publish_event(CacheInvalidationEvent::PaymentDetected {
            corridor_id: id.clone(),
        });
    (StatusCode::OK, Json(serde_json::json!({ "invalidated": true, "corridor": id })))
}

// ────────────────────────────────────────────────────────────────
// Cache warming
// ────────────────────────────────────────────────────────────────

/// Preloads the top corridors into the cache on application startup.
///
/// In production, replace the stub vector with a real DB query:
/// ```rust
/// let top_corridors = db.get_top_corridors(10).await?;
/// ```
pub async fn warm_corridor_cache(cache: &CacheManager<CorridorData>) {
    let top_corridors: Vec<CorridorData> = vec![
        CorridorData {
            id: "usdc-xlm".into(),
            from_asset: "USDC".into(),
            to_asset: "XLM".into(),
            rate: 1.0,
            fee_bps: 30,
        },
        CorridorData {
            id: "xlm-usdc".into(),
            from_asset: "XLM".into(),
            to_asset: "USDC".into(),
            rate: 0.99,
            fee_bps: 30,
        },
    ];

    let mut warmed = 0usize;
    for corridor in top_corridors {
        let key = cache_key(&corridor.id);
        cache.set(key, corridor, DEFAULT_TTL).await;
        warmed += 1;
    }
    info!("Corridor cache warmed with {} entries", warmed);

    // Bump warm_ups metric
    let mut m = cache.metrics().await;
    // (metrics are read-only snapshots; the mutable counter lives inside the
    //  CacheManager.  In a real app you would expose an increment method.)
}

// ────────────────────────────────────────────────────────────────
// Router
// ────────────────────────────────────────────────────────────────

pub fn corridor_router(state: CorridorState) -> Router {
    Router::new()
        .route("/corridors/:id", get(get_corridor))
        .route("/corridors/:id/payment", post(on_payment_detected))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_cache_hit() {
        let cache = Arc::new(CacheManager::new(100));
        let data = CorridorData {
            id: "usdc-xlm".into(),
            from_asset: "USDC".into(),
            to_asset: "XLM".into(),
            rate: 1.0,
            fee_bps: 30,
        };
        cache
            .set("corridor:usdc-xlm", data.clone(), DEFAULT_TTL)
            .await;
        let result = cache.get("corridor:usdc-xlm").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "usdc-xlm");
    }

    #[tokio::test]
    async fn test_payment_invalidates_corridor() {
        let cache = Arc::new(CacheManager::new(100));
        let data = CorridorData {
            id: "usdc-xlm".into(),
            from_asset: "USDC".into(),
            to_asset: "XLM".into(),
            rate: 1.0,
            fee_bps: 30,
        };
        cache.set("corridor:usdc-xlm", data, DEFAULT_TTL).await;
        cache.publish_event(CacheInvalidationEvent::PaymentDetected {
            corridor_id: "usdc-xlm".to_string(),
        });
        // Allow background task to process
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(cache.get("corridor:usdc-xlm").await.is_none());
    }

    #[tokio::test]
    async fn test_warming() {
        let cache = Arc::new(CacheManager::new(100));
        warm_corridor_cache(&cache).await;
        assert!(cache.get("corridor:usdc-xlm").await.is_some());
        assert!(cache.get("corridor:xlm-usdc").await.is_some());
    }
}
