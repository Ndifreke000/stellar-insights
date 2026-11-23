//! API handlers for snapshot verification rewards
//!
//! #1868 N+1 audit: verify/stats/history/leaderboard handlers each call a
//! single service method backed by one (or a fixed small number of) queries.
//! No per-row await loops in these handlers.

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

use crate::auth::sep10_simple::Sep10Service;
use crate::auth::{sep10_auth_middleware, Sep10User};
use crate::services::verification_rewards::{
    ClusterInfo, ReviewQueueItem, VerificationRewardsService, VerifySnapshotRequest,
};

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
            sep10_service,
            sep10_auth_middleware,
        ))
        .route("/leaderboard", get(get_leaderboard))
        .route("/stats/{user_id}", get(get_public_user_stats))
        .route("/review-queue", get(get_review_queue))
        .route("/review-queue/{id}/approve", post(approve_review))
        .route("/review-queue/{id}/reject", post(reject_review))
        .route("/cluster/{user_id}", get(get_cluster_info))
        .with_state(service)
}

#[derive(Debug, Deserialize)]
pub struct ReviewQueueQuery {
    pub status: Option<String>,
    #[serde(default = "default_review_limit")]
    pub limit: i32,
}

const fn default_review_limit() -> i32 {
    50
}

/// Query parameters for leaderboard
#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    #[serde(default = "default_limit")]
    pub limit: i32,
}

const fn default_limit() -> i32 {
    10
}

/// Query parameters for user verifications
#[derive(Debug, Deserialize)]
pub struct VerificationsQuery {
    #[serde(default = "default_verification_limit")]
    pub limit: i32,
}

const fn default_verification_limit() -> i32 {
    20
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// POST /api/verifications/verify - Verify a snapshot hash and earn rewards
#[utoipa::path(
    post,
    path = "/api/verifications/verify",
    operation_id = "verify_snapshot_for_reward",
    request_body = VerifySnapshotRequest,
    responses(
        (status = 200, description = "Snapshot verified and reward issued"),
        (status = 400, description = "Verification failed"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Verification Rewards"
)]
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

/// GET /api/verifications/stats - Get current user's reward statistics
#[utoipa::path(
    get,
    path = "/api/verifications/stats",
    responses(
        (status = 200, description = "User reward statistics"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Verification Rewards"
)]
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

/// GET /api/verifications/leaderboard - Get leaderboard of top verifiers
#[utoipa::path(
    get,
    path = "/api/verifications/leaderboard",
    params(
        ("limit" = Option<i32>, Query, description = "Maximum results (1-100, default 10)")
    ),
    responses(
        (status = 200, description = "Leaderboard of top verifiers")
    ),
    tag = "Verification Rewards"
)]
pub async fn get_leaderboard(
    State(service): State<Arc<VerificationRewardsService>>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Response, VerificationError> {
    let limit = query.limit.clamp(1, 100); // Cap between 1 and 100

    let leaderboard = service
        .get_leaderboard(limit)
        .await
        .map_err(|e| VerificationError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(leaderboard)).into_response())
}

/// GET /api/verifications/history - Get current user's verification history
#[utoipa::path(
    get,
    path = "/api/verifications/history",
    params(
        ("limit" = Option<i32>, Query, description = "Maximum results (1-100, default 20)")
    ),
    responses(
        (status = 200, description = "User's verification history"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Verification Rewards"
)]
pub async fn get_user_verifications(
    State(service): State<Arc<VerificationRewardsService>>,
    sep10_user: axum::Extension<Sep10User>,
    Query(query): Query<VerificationsQuery>,
) -> Result<Response, VerificationError> {
    let limit = query.limit.clamp(1, 100); // Cap between 1 and 100

    let verifications = service
        .get_user_verifications(&sep10_user.account, limit)
        .await
        .map_err(|e| VerificationError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(verifications)).into_response())
}

/// GET /api/verifications/stats/:user_id - Get reward statistics for a specific user (public)
#[utoipa::path(
    get,
    path = "/api/verifications/stats/{user_id}",
    params(
        ("user_id" = String, Path, description = "User ID to get stats for")
    ),
    responses(
        (status = 200, description = "User's reward statistics")
    ),
    tag = "Verification Rewards"
)]
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

/// GET /api/verifications/review-queue - Get items in verification review queue (#2823)
#[utoipa::path(
    get,
    path = "/api/verifications/review-queue",
    params(
        ("status" = Option<String>, Query, description = "Status filter ('pending', 'approved', 'rejected')"),
        ("limit" = Option<i32>, Query, description = "Maximum items to return")
    ),
    responses(
        (status = 200, description = "Verification review queue items", body = Vec<ReviewQueueItem>)
    ),
    tag = "Verification Rewards"
)]
pub async fn get_review_queue(
    State(service): State<Arc<VerificationRewardsService>>,
    Query(query): Query<ReviewQueueQuery>,
) -> Result<Response, VerificationError> {
    let limit = query.limit.clamp(1, 100);
    let items = service
        .get_review_queue(query.status.as_deref(), limit)
        .await
        .map_err(|e| VerificationError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(items)).into_response())
}

/// POST /api/verifications/review-queue/{id}/approve - Approve review item
#[utoipa::path(
    post,
    path = "/api/verifications/review-queue/{id}/approve",
    params(
        ("id" = String, Path, description = "Review item ID")
    ),
    responses(
        (status = 200, description = "Review item approved")
    ),
    tag = "Verification Rewards"
)]
pub async fn approve_review(
    State(service): State<Arc<VerificationRewardsService>>,
    Path(id): Path<String>,
) -> Result<Response, VerificationError> {
    service
        .approve_review(&id, "admin")
        .await
        .map_err(|e| VerificationError::DatabaseError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"status": "approved"})),
    )
        .into_response())
}

/// POST /api/verifications/review-queue/{id}/reject - Reject review item
#[utoipa::path(
    post,
    path = "/api/verifications/review-queue/{id}/reject",
    params(
        ("id" = String, Path, description = "Review item ID")
    ),
    responses(
        (status = 200, description = "Review item rejected")
    ),
    tag = "Verification Rewards"
)]
pub async fn reject_review(
    State(service): State<Arc<VerificationRewardsService>>,
    Path(id): Path<String>,
) -> Result<Response, VerificationError> {
    service
        .reject_review(&id, "admin")
        .await
        .map_err(|e| VerificationError::DatabaseError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"status": "rejected"})),
    )
        .into_response())
}

/// GET /api/verifications/cluster/{user_id} - Get cluster information for sybil tracking
#[utoipa::path(
    get,
    path = "/api/verifications/cluster/{user_id}",
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Cluster info", body = ClusterInfo)
    ),
    tag = "Verification Rewards"
)]
pub async fn get_cluster_info(
    State(service): State<Arc<VerificationRewardsService>>,
    Path(user_id): Path<String>,
) -> Result<Response, VerificationError> {
    let info = service
        .get_cluster_info(&user_id)
        .await
        .map_err(|e| VerificationError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(info)).into_response())
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
            Self::VerificationFailed(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
        };

        let body = Json(ErrorResponse { error: message });

        (status, body).into_response()
    }
}
