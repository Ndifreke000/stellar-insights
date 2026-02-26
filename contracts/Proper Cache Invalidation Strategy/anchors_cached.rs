/// Cached anchor API handlers.
///
/// Anchor data is invalidated whenever an `AnchorStatusChanged` event is
/// published (e.g. when the anchor's sep-10 or sep-12 status changes).

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::cache::{CacheInvalidationEvent, CacheManager, DEFAULT_TTL};

// ────────────────────────────────────────────────────────────────
// Domain types (stubs – replace with your real models)
// ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorInfo {
    pub id: String,
    pub name: String,
    pub home_domain: String,
    pub sep_10: bool,
    pub sep_31: bool,
    pub status: AnchorStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnchorStatus {
    Active,
    Degraded,
    Offline,
}

// ────────────────────────────────────────────────────────────────
// App state
// ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AnchorState {
    pub cache: Arc<CacheManager<AnchorInfo>>,
    // pub db: Arc<Database>,
}

fn cache_key(anchor_id: &str) -> String {
    format!("anchor:{}", anchor_id)
}

// ────────────────────────────────────────────────────────────────
// Handlers
// ────────────────────────────────────────────────────────────────

/// GET /anchors/:id
pub async fn get_anchor(
    Path(id): Path<String>,
    State(state): State<AnchorState>,
) -> impl IntoResponse {
    let key = cache_key(&id);

    if let Some(cached) = state.cache.get(&key).await {
        info!("Cache HIT for anchor '{}'", id);
        return (StatusCode::OK, Json(cached)).into_response();
    }

    info!("Cache MISS for anchor '{}' – fetching from source", id);

    // ── Replace with real DB/SEP lookup ────────────────────────
    let data = AnchorInfo {
        id: id.clone(),
        name: format!("Anchor {}", id),
        home_domain: format!("{}.example.com", id),
        sep_10: true,
        sep_31: true,
        status: AnchorStatus::Active,
    };
    // ───────────────────────────────────────────────────────────

    state.cache.set(key, data.clone(), DEFAULT_TTL).await;

    (StatusCode::OK, Json(data)).into_response()
}

/// POST /anchors/:id/status-change
/// Called internally when an anchor's status changes.
pub async fn on_anchor_status_change(
    Path(id): Path<String>,
    State(state): State<AnchorState>,
) -> impl IntoResponse {
    info!("Anchor '{}' status changed – invalidating cache", id);
    state
        .cache
        .publish_event(CacheInvalidationEvent::AnchorStatusChanged {
            anchor_id: id.clone(),
        });
    (
        StatusCode::OK,
        Json(serde_json::json!({ "invalidated": true, "anchor": id })),
    )
}

// ────────────────────────────────────────────────────────────────
// Cache warming
// ────────────────────────────────────────────────────────────────

pub async fn warm_anchor_cache(cache: &CacheManager<AnchorInfo>) {
    // Replace with: db.get_top_anchors(20).await?
    let top_anchors = vec![
        AnchorInfo {
            id: "anchor-a".into(),
            name: "Anchor A".into(),
            home_domain: "anchor-a.example.com".into(),
            sep_10: true,
            sep_31: true,
            status: AnchorStatus::Active,
        },
        AnchorInfo {
            id: "anchor-b".into(),
            name: "Anchor B".into(),
            home_domain: "anchor-b.example.com".into(),
            sep_10: true,
            sep_31: false,
            status: AnchorStatus::Active,
        },
    ];

    for anchor in &top_anchors {
        let key = cache_key(&anchor.id);
        cache.set(key, anchor.clone(), DEFAULT_TTL).await;
    }
    info!("Anchor cache warmed with {} entries", top_anchors.len());
}

// ────────────────────────────────────────────────────────────────
// Router
// ────────────────────────────────────────────────────────────────

pub fn anchor_router(state: AnchorState) -> Router {
    Router::new()
        .route("/anchors/:id", get(get_anchor))
        .route("/anchors/:id/status-change", post(on_anchor_status_change))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_anchor_cache_hit() {
        let cache = Arc::new(CacheManager::new(100));
        let anchor = AnchorInfo {
            id: "anchor-a".into(),
            name: "Anchor A".into(),
            home_domain: "anchor-a.example.com".into(),
            sep_10: true,
            sep_31: true,
            status: AnchorStatus::Active,
        };
        cache
            .set("anchor:anchor-a", anchor.clone(), DEFAULT_TTL)
            .await;
        let result = cache.get("anchor:anchor-a").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "anchor-a");
    }

    #[tokio::test]
    async fn test_anchor_status_change_invalidates() {
        let cache = Arc::new(CacheManager::new(100));
        let anchor = AnchorInfo {
            id: "anchor-a".into(),
            name: "Anchor A".into(),
            home_domain: "anchor-a.example.com".into(),
            sep_10: true,
            sep_31: true,
            status: AnchorStatus::Active,
        };
        cache.set("anchor:anchor-a", anchor, DEFAULT_TTL).await;
        cache.publish_event(CacheInvalidationEvent::AnchorStatusChanged {
            anchor_id: "anchor-a".to_string(),
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(cache.get("anchor:anchor-a").await.is_none());
    }

    #[tokio::test]
    async fn test_anchor_warming() {
        let cache = Arc::new(CacheManager::new(100));
        warm_anchor_cache(&cache).await;
        assert!(cache.get("anchor:anchor-a").await.is_some());
        assert!(cache.get("anchor:anchor-b").await.is_some());
    }
}
