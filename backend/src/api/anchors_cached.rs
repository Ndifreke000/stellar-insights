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
use crate::rpc::StellarRpcClient;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnchorsResponse {
    pub anchors: Vec<AnchorMetricsResponse>,
    pub total: usize,
}

/// GET /api/anchors - List all anchors with key metrics (cached)
/// 
/// **DATA SOURCE: RPC + Database**
/// - Anchor metadata (name, account) from database
/// - Transaction metrics calculated from RPC payment data
pub async fn get_anchors(
    State((db, cache, rpc_client)): State<(Arc<Database>, Arc<CacheManager>, Arc<StellarRpcClient>)>,
    Query(params): Query<ListAnchorsQuery>,
) -> ApiResult<Json<AnchorsResponse>> {
    let cache_key = keys::anchor_list(params.limit, params.offset);

    let response = <()>::get_or_fetch(
        &cache,
        &cache_key,
        cache.config.get_ttl("anchor"),
        async {
            // Get anchor metadata from database (names, accounts, etc.)
            let anchors = db.list_anchors(params.limit, params.offset).await?;

            let mut anchor_responses = Vec::new();

            for anchor in anchors {
                let anchor_id = uuid::Uuid::parse_str(&anchor.id)
                    .unwrap_or_else(|_| uuid::Uuid::nil());
                
                // Get asset count from database (metadata)
                let assets = db.get_assets_by_anchor(anchor_id).await?;

                // **RPC DATA**: Fetch real-time payment data for this anchor
                let payments = match rpc_client
                    .fetch_account_payments(&anchor.stellar_account, 200)
                    .await
                {
                    Ok(payments) => payments,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to fetch payments for anchor {}: {}. Using cached data.",
                            anchor.stellar_account,
                            e
                        );
                        // Fallback to database values if RPC fails
                        vec![]
                    }
                };

                // Calculate metrics from RPC payment data
                let (total_transactions, successful_transactions, failed_transactions) = 
                    if !payments.is_empty() {
                        let total = payments.len() as i64;
                        // In Stellar, if a payment appears in the ledger, it was successful
                        // Failed payments don't appear in the payment stream
                        let successful = total;
                        let failed = 0;
                        (total, successful, failed)
                    } else {
                        // Fallback to database values
                        (
                            anchor.total_transactions,
                            anchor.successful_transactions,
                            anchor.failed_transactions,
                        )
                    };

                let failure_rate = if total_transactions > 0 {
                    (failed_transactions as f64 / total_transactions as f64) * 100.0
                } else {
                    0.0
                };

                let reliability_score = if total_transactions > 0 {
                    (successful_transactions as f64 / total_transactions as f64) * 100.0
                } else {
                    anchor.reliability_score
                };

                let status = if reliability_score >= 99.0 {
                    "green".to_string()
                } else if reliability_score >= 95.0 {
                    "yellow".to_string()
                } else {
                    "red".to_string()
                };

                let anchor_response = AnchorMetricsResponse {
                    id: anchor.id.to_string(),
                    name: anchor.name,
                    stellar_account: anchor.stellar_account,
                    reliability_score,
                    asset_coverage: assets.len(),
                    failure_rate,
                    total_transactions,
                    successful_transactions,
                    failed_transactions,
                    status,
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
