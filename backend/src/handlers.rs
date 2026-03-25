use axum::{
    response::IntoResponse,
    extract::State,
    Json,
};

use crate::state::AppState;

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "stellar-insights-backend",
        "version": env!("CARGO_PKG_VERSION"),
        "api": {
            "current_version": "v1",
            "supported_versions": ["v1"],
            "status": "active"
        }
    }))
}

/// Database pool metrics endpoint
pub async fn pool_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = state.db.pool_metrics();
    Json(metrics)
}

pub async fn ingestion_status(
    State(app_state): State<AppState>,
) -> crate::error::ApiResult<Json<crate::ingestion::IngestionStatus>> {
    let status = app_state.ingestion.get_ingestion_status().await?;
    Ok(Json(status))
}

