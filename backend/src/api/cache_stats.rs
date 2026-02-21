use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;

use crate::cache::{CacheManager, CacheStats};

#[derive(Serialize)]
pub struct CacheStatsResponse {
    pub hits: u64,
    pub misses: u64,
    pub invalidations: u64,
    pub hit_rate_percent: f64,
    pub total_requests: u64,
}

impl From<CacheStats> for CacheStatsResponse {
    fn from(stats: CacheStats) -> Self {
        let total_requests = stats.hits + stats.misses;
        Self {
            hits: stats.hits,
            misses: stats.misses,
            invalidations: stats.invalidations,
            hit_rate_percent: stats.hit_rate(),
            total_requests,
        }
    }
}

/// Handler for GET /api/cache/stats - Get cache hit rate monitoring
pub async fn get_cache_stats(
    State(cache): State<Arc<CacheManager>>,
    headers: HeaderMap,
) -> Response {
    let stats = cache.get_stats();
    let response = CacheStatsResponse::from(stats);

    match crate::http_cache::cached_json_response(&headers, "cache:stats", &response, 30) {
        Ok(resp) => resp,
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Handler for POST /api/cache/reset - Reset cache statistics
pub async fn reset_cache_stats(State(cache): State<Arc<CacheManager>>) -> Json<serde_json::Value> {
    cache.reset_stats();
    Json(serde_json::json!({
        "status": "success",
        "message": "Cache statistics reset"
    }))
}

pub fn routes(cache: Arc<CacheManager>) -> Router {
    Router::new()
        .route("/api/cache/stats", get(get_cache_stats))
        .route("/api/cache/reset", axum::routing::post(reset_cache_stats))
        .with_state(cache)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats_response_conversion() {
        let stats = CacheStats {
            hits: 80,
            misses: 20,
            invalidations: 5,
        };

        let response = CacheStatsResponse::from(stats);
        assert_eq!(response.hits, 80);
        assert_eq!(response.misses, 20);
        assert_eq!(response.invalidations, 5);
        assert_eq!(response.hit_rate_percent, 80.0);
        assert_eq!(response.total_requests, 100);
    }

    #[test]
    fn test_cache_stats_response_zero_requests() {
        let stats = CacheStats {
            hits: 0,
            misses: 0,
            invalidations: 0,
        };

        let response = CacheStatsResponse::from(stats);
        assert_eq!(response.hit_rate_percent, 0.0);
        assert_eq!(response.total_requests, 0);
    }
}
