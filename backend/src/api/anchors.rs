use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::AnchorMetadata;
use crate::services::stellar_toml::StellarTomlClient;
use crate::state::AppState;

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

#[derive(Debug, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<AnchorMetadata>,
}

#[derive(Debug, Serialize)]
pub struct AnchorsResponse {
    pub anchors: Vec<AnchorMetricsResponse>,
    pub total: usize,
}

/// GET /api/anchors - List all anchors with key metrics
pub async fn get_anchors(
    State(app_state): State<AppState>,
    Query(params): Query<ListAnchorsQuery>,
) -> ApiResult<Json<AnchorsResponse>> {
    let anchors = app_state
        .db
        .list_anchors(params.limit, params.offset)
        .await?;

    // Create stellar.toml client
    let toml_client = Arc::new(
        StellarTomlClient::new(
            app_state.redis_connection.clone(),
            Some("Public Global Stellar Network ; September 2015".to_string()),
        )
        .map_err(|e| ApiError::InternalError(format!("Failed to create TOML client: {}", e)))?,
    );

    let mut anchor_responses = Vec::new();

    for anchor in anchors {
        // Get assets for each anchor to calculate asset coverage
        let anchor_id = uuid::Uuid::parse_str(&anchor.id).unwrap_or_else(|_| uuid::Uuid::nil());
        let assets = app_state.db.get_assets_by_anchor(anchor_id).await?;

        let failure_rate = if anchor.total_transactions > 0 {
            (anchor.failed_transactions as f64 / anchor.total_transactions as f64) * 100.0
        } else {
            0.0
        };

        // Fetch stellar.toml metadata if home_domain is available
        let metadata = if let Some(ref domain) = anchor.home_domain {
            match toml_client.fetch_toml(domain).await {
                Ok(toml) => {
                    let supported_currencies = toml
                        .currencies
                        .as_ref()
                        .map(|currencies| currencies.iter().map(|c| c.code.clone()).collect());

                    Some(AnchorMetadata {
                        organization_name: toml.organization_name,
                        organization_dba: toml.organization_dba,
                        organization_url: toml.organization_url,
                        organization_logo: toml.organization_logo,
                        organization_description: toml.organization_description,
                        organization_support_email: toml.organization_support_email,
                        supported_currencies,
                        fetched_at: Some(toml.fetched_at),
                    })
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch stellar.toml for {}: {}", domain, e);
                    None
                }
            }
        } else {
            None
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
            metadata,
        };

        anchor_responses.push(anchor_response);
    }

    let total = anchor_responses.len();

    Ok(Json(AnchorsResponse {
        anchors: anchor_responses,
        total,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Anchor;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_anchor_metrics_response_creation() {
        let anchor_id = Uuid::new_v4();
        let anchor = Anchor {
            id: anchor_id.to_string(),
            name: "Test Anchor".to_string(),
            stellar_account: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
            home_domain: Some("test.com".to_string()),
            total_transactions: 1000,
            successful_transactions: 950,
            failed_transactions: 50,
            total_volume_usd: 1000000.0,
            avg_settlement_time_ms: 2000,
            reliability_score: 95.5,
            status: "green".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let failure_rate =
            (anchor.failed_transactions as f64 / anchor.total_transactions as f64) * 100.0;

        let response = AnchorMetricsResponse {
            id: anchor.id.to_string(),
            name: anchor.name,
            stellar_account: anchor.stellar_account,
            reliability_score: anchor.reliability_score,
            asset_coverage: 3,
            failure_rate,
            total_transactions: anchor.total_transactions,
            successful_transactions: anchor.successful_transactions,
            failed_transactions: anchor.failed_transactions,
            status: anchor.status,
        };

        assert_eq!(response.name, "Test Anchor");
        assert_eq!(response.reliability_score, 95.5);
        assert_eq!(response.asset_coverage, 3);
        assert_eq!(response.failure_rate, 5.0);
        assert_eq!(response.status, "green");
    }

    #[test]
    fn test_failure_rate_calculation_zero_transactions() {
        let anchor = Anchor {
            id: Uuid::new_v4().to_string(),
            name: "Empty Anchor".to_string(),
            stellar_account: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
            home_domain: None,
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            total_volume_usd: 0.0,
            avg_settlement_time_ms: 0,
            reliability_score: 0.0,
            status: "red".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let failure_rate = if anchor.total_transactions > 0 {
            (anchor.failed_transactions as f64 / anchor.total_transactions as f64) * 100.0
        } else {
            0.0
        };

        assert_eq!(failure_rate, 0.0);
    }

    #[test]
    fn test_failure_rate_calculation_with_transactions() {
        let anchor = Anchor {
            id: Uuid::new_v4().to_string(),
            name: "Test Anchor".to_string(),
            stellar_account: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
            home_domain: None,
            total_transactions: 100,
            successful_transactions: 80,
            failed_transactions: 20,
            total_volume_usd: 10000.0,
            avg_settlement_time_ms: 5000,
            reliability_score: 80.0,
            status: "yellow".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let failure_rate =
            (anchor.failed_transactions as f64 / anchor.total_transactions as f64) * 100.0;

        assert_eq!(failure_rate, 20.0);
    }
}
