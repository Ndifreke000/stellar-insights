use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

/// Machine-readable reason code attached to each preflight check so that
/// integrators can branch on the outcome without parsing free-text `detail`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreflightReasonCode {
    LowSuccessRate,
    ThinLiquidity,
    SmallSample,
    UnknownCorridor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
    pub code: PreflightReasonCode,
}

#[derive(Debug, Deserialize)]
pub struct PreflightQuery {
    pub source_asset: String,
    pub destination_asset: String,
    pub amount_usd: f64,
}

#[derive(Debug, Serialize)]
pub struct PreflightResponse {
    pub checks: Vec<PreflightCheck>,
    pub passed: bool,
}

pub async fn preflight_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PreflightQuery>,
) -> Response {
    let corridor = state
        .corridor_stats(&query.source_asset, &query.destination_asset)
        .await;

    let mut checks: Vec<PreflightCheck> = Vec::new();

    match corridor {
        None => {
            checks.push(PreflightCheck {
                name: "corridor".to_string(),
                passed: false,
                detail: format!(
                    "No data for corridor {} -> {}",
                    query.source_asset, query.destination_asset
                ),
                code: PreflightReasonCode::UnknownCorridor,
            });
        }
        Some(stats) => {
            let success_rate = stats.success_rate;
            checks.push(PreflightCheck {
                name: "success_rate".to_string(),
                passed: success_rate >= 0.9,
                detail: format!("Success rate is {:.2}%", success_rate * 100.0),
                code: PreflightReasonCode::LowSuccessRate,
            });

            let liquidity = stats.liquidity_usd;
            checks.push(PreflightCheck {
                name: "liquidity".to_string(),
                passed: liquidity >= query.amount_usd,
                detail: format!(
                    "Available liquidity ${:.2} vs requested ${:.2}",
                    liquidity, query.amount_usd
                ),
                code: PreflightReasonCode::ThinLiquidity,
            });

            let sample_size = stats.sample_size;
            checks.push(PreflightCheck {
                name: "sample_size".to_string(),
                passed: sample_size >= 30,
                detail: format!("Based on {} observations", sample_size),
                code: PreflightReasonCode::SmallSample,
            });
        }
    }

    let passed = checks.iter().all(|check| check.passed);

    (StatusCode::OK, Json(PreflightResponse { checks, passed })).into_response()
}
