use axum::{extract::Query, http::StatusCode, response::Json, Extension};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::ml::{MLService, PredictionResult};

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

pub async fn retrain_model(
    Extension(ml_service): Extension<Arc<RwLock<MLService>>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut service = ml_service.write().await;
    
    match service.retrain_weekly().await {
        Ok(_) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Model retrained successfully"
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
