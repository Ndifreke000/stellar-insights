use axum::{extract::Query, Json};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PredictionQuery {
    pub source_asset: String,
    pub destination_asset: String,
    pub amount: f64,
    pub time_of_day: String,
}

#[derive(Debug, Serialize)]
pub struct PredictionResponse {
    pub success_probability: f64,
    pub confidence_interval: (f64, f64),
    pub alternative_routes: Vec<String>,
}

/// POST /api/predict/success - Predict payment success
#[utoipa::path(
    post,
    path = "/api/predict/success",
    params(
        ("source_asset" = String, Query, description = "Source asset code (e.g., 'USD')"),
        ("destination_asset" = String, Query, description = "Destination asset code (e.g., 'EUR')"),
        ("amount" = f64, Query, description = "Payment amount"),
        ("time_of_day" = String, Query, description = "Time of day for prediction (e.g., 'morning', 'afternoon', 'evening')")
    ),
    responses(
        (status = 200, description = "Payment success prediction", body = PredictionResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prediction"
)]
pub async fn predict_success(Query(_params): Query<PredictionQuery>) -> Json<PredictionResponse> {
    // Mock implementation
    let mut rng = rand::thread_rng();
    let probability = rng.gen_range(0.8..0.98);

    let response = PredictionResponse {
        success_probability: probability,
        confidence_interval: (probability - 0.05, probability + 0.02),
        alternative_routes: vec![
            "Route via XLM".to_string(),
            "Route via secondary anchor".to_string(),
        ],
    };

    Json(response)
}
