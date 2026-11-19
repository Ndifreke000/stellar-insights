use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::services::preflight::{self, PreflightError, PreflightRequest};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct PreflightQuery {
    pub source_asset: String,
    pub destination_asset: String,
    pub amount_usd: f64,
    #[serde(default)]
    pub min_success_rate: Option<f64>,
    #[serde(default)]
    pub max_risk_score: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
}

pub async fn preflight_handler(
    State(state): State<AppState>,
    Query(query): Query<PreflightQuery>,
) -> impl IntoResponse {
    let request = PreflightRequest {
        source_asset: query.source_asset,
        destination_asset: query.destination_asset,
        amount_usd: query.amount_usd,
        min_success_rate: query.min_success_rate,
        max_risk_score: query.max_risk_score,
    };

    match preflight::run_preflight(&state.db, request).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(PreflightError::InvalidInput(msg)) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody { error: msg }),
        )
            .into_response(),
        Err(PreflightError::NotFound(msg)) => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody { error: msg }),
        )
            .into_response(),
        Err(PreflightError::Internal(msg)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody { error: msg }),
        )
            .into_response(),
    }
}
