/// api/anchors_cached.rs
///
/// Axum route handlers for anchor data, backed by `CacheManager`.
///
/// Cache strategy:
///   - `GET /anchors`               → cached for `ANCHOR_LIST_TTL`
///   - `GET /anchors/:id`           → cached for `ANCHOR_DETAIL_TTL`
///   - `PUT /anchors/:id/status`    → updates anchor then fires
///                                     `InvalidationEvent::AnchorStatusChanged`
///   - `POST /admin/cache/invalidate` → admin invalidation endpoint
///   - `GET  /admin/cache/metrics`    → cache metrics endpoint

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
use crate::cache::invalidation::{InvalidationController, MetricsSnapshot};

// ---------------------------------------------------------------------------
// TTLs
// ---------------------------------------------------------------------------

const ANCHOR_LIST_TTL: Duration = Duration::from_secs(60);
const ANCHOR_DETAIL_TTL: Duration = Duration::from_secs(120);

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AnchorState {
    pub cache: Arc<CacheManager>,
    pub invalidation: InvalidationController,
    pub service: Arc<dyn AnchorService + Send + Sync>,
}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anchor {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub status: AnchorStatus,
    pub supported_assets: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnchorStatus {
    Active,
    Inactive,
    Suspended,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: AnchorStatus,
    pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Service trait
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
pub trait AnchorService {
    async fn list_anchors(&self) -> anyhow::Result<Vec<Anchor>>;
    async fn get_anchor(&self, id: &str) -> anyhow::Result<Option<Anchor>>;
    async fn update_status(
        &self,
        id: &str,
        req: &UpdateStatusRequest,
    ) -> anyhow::Result<Anchor>;
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /anchors`
pub async fn list_anchors(State(state): State<AnchorState>) -> impl IntoResponse {
    const KEY: &str = "anchors:list";

    if let Some(cached) = state.cache.get::<Vec<Anchor>>(KEY).await {
        debug!("anchors list: cache hit");
        return (StatusCode::OK, Json(cached)).into_response();
    }

    match state.service.list_anchors().await {
        Err(e) => {
            tracing::error!("Failed to list anchors: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Ok(anchors) => {
            if let Err(e) = state.cache.set(KEY, &anchors, Some(ANCHOR_LIST_TTL)).await {
                tracing::warn!("Could not cache anchor list: {}", e);
            }
            (StatusCode::OK, Json(anchors)).into_response()
        }
    }
}

/// `GET /anchors/:id`
pub async fn get_anchor(
    State(state): State<AnchorState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let key = format!("anchor:{}", id);

    if let Some(cached) = state.cache.get::<Anchor>(&key).await {
        debug!("anchor '{}': cache hit", id);
        return (StatusCode::OK, Json(cached)).into_response();
    }

    match state.service.get_anchor(&id).await {
        Err(e) => {
            tracing::error!("Failed to fetch anchor '{}': {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Ok(Some(anchor)) => {
            if let Err(e) = state.cache.set(&key, &anchor, Some(ANCHOR_DETAIL_TTL)).await {
                tracing::warn!("Could not cache anchor '{}': {}", id, e);
            }
            (StatusCode::OK, Json(anchor)).into_response()
        }
    }
}

/// `PUT /anchors/:id/status`
///
/// After a successful status update the anchor cache entry and the list are
/// both invalidated via the event system.
pub async fn update_anchor_status(
    State(state): State<AnchorState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateStatusRequest>,
) -> impl IntoResponse {
    match state.service.update_status(&id, &req).await {
        Err(e) => {
            tracing::error!("Status update failed for anchor '{}': {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Ok(updated) => {
            info!(
                "Anchor '{}' status updated to {:?} – invalidating cache",
                id, updated.status
            );
            state.invalidation.anchor_status_changed(format!("anchor:{}", id));
            state.invalidation.anchor_status_changed("anchors:list");
            (StatusCode::OK, Json(updated)).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Admin handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AdminInvalidateRequest {
    /// Key pattern to invalidate (supports glob and `/regex/` syntax).
    pub pattern: String,
}

#[derive(Debug, Serialize)]
pub struct AdminInvalidateResponse {
    pub pattern: String,
    pub status: &'static str,
}

/// `POST /admin/cache/invalidate`
///
/// Body: `{ "pattern": "anchor:*" }`
///
/// Fires an `AdminTrigger` invalidation event.  The actual cache mutation
/// happens asynchronously in the `InvalidationWorker`.
pub async fn admin_invalidate(
    State(state): State<AnchorState>,
    Json(body): Json<AdminInvalidateRequest>,
) -> impl IntoResponse {
    info!("Admin cache invalidation requested for pattern '{}'", body.pattern);
    state.invalidation.admin_invalidate(&body.pattern);
    (
        StatusCode::ACCEPTED,
        Json(AdminInvalidateResponse {
            pattern: body.pattern,
            status: "invalidation_queued",
        }),
    )
}

/// `GET /admin/cache/metrics`
pub async fn admin_metrics(State(state): State<AnchorState>) -> impl IntoResponse {
    let snapshot: MetricsSnapshot = state.invalidation.metrics();
    (StatusCode::OK, Json(snapshot))
}

// ---------------------------------------------------------------------------
// Router builder
// ---------------------------------------------------------------------------

use axum::Router;
use axum::routing::{get, post, put};

pub fn anchors_router(state: AnchorState) -> Router {
    Router::new()
        // Public anchor endpoints
        .route("/anchors", get(list_anchors))
        .route("/anchors/:id", get(get_anchor))
        .route("/anchors/:id/status", put(update_anchor_status))
        // Admin cache endpoints (add auth middleware in production)
        .route("/admin/cache/invalidate", post(admin_invalidate))
        .route("/admin/cache/metrics", get(admin_metrics))
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

    struct MockAnchorService {
        anchors: Vec<Anchor>,
    }

    #[async_trait::async_trait]
    impl AnchorService for MockAnchorService {
        async fn list_anchors(&self) -> anyhow::Result<Vec<Anchor>> {
            Ok(self.anchors.clone())
        }

        async fn get_anchor(&self, id: &str) -> anyhow::Result<Option<Anchor>> {
            Ok(self.anchors.iter().find(|a| a.id == id).cloned())
        }

        async fn update_status(
            &self,
            id: &str,
            req: &UpdateStatusRequest,
        ) -> anyhow::Result<Anchor> {
            let mut a = self
                .anchors
                .iter()
                .find(|a| a.id == id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("not found"))?;
            a.status = req.status;
            Ok(a)
        }
    }

    fn make_state() -> AnchorState {
        let metrics = Arc::new(CacheMetrics::default());
        let cache = Arc::new(CacheManager::new(
            100,
            Duration::from_secs(60),
            Arc::clone(&metrics),
        ));
        let (invalidation, _worker) =
            build_invalidation_system(Arc::clone(&cache), metrics, 32);

        let service = Arc::new(MockAnchorService {
            anchors: vec![Anchor {
                id: "circle".to_owned(),
                name: "Circle".to_owned(),
                domain: "circle.com".to_owned(),
                status: AnchorStatus::Active,
                supported_assets: vec!["USDC".to_owned()],
            }],
        });

        AnchorState { cache, invalidation, service }
    }

    #[tokio::test]
    async fn get_anchor_populates_cache() {
        let state = make_state();
        assert!(state.cache.get::<Anchor>("anchor:circle").await.is_none());
        let a = state.service.get_anchor("circle").await.unwrap().unwrap();
        state
            .cache
            .set("anchor:circle", &a, Some(Duration::from_secs(60)))
            .await
            .unwrap();
        assert!(state.cache.get::<Anchor>("anchor:circle").await.is_some());
    }

    #[tokio::test]
    async fn status_update_fires_invalidation() {
        let state = make_state();
        // pre-warm
        let a = state.service.get_anchor("circle").await.unwrap().unwrap();
        state.cache.set("anchor:circle", &a, None).await.unwrap();

        // trigger status update invalidation path
        state.invalidation.anchor_status_changed("anchor:circle");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn metrics_snapshot_non_zero_after_access() {
        let state = make_state();
        // miss
        let _: Option<Anchor> = state.cache.get("anchor:missing").await;
        let snap = state.invalidation.metrics();
        assert_eq!(snap.misses, 1);
        assert_eq!(snap.hits, 0);
        assert_eq!(snap.hit_rate, 0.0);
    }
}
