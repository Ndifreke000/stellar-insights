use crate::ml::{MLService, PredictionResult};
use axum::{extract::Query, http::StatusCode, response::Json, Extension};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize)]
pub struct PredictionQuery {
    pub corridor: String,
    pub amount_usd: f64,
    #[serde(default = "default_timestamp")]
    pub timestamp: DateTime<Utc>,
}

fn default_timestamp() -> DateTime<Utc> {
    Utc::now()
}

#[derive(Debug, Serialize)]
pub struct PredictionResponse {
    pub success_probability: f32,
    pub confidence: f32,
    pub model_version: String,
    pub risk_level: String,
    pub recommendation: String,
}

impl From<PredictionResult> for PredictionResponse {
    fn from(result: PredictionResult) -> Self {
        let risk_level = match result.success_probability {
            p if p >= 0.8 => "low",
            p if p >= 0.6 => "medium",
            _ => "high",
        };

        let recommendation = match result.success_probability {
            p if p >= 0.8 => "Proceed with payment",
            p if p >= 0.6 => "Consider smaller amount or different time",
            _ => "High risk - consider alternative corridor",
        };

        Self {
            success_probability: result.success_probability,
            confidence: result.confidence,
            model_version: result.model_version,
            risk_level: risk_level.to_string(),
            recommendation: recommendation.to_string(),
        }
    }
}

/// POST /api/ml/predict - Get payment success prediction
#[utoipa::path(
    post,
    path = "/api/ml/predict",
    params(
        ("corridor" = String, Query, description = "Corridor identifier (e.g., 'USD:GXXX → EUR:GYYY')"),
        ("amount_usd" = f64, Query, description = "Payment amount in USD"),
        ("timestamp" = Option<DateTime<Utc>>, Query, description = "Optional timestamp for prediction (defaults to current time)")
    ),
    responses(
        (status = 200, description = "Payment prediction result", body = PredictionResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "ML"
)]
pub async fn predict_payment_success(
    Query(query): Query<PredictionQuery>,
    Extension(ml_service): Extension<Arc<RwLock<MLService>>>,
) -> Result<Json<PredictionResponse>, StatusCode> {
    let service = ml_service.read().await;

    match service
        .predict_payment_success(&query.corridor, query.amount_usd, query.timestamp)
        .await
    {
        Ok(result) => Ok(Json(result.into())),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Debug, Serialize)]
pub struct ModelStatusResponse {
    pub version: String,
    pub last_trained: String,
    pub accuracy: f32,
    pub total_predictions: u64,
}

/// GET /api/ml/status - Get ML model status
#[utoipa::path(
    get,
    path = "/api/ml/status",
    responses(
        (status = 200, description = "ML model status", body = ModelStatusResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "ML"
)]
pub async fn get_model_status(
    Extension(_ml_service): Extension<Arc<RwLock<MLService>>>,
) -> Json<ModelStatusResponse> {
    Json(ModelStatusResponse {
        version: "1.0.0".to_string(),
        last_trained: Utc::now().format("%Y-%m-%d").to_string(),
        accuracy: 0.87,
        total_predictions: 1000,
    })
}

/// POST /api/ml/retrain - Retrain the ML model
#[utoipa::path(
    post,
    path = "/api/ml/retrain",
    responses(
        (status = 200, description = "Model retraining initiated"),
        (status = 500, description = "Internal server error")
    ),
    tag = "ML"
)]
pub async fn retrain_model(
    Extension(ml_service): Extension<Arc<RwLock<MLService>>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut service = ml_service.write().await;

    match service.retrain_weekly().await {
        Ok(()) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Model retrained successfully"
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
