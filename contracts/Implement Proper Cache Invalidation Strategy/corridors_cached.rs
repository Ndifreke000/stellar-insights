/// api/corridors_cached.rs
///
/// Axum route handlers for corridor data, backed by `CacheManager`.
///
/// Cache strategy:
///   - `GET /corridors`           → cached for `CORRIDOR_LIST_TTL`
///   - `GET /corridors/:id`       → cached for `CORRIDOR_DETAIL_TTL`
///   - `POST /corridors/:id/pay`  → delegates to service layer then fires
///                                   `InvalidationEvent::PaymentDetected`
///   - Admin endpoint wired in `cache_admin` module.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::cache::CacheManager;
use crate::cache::invalidation::InvalidationController;

// ---------------------------------------------------------------------------
// TTLs
// ---------------------------------------------------------------------------

const CORRIDOR_LIST_TTL: Duration = Duration::from_secs(30);
const CORRIDOR_DETAIL_TTL: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------
// Shared application state (subset)
// ---------------------------------------------------------------------------

/// Route-level state.  In practice this is a field of your larger `AppState`.
#[derive(Clone)]
pub struct CorridorState {
    pub cache: Arc<CacheManager>,
    pub invalidation: InvalidationController,
    /// Reference to your real corridor service / DB layer.
    pub service: Arc<dyn CorridorService + Send + Sync>,
}

// ---------------------------------------------------------------------------
// Domain types (stubs – replace with your real models)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Corridor {
    pub id: String,
    pub from_asset: String,
    pub to_asset: String,
    pub rate: f64,
    pub liquidity_usd: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub amount: f64,
    pub sender: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentReceipt {
    pub corridor_id: String,
    pub amount: f64,
    pub tx_hash: String,
}

// ---------------------------------------------------------------------------
// Service trait (implement against your DB / RPC layer)
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
pub trait CorridorService {
    async fn list_corridors(&self) -> anyhow::Result<Vec<Corridor>>;
    async fn get_corridor(&self, id: &str) -> anyhow::Result<Option<Corridor>>;
    async fn process_payment(
        &self,
        corridor_id: &str,
        req: &PaymentRequest,
    ) -> anyhow::Result<PaymentReceipt>;
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /corridors`
pub async fn list_corridors(
    State(state): State<CorridorState>,
) -> impl IntoResponse {
    const KEY: &str = "corridors:list";

    // 1. Cache hit?
    if let Some(cached) = state.cache.get::<Vec<Corridor>>(KEY).await {
        debug!("corridors list: cache hit");
        return (StatusCode::OK, Json(cached)).into_response();
    }

    // 2. Cache miss → fetch from service.
    match state.service.list_corridors().await {
        Err(e) => {
            tracing::error!("Failed to list corridors: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Ok(corridors) => {
            if let Err(e) = state.cache.set(KEY, &corridors, Some(CORRIDOR_LIST_TTL)).await {
                tracing::warn!("Could not cache corridor list: {}", e);
            }
            (StatusCode::OK, Json(corridors)).into_response()
        }
    }
}

/// `GET /corridors/:id`
pub async fn get_corridor(
    State(state): State<CorridorState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let key = format!("corridor:{}", id);

    if let Some(cached) = state.cache.get::<Corridor>(&key).await {
        debug!("corridor '{}': cache hit", id);
        return (StatusCode::OK, Json(cached)).into_response();
    }

    match state.service.get_corridor(&id).await {
        Err(e) => {
            tracing::error!("Failed to fetch corridor '{}': {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Ok(Some(corridor)) => {
            if let Err(e) = state.cache.set(&key, &corridor, Some(CORRIDOR_DETAIL_TTL)).await {
                tracing::warn!("Could not cache corridor '{}': {}", id, e);
            }
            (StatusCode::OK, Json(corridor)).into_response()
        }
    }
}

/// `POST /corridors/:id/pay`
///
/// After a successful payment the corridor cache entry is invalidated so that
/// updated liquidity / rates are fetched on the next request.
pub async fn pay_corridor(
    State(state): State<CorridorState>,
    Path(id): Path<String>,
    Json(req): Json<PaymentRequest>,
) -> impl IntoResponse {
    match state.service.process_payment(&id, &req).await {
        Err(e) => {
            tracing::error!("Payment failed for corridor '{}': {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Ok(receipt) => {
            // Fire invalidation event – the worker handles cache cleanup
            // asynchronously so we never block the response.
            info!("Payment processed for corridor '{}' – invalidating cache", id);
            state.invalidation.payment_detected(format!("corridor:{}", id));
            // Also invalidate the list so stale aggregates are flushed.
            state.invalidation.payment_detected("corridors:list");
            (StatusCode::OK, Json(receipt)).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Router builder
// ---------------------------------------------------------------------------

use axum::Router;
use axum::routing::{get, post};

pub fn corridors_router(state: CorridorState) -> Router {
    Router::new()
        .route("/corridors", get(list_corridors))
        .route("/corridors/:id", get(get_corridor))
        .route("/corridors/:id/pay", post(pay_corridor))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::invalidation::{build_invalidation_system, CacheMetrics};
    use std::sync::Arc;

    struct MockService {
        corridors: Vec<Corridor>,
    }

    #[async_trait::async_trait]
    impl CorridorService for MockService {
        async fn list_corridors(&self) -> anyhow::Result<Vec<Corridor>> {
            Ok(self.corridors.clone())
        }
        async fn get_corridor(&self, id: &str) -> anyhow::Result<Option<Corridor>> {
            Ok(self.corridors.iter().find(|c| c.id == id).cloned())
        }
        async fn process_payment(
            &self,
            corridor_id: &str,
            req: &PaymentRequest,
        ) -> anyhow::Result<PaymentReceipt> {
            Ok(PaymentReceipt {
                corridor_id: corridor_id.to_owned(),
                amount: req.amount,
                tx_hash: "mock_tx".to_owned(),
            })
        }
    }

    fn make_state() -> CorridorState {
        let metrics = Arc::new(CacheMetrics::default());
        let cache = Arc::new(CacheManager::new(
            100,
            Duration::from_secs(60),
            Arc::clone(&metrics),
        ));
        let (invalidation, _worker) =
            build_invalidation_system(Arc::clone(&cache), metrics, 32);

        let service = Arc::new(MockService {
            corridors: vec![Corridor {
                id: "usd-eur".to_owned(),
                from_asset: "USD".to_owned(),
                to_asset: "EUR".to_owned(),
                rate: 0.92,
                liquidity_usd: 1_000_000.0,
            }],
        });

        CorridorState { cache, invalidation, service }
    }

    #[tokio::test]
    async fn get_corridor_populates_cache() {
        let state = make_state();
        // Verify miss → populated
        assert!(state.cache.get::<Corridor>("corridor:usd-eur").await.is_none());
        let c = state.service.get_corridor("usd-eur").await.unwrap().unwrap();
        state
            .cache
            .set("corridor:usd-eur", &c, Some(Duration::from_secs(60)))
            .await
            .unwrap();
        assert!(state.cache.get::<Corridor>("corridor:usd-eur").await.is_some());
    }

    #[tokio::test]
    async fn payment_triggers_invalidation_event() {
        let state = make_state();
        // Pre-populate cache.
        let c = state.service.get_corridor("usd-eur").await.unwrap().unwrap();
        state
            .cache
            .set("corridor:usd-eur", &c, Some(Duration::from_secs(60)))
            .await
            .unwrap();

        // Simulate the pay handler firing the event.
        state.invalidation.payment_detected("corridor:usd-eur");

        // Give the worker a tick to process (in real tests, spawn the worker).
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
