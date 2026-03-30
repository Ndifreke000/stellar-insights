use axum::{extract::State, Json};

use crate::error::ApiResult;
use crate::state::AppState;

pub async fn ingestion_status(
    State(app_state): State<AppState>,
) -> ApiResult<Json<crate::ingestion::IngestionStatus>> {
    let status = app_state.ingestion.get_ingestion_status().await?;
    Ok(Json(status))
}
