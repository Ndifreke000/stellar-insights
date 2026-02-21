//! API handlers for snapshot verification rewards

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::auth::sep10_middleware::{sep10_auth_middleware, Sep10User};
use crate::auth::sep10_simple::Sep10Service;
use crate::services::verification_rewards::{VerificationRewardsService, VerifySnapshotRequest};

/// Build verification rewards routes
pub fn routes(
    service: Arc<VerificationRewardsService>,
    sep10_service: Arc<Sep10Service>,
) -> Router {
    Router::new()
        .route("/verify", post(verify_snapshot))
        .route("/stats", get(get_user_stats))
        .route("/history", get(get_user_verifications))
        .layer(middleware::from_fn_with_state(
            sep10_service.clone(),
            sep10_auth_middleware,
        ))
        .route("/leaderboard", get(get_leaderboard))
        .route("/stats/:user_id", get(get_public_user_stats))
        .with_state(service)
}

/// Query parameters for leaderboard
#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    #[serde(default = "default_limit")]
    pub limit: i32,
}

fn default_limit() -> i32 {
    10
}

/// Query parameters for user verifications
#[derive(Debug, Deserialize)]
pub struct VerificationsQuery {
    #[serde(default = "default_verification_limit")]
    pub limit: i32,
}

fn default_verification_limit() -> i32 {
    20
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// POST /api/verifications/verify
/// Verify a snapshot hash and earn rewards
pub async fn verify_snapshot(
    State(service): State<Arc<VerificationRewardsService>>,
    sep10_user: axum::Extension<Sep10User>,
    Json(request): Json<VerifySnapshotRequest>,
) -> Result<Response, VerificationError> {
    info!(
        "Verification request from user {} for snapshot {}",
        sep10_user.account, request.snapshot_id
    );

    let response = service
        .verify_and_reward(&sep10_user.account, request)
        .await
        .map_err(|e| VerificationError::VerificationFailed(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// GET /api/verifications/stats
/// Get current user's reward statistics
pub async fn get_user_stats(
    State(service): State<Arc<VerificationRewardsService>>,
    sep10_user: axum::Extension<Sep10User>,
) -> Result<Response, VerificationError> {
    let stats = service
        .get_user_stats(&sep10_user.account)
        .await
        .map_err(|e| VerificationError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(stats)).into_response())
}

/// GET /api/verifications/leaderboard
/// Get leaderboard of top verifiers
pub async fn get_leaderboard(
    State(service): State<Arc<VerificationRewardsService>>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Response, VerificationError> {
    let limit = query.limit.min(100).max(1); // Cap between 1 and 100

    let leaderboard = service
        .get_leaderboard(limit)
        .await
        .map_err(|e| VerificationError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(leaderboard)).into_response())
}

/// GET /api/verifications/history
/// Get current user's verification history
pub async fn get_user_verifications(
    State(service): State<Arc<VerificationRewardsService>>,
    sep10_user: axum::Extension<Sep10User>,
    Query(query): Query<VerificationsQuery>,
) -> Result<Response, VerificationError> {
    let limit = query.limit.min(100).max(1); // Cap between 1 and 100

    let verifications = service
        .get_user_verifications(&sep10_user.account, limit)
        .await
        .map_err(|e| VerificationError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(verifications)).into_response())
}

/// GET /api/verifications/stats/:user_id
/// Get reward statistics for a specific user (public)
pub async fn get_public_user_stats(
    State(service): State<Arc<VerificationRewardsService>>,
    Path(user_id): Path<String>,
) -> Result<Response, VerificationError> {
    let stats = service
        .get_user_stats(&user_id)
        .await
        .map_err(|e| VerificationError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(stats)).into_response())
}

/// Error types for verification API
#[derive(Debug)]
pub enum VerificationError {
    VerificationFailed(String),
    DatabaseError(String),
    Unauthorized(String),
}

impl IntoResponse for VerificationError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            VerificationError::VerificationFailed(msg) => (StatusCode::BAD_REQUEST, msg),
            VerificationError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            VerificationError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
        };

        let body = Json(ErrorResponse { error: message });

        (status, body).into_response()
    }
}
