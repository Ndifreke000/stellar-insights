use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::cache::{keys, CacheManager};
use crate::cache_middleware::CacheAware;
use crate::database::Database;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

#[derive(Debug, Deserialize)]
pub struct ListAnchorsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize, Clone)]
pub struct AnchorMetricsResponse {
    pub id: String,
    pub name: String,
    pub stellar_account: String,
    pub reliability_score: f64,
    pub asset_coverage: usize,
    pub failure_rate: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub status: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AnchorsResponse {
    pub anchors: Vec<AnchorMetricsResponse>,
    pub total: usize,
}

/// GET /api/anchors - List all anchors with key metrics (cached)
pub async fn get_anchors(
    State((db, cache)): State<(Arc<Database>, Arc<CacheManager>)>,
    Query(params): Query<ListAnchorsQuery>,
) -> ApiResult<Json<AnchorsResponse>> {
    let cache_key = keys::anchor_list(params.limit, params.offset);

    let response = <()>::get_or_fetch(
        &cache,
        &cache_key,
        cache.config.anchor_data_ttl,
        async {
            let anchors = db.list_anchors(params.limit, params.offset).await?;

            let mut anchor_responses = Vec::new();

            for anchor in anchors {
                let anchor_id = uuid::Uuid::parse_str(&anchor.id)
                    .unwrap_or_else(|_| uuid::Uuid::nil());
                let assets = db.get_assets_by_anchor(anchor_id).await?;

                let failure_rate = if anchor.total_transactions > 0 {
                    (anchor.failed_transactions as f64 / anchor.total_transactions as f64) * 100.0
                } else {
                    0.0
                };

                let anchor_response = AnchorMetricsResponse {
                    id: anchor.id.to_string(),
                    name: anchor.name,
                    stellar_account: anchor.stellar_account,
                    reliability_score: anchor.reliability_score,
                    asset_coverage: assets.len(),
                    failure_rate,
                    total_transactions: anchor.total_transactions,
                    successful_transactions: anchor.successful_transactions,
                    failed_transactions: anchor.failed_transactions,
                    status: anchor.status,
                };

                anchor_responses.push(anchor_response);
            }

            let total = anchor_responses.len();

            Ok(AnchorsResponse {
                anchors: anchor_responses,
                total,
            })
        },
    )
    .await?;

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let key = keys::anchor_list(50, 0);
        assert_eq!(key, "anchor:list:50:0");
    }

    #[test]
    fn test_anchor_metrics_response_creation() {
        let response = AnchorMetricsResponse {
            id: "123".to_string(),
            name: "Test Anchor".to_string(),
            stellar_account: "GA123".to_string(),
            reliability_score: 95.5,
            asset_coverage: 3,
            failure_rate: 5.0,
            total_transactions: 1000,
            successful_transactions: 950,
            failed_transactions: 50,
            status: "green".to_string(),
        };

        assert_eq!(response.name, "Test Anchor");
        assert_eq!(response.reliability_score, 95.5);
        assert_eq!(response.asset_coverage, 3);
    }
}
