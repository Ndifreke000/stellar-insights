//! Contract Events API Handlers
//!
//! Provides REST API endpoints for querying contract events,
//! verification status, and on-chain audit trails.

use crate::services::event_indexer::{EventIndexer, EventOrderBy, EventQuery, VerificationSummary};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

/// Response for verification summary endpoint
#[derive(Debug, Serialize)]
pub struct VerificationSummaryResponse {
    #[serde(rename = "latestEpoch")]
    pub latest_epoch: Option<u64>,
    #[serde(rename = "latestStatus")]
    pub latest_status: Option<String>,
    #[serde(rename = "latestHash")]
    pub latest_hash: Option<String>,
    #[serde(rename = "latestLedger")]
    pub latest_ledger: Option<u64>,
    #[serde(rename = "latestSubmitted")]
    pub latest_submitted: Option<String>,
    #[serde(rename = "auditTrail")]
    pub audit_trail: Vec<VerificationSummary>,
}

/// Query parameters for event listing
#[derive(Debug, Deserialize)]
pub struct EventListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub event_type: Option<String>,
    pub verification_status: Option<String>,
}

/// Handler for GET /api/analytics/verification-summary
#[utoipa::path(
    get,
    path = "/api/analytics/verification-summary",
    responses(
        (status = 200, description = "Verification summary retrieved", body = VerificationSummaryResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Contract Events"
)]
pub async fn get_verification_summary(
    State(event_indexer): State<Arc<EventIndexer>>,
) -> Result<Json<VerificationSummaryResponse>, (StatusCode, String)> {
    info!("Fetching verification summary");

    let summaries = event_indexer
        .get_verification_summary(10)
        .await
        .map_err(|e| {
            error!("Failed to get verification summary: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch verification summary: {e}"),
            )
        })?;

    let latest = summaries.first();

    let response = VerificationSummaryResponse {
        latest_epoch: latest.map(|s| s.epoch),
        latest_status: latest.map(|s| s.verification_status.clone()),
        latest_hash: latest.and_then(|s| s.hash.clone()),
        latest_ledger: latest.map(|s| s.ledger),
        latest_submitted: latest.map(|s| s.created_at.to_rfc3339()),
        audit_trail: summaries,
    };

    Ok(Json(response))
}

/// Handler for GET /api/analytics/contract-events
#[utoipa::path(
    get,
    path = "/api/analytics/contract-events",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of events to return"),
        ("offset" = Option<i64>, Query, description = "Number of events to skip"),
        ("event_type" = Option<String>, Query, description = "Filter by event type"),
        ("verification_status" = Option<String>, Query, description = "Filter by verification status")
    ),
    responses(
        (status = 200, description = "List of contract events", body = Vec<crate::services::event_indexer::IndexedEvent>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Contract Events"
)]
pub async fn list_contract_events(
    State(event_indexer): State<Arc<EventIndexer>>,
    Query(params): Query<EventListQuery>,
) -> Result<Json<Vec<crate::services::event_indexer::IndexedEvent>>, (StatusCode, String)> {
    info!("Listing contract events with params: {:?}", params);

    let query = EventQuery {
        event_type: params.event_type,
        verification_status: params.verification_status,
        limit: params.limit.or(Some(50)),
        offset: params.offset,
        order_by: Some(EventOrderBy::CreatedAtDesc),
        ..Default::default()
    };

    let events = event_indexer.query_events(query).await.map_err(|e| {
        error!("Failed to query events: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to query events: {e}"),
        )
    })?;

    Ok(Json(events))
}

/// Handler for GET /api/analytics/contract-events/:id
#[utoipa::path(
    get,
    path = "/api/analytics/contract-events/{id}",
    params(
        ("id" = String, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "Contract event details", body = crate::services::event_indexer::IndexedEvent),
        (status = 404, description = "Event not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Contract Events"
)]
pub async fn get_contract_event(
    State(event_indexer): State<Arc<EventIndexer>>,
    Path(id): Path<String>,
) -> Result<Json<crate::services::event_indexer::IndexedEvent>, (StatusCode, String)> {
    info!("Fetching contract event: {}", id);

    let event = event_indexer
        .get_event_by_id(&id)
        .await
        .map_err(|e| {
            error!("Failed to get event: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get event: {e}"),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Event not found: {id}")))?;

    Ok(Json(event))
}

/// Handler for GET /api/analytics/contract-events/epoch/:epoch
#[utoipa::path(
    get,
    path = "/api/analytics/contract-events/epoch/{epoch}",
    params(
        ("epoch" = u64, Path, description = "Epoch number")
    ),
    responses(
        (status = 200, description = "List of events for the epoch", body = Vec<crate::services::event_indexer::IndexedEvent>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Contract Events"
)]
pub async fn get_events_for_epoch(
    State(event_indexer): State<Arc<EventIndexer>>,
    Path(epoch): Path<u64>,
) -> Result<Json<Vec<crate::services::event_indexer::IndexedEvent>>, (StatusCode, String)> {
    info!("Fetching events for epoch: {}", epoch);

    let events = event_indexer
        .get_events_for_epoch(epoch)
        .await
        .map_err(|e| {
            error!("Failed to get events for epoch: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get events for epoch: {e}"),
            )
        })?;

    Ok(Json(events))
}

/// Handler for GET /api/analytics/event-stats
#[utoipa::path(
    get,
    path = "/api/analytics/event-stats",
    responses(
        (status = 200, description = "Event statistics", body = crate::services::event_indexer::EventStats),
        (status = 500, description = "Internal server error")
    ),
    tag = "Contract Events"
)]
pub async fn get_event_stats(
    State(event_indexer): State<Arc<EventIndexer>>,
) -> Result<Json<crate::services::event_indexer::EventStats>, (StatusCode, String)> {
    info!("Fetching event statistics");

    let stats = event_indexer.get_event_stats().await.map_err(|e| {
        error!("Failed to get event stats: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get event stats: {e}"),
        )
    })?;

    Ok(Json(stats))
}

/// Create router with all contract event endpoints
pub fn routes(event_indexer: Arc<EventIndexer>) -> Router {
    Router::new()
        .route(
            "/api/analytics/verification-summary",
            get(get_verification_summary),
        )
        .route("/api/analytics/contract-events", get(list_contract_events))
        .route(
            "/api/analytics/contract-events/:id",
            get(get_contract_event),
        )
        .route(
            "/api/analytics/contract-events/epoch/:epoch",
            get(get_events_for_epoch),
        )
        .route("/api/analytics/event-stats", get(get_event_stats))
        .with_state(event_indexer)
}
