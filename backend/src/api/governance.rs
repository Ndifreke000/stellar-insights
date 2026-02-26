use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::auth::sep10_middleware::{sep10_auth_middleware, Sep10User};
use crate::auth::sep10_simple::Sep10Service;
use crate::services::governance::{
    AddCommentRequest, CastVoteRequest, CreateProposalRequest, GovernanceService,
};

pub fn routes(service: Arc<GovernanceService>, sep10_service: Arc<Sep10Service>) -> Router {
    Router::new()
        // Protected routes (require SEP-10 auth)
        .route("/proposals", post(create_proposal))
        .route("/proposals/:id/vote", post(cast_vote))
        .route("/proposals/:id/comments", post(add_comment))
        .route("/proposals/:id/activate", put(activate_proposal))
        .layer(middleware::from_fn_with_state(
            sep10_service.clone(),
            sep10_auth_middleware,
        ))
        // Public routes
        .route("/proposals", get(list_proposals))
        .route("/proposals/:id", get(get_proposal))
        .route("/proposals/:id/votes", get(get_votes))
        .route("/proposals/:id/comments", get(get_comments))
        .route("/proposals/:id/has-voted/:address", get(has_voted))
        .with_state(service)
}

#[derive(Debug, Deserialize)]
pub struct ListProposalsQuery {
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Deserialize)]
pub struct VotesQuery {
    #[serde(default = "default_votes_limit")]
    pub limit: i64,
}

fn default_votes_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
pub struct CommentsQuery {
    #[serde(default = "default_comments_limit")]
    pub limit: i64,
}

fn default_comments_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
pub struct ActivateRequest {
    #[serde(default = "default_voting_duration")]
    pub voting_duration_secs: i64,
}

fn default_voting_duration() -> i64 {
    604800 // 7 days
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct HasVotedResponse {
    pub has_voted: bool,
}

// POST /proposals
async fn create_proposal(
    State(service): State<Arc<GovernanceService>>,
    sep10_user: axum::Extension<Sep10User>,
    Json(request): Json<CreateProposalRequest>,
) -> Result<Response, GovernanceError> {
    info!("Create proposal request from {}", sep10_user.account);

    let response = service
        .create_proposal(&sep10_user.account, request)
        .await
        .map_err(|e| GovernanceError::BadRequest(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

// PUT /proposals/:id/activate
async fn activate_proposal(
    State(service): State<Arc<GovernanceService>>,
    Path(id): Path<String>,
    sep10_user: axum::Extension<Sep10User>,
    Json(request): Json<ActivateRequest>,
) -> Result<Response, GovernanceError> {
    info!(
        "Activate proposal {} request from {}",
        id, sep10_user.account
    );

    let response = service
        .activate_proposal(&id, request.voting_duration_secs)
        .await
        .map_err(|e| GovernanceError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

// GET /proposals
async fn list_proposals(
    State(service): State<Arc<GovernanceService>>,
    Query(query): Query<ListProposalsQuery>,
) -> Result<Response, GovernanceError> {
    let limit = query.limit.min(100).max(1);
    let offset = query.offset.max(0);

    let response = service
        .list_proposals(query.status.as_deref(), limit, offset)
        .await
        .map_err(|e| GovernanceError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

// GET /proposals/:id
async fn get_proposal(
    State(service): State<Arc<GovernanceService>>,
    Path(id): Path<String>,
) -> Result<Response, GovernanceError> {
    let response = service
        .get_proposal(&id)
        .await
        .map_err(|e| GovernanceError::NotFound(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

// POST /proposals/:id/vote
async fn cast_vote(
    State(service): State<Arc<GovernanceService>>,
    Path(id): Path<String>,
    sep10_user: axum::Extension<Sep10User>,
    Json(request): Json<CastVoteRequest>,
) -> Result<Response, GovernanceError> {
    info!(
        "Vote on proposal {} from {}: {}",
        id, sep10_user.account, request.choice
    );

    let response = service
        .cast_vote(&id, &sep10_user.account, request)
        .await
        .map_err(|e| GovernanceError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

// GET /proposals/:id/votes
async fn get_votes(
    State(service): State<Arc<GovernanceService>>,
    Path(id): Path<String>,
    Query(query): Query<VotesQuery>,
) -> Result<Response, GovernanceError> {
    let limit = query.limit.min(100).max(1);

    let response = service
        .get_votes(&id, limit)
        .await
        .map_err(|e| GovernanceError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

// GET /proposals/:id/has-voted/:address
async fn has_voted(
    State(service): State<Arc<GovernanceService>>,
    Path((id, address)): Path<(String, String)>,
) -> Result<Response, GovernanceError> {
    let voted = service
        .has_voted(&id, &address)
        .await
        .map_err(|e| GovernanceError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(HasVotedResponse { has_voted: voted })).into_response())
}

// POST /proposals/:id/comments
async fn add_comment(
    State(service): State<Arc<GovernanceService>>,
    Path(id): Path<String>,
    sep10_user: axum::Extension<Sep10User>,
    Json(request): Json<AddCommentRequest>,
) -> Result<Response, GovernanceError> {
    info!("Add comment to proposal {} from {}", id, sep10_user.account);

    let response = service
        .add_comment(&id, &sep10_user.account, request)
        .await
        .map_err(|e| GovernanceError::BadRequest(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

// GET /proposals/:id/comments
async fn get_comments(
    State(service): State<Arc<GovernanceService>>,
    Path(id): Path<String>,
    Query(query): Query<CommentsQuery>,
) -> Result<Response, GovernanceError> {
    let limit = query.limit.min(100).max(1);

    let response = service
        .get_comments(&id, limit)
        .await
        .map_err(|e| GovernanceError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

#[derive(Debug)]
pub enum GovernanceError {
    BadRequest(String),
    NotFound(String),
    DatabaseError(String),
}

impl IntoResponse for GovernanceError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            GovernanceError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            GovernanceError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            GovernanceError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(ErrorResponse { error: message });

        (status, body).into_response()
    }
}
