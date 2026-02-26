use crate::models::{CreateSponsorshipRequest, Sponsorship};
use crate::services::SponsorshipTrackerService;
use crate::AppState;
use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PaginationParams {
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Get all sponsorships with pagination
pub async fn get_all_sponsorships(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Vec<Sponsorship>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).min(1000);
    let offset = params.offset.unwrap_or(0).max(0);

    let service = SponsorshipTrackerService::new((**state.db).clone());
    
    service
        .get_all_sponsorships(limit, offset)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a new sponsorship record
pub async fn create_sponsorship(
    State(state): State<AppState>,
    Json(req): Json<CreateSponsorshipRequest>,
) -> Result<(StatusCode, Json<Sponsorship>), (StatusCode, String)> {
    let service = SponsorshipTrackerService::new((**state.db).clone());
    
    service
        .track_sponsorship(
            req.sponsor,
            req.sponsored_account,
            req.sponsored_reserves,
            req.total_amount,
        )
        .await
        .map(|s| (StatusCode::CREATED, Json(s)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get a specific sponsorship by ID
pub async fn get_sponsorship(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Sponsorship>, (StatusCode, String)> {
    let query = "SELECT * FROM sponsorships WHERE id = ?";
    
    sqlx::query_as::<_, Sponsorship>(query)
        .bind(&id)
        .fetch_one(&**state.db)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))
}

/// Get sponsorship history for a specific sponsorship
pub async fn get_sponsorship_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<crate::models::SponsorshipHistory>>, (StatusCode, String)> {
    let service = SponsorshipTrackerService::new((**state.db).clone());
    
    service
        .get_sponsorship_history(id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get sponsor leaderboard
pub async fn get_sponsor_leaderboard(
    State(state): State<AppState>,
    Query(params): Query<serde_json::Map<String, serde_json::Value>>,
) -> Result<Json<Vec<crate::models::SponsorLeaderboard>>, (StatusCode, String)> {
    let limit = params
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(100)
        .min(1000)
        .max(1);

    let service = SponsorshipTrackerService::new((**state.db).clone());
    
    service
        .get_sponsor_leaderboard(limit)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get sponsorships for a specific account
pub async fn get_sponsorships_by_account(
    State(state): State<AppState>,
    Path(account): Path<String>,
) -> Result<Json<Vec<Sponsorship>>, (StatusCode, String)> {
    let service = SponsorshipTrackerService::new((**state.db).clone());
    
    service
        .get_sponsorships_for_account(account)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get analytics summary
pub async fn get_analytics_summary(
    State(state): State<AppState>,
) -> Result<Json<crate::models::SponsorshipAnalytics>, (StatusCode, String)> {
    let service = SponsorshipTrackerService::new((**state.db).clone());
    
    service
        .get_analytics()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
