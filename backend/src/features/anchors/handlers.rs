// Placeholder for anchor handlers

use anyhow::Result;
use axum::{extract::State, Json};
use stellar_insights_backend::error::ApiResult;
use stellar_insights_backend::state::AppState;

pub async fn get_anchor(
    State(app_state): State<AppState>,
) -> ApiResult<Json<crate::models::Anchor>> {
    // TODO: implement
    Ok(Json(crate::models::Anchor::default()))
}

