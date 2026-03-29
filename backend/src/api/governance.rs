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

use crate::auth::sep10_simple::Sep10Service;
use crate::auth::{sep10_auth_middleware, Sep10User};
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
            sep10_service,
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

const fn default_limit() -> i64 {
    20
}

#[derive(Debug, Deserialize)]
pub struct VotesQuery {
    #[serde(default = "default_votes_limit")]
    pub limit: i64,
}

const fn default_votes_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
pub struct CommentsQuery {
    #[serde(default = "default_comments_limit")]
    pub limit: i64,
}

const fn default_comments_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
pub struct ActivateRequest {
    #[serde(default = "default_voting_duration")]
    pub voting_duration_secs: i64,
}

const fn default_voting_duration() -> i64 {
    604_800 // 7 days
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct HasVotedResponse {
    pub has_voted: bool,
}

// POST /api/governance/proposals - Create a new governance proposal
#[utoipa::path(
    post,
    path = "/api/governance/proposals",
    request_body = CreateProposalRequest,
    responses(
        (status = 201, description = "Proposal created"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Governance"
)]
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

// PUT /api/governance/proposals/:id/activate - Activate a proposal for voting
#[utoipa::path(
    put,
    path = "/api/governance/proposals/{id}/activate",
    params(
        ("id" = String, Path, description = "Proposal ID")
    ),
    request_body = ActivateRequest,
    responses(
        (status = 200, description = "Proposal activated"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Governance"
)]
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

// GET /api/governance/proposals - List all proposals
#[utoipa::path(
    get,
    path = "/api/governance/proposals",
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("limit" = Option<i64>, Query, description = "Maximum results (1-100, default 20)"),
        ("offset" = Option<i64>, Query, description = "Results offset")
    ),
    responses(
        (status = 200, description = "List of proposals")
    ),
    tag = "Governance"
)]
async fn list_proposals(
    State(service): State<Arc<GovernanceService>>,
    Query(query): Query<ListProposalsQuery>,
) -> Result<Response, GovernanceError> {
    let limit = query.limit.clamp(1, 100);
    let offset = query.offset.max(0);

    let response = service
        .list_proposals(query.status.as_deref(), limit, offset)
        .await
        .map_err(|e| GovernanceError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

// GET /api/governance/proposals/:id - Get a specific proposal
#[utoipa::path(
    get,
    path = "/api/governance/proposals/{id}",
    params(
        ("id" = String, Path, description = "Proposal ID")
    ),
    responses(
        (status = 200, description = "Proposal details"),
        (status = 404, description = "Proposal not found")
    ),
    tag = "Governance"
)]
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

// POST /api/governance/proposals/:id/vote - Cast a vote on a proposal
#[utoipa::path(
    post,
    path = "/api/governance/proposals/{id}/vote",
    params(
        ("id" = String, Path, description = "Proposal ID")
    ),
    request_body = CastVoteRequest,
    responses(
        (status = 200, description = "Vote cast successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Governance"
)]
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

// GET /api/governance/proposals/:id/votes - Get votes for a proposal
#[utoipa::path(
    get,
    path = "/api/governance/proposals/{id}/votes",
    params(
        ("id" = String, Path, description = "Proposal ID"),
        ("limit" = Option<i64>, Query, description = "Maximum results (1-100, default 50)")
    ),
    responses(
        (status = 200, description = "List of votes")
    ),
    tag = "Governance"
)]
async fn get_votes(
    State(service): State<Arc<GovernanceService>>,
    Path(id): Path<String>,
    Query(query): Query<VotesQuery>,
) -> Result<Response, GovernanceError> {
    let limit = query.limit.clamp(1, 100);

    let response = service
        .get_votes(&id, limit)
        .await
        .map_err(|e| GovernanceError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

// GET /api/governance/proposals/:id/has-voted/:address - Check if address has voted
#[utoipa::path(
    get,
    path = "/api/governance/proposals/{id}/has-voted/{address}",
    params(
        ("id" = String, Path, description = "Proposal ID"),
        ("address" = String, Path, description = "Stellar address to check")
    ),
    responses(
        (status = 200, description = "Voting status", body = HasVotedResponse)
    ),
    tag = "Governance"
)]
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

// POST /api/governance/proposals/:id/comments - Add a comment to a proposal
#[utoipa::path(
    post,
    path = "/api/governance/proposals/{id}/comments",
    params(
        ("id" = String, Path, description = "Proposal ID")
    ),
    request_body = AddCommentRequest,
    responses(
        (status = 201, description = "Comment added"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Governance"
)]
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

// GET /api/governance/proposals/:id/comments - Get comments for a proposal
#[utoipa::path(
    get,
    path = "/api/governance/proposals/{id}/comments",
    params(
        ("id" = String, Path, description = "Proposal ID"),
        ("limit" = Option<i64>, Query, description = "Maximum results (1-100, default 50)")
    ),
    responses(
        (status = 200, description = "List of comments")
    ),
    tag = "Governance"
)]
async fn get_comments(
    State(service): State<Arc<GovernanceService>>,
    Path(id): Path<String>,
    Query(query): Query<CommentsQuery>,
) -> Result<Response, GovernanceError> {
    let limit = query.limit.clamp(1, 100);

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
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(ErrorResponse { error: message });

        (status, body).into_response()
    }
}
