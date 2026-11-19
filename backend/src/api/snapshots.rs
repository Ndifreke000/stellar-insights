//! HTTP handlers for snapshot generation and submission

use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{error, info, warn};
use utoipa::ToSchema;

use crate::database::Database;
use crate::error::ApiError;
use crate::services::contract::ContractService;
use crate::services::snapshot::SnapshotService;
use crate::snapshot::verify::{
    build_report, load_stored_snapshot, OnChainLookup, SnapshotRef, StoredSnapshot,
    VerificationReport,
};

/// Response for snapshot generation
#[derive(Debug, Serialize, ToSchema)]
pub struct SnapshotResponse {
    pub epoch: u64,
    pub timestamp: String,
    pub hash: String,
    pub schema_version: u32,
    pub anchor_count: usize,
    pub corridor_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission: Option<SubmissionInfo>,
}

/// Submission information
#[derive(Debug, Serialize, ToSchema)]
pub struct SubmissionInfo {
    pub transaction_hash: String,
    pub ledger: u64,
    pub contract_timestamp: u64,
}

/// Request for snapshot generation
#[derive(Debug, Deserialize, ToSchema)]
pub struct GenerateSnapshotRequest {
    pub epoch: u64,
    #[serde(default)]
    pub submit_to_contract: bool,
}

/// Shared application state for snapshot handlers
#[derive(Clone)]
pub struct SnapshotAppState {
    pub db: Arc<Database>,
    pub contract_service: Option<Arc<ContractService>>,
    pub snapshot_service: Arc<SnapshotService>,
}

/// Generate a snapshot (optionally submit to contract)
///
/// POST /api/snapshots/generate
#[utoipa::path(
    post,
    path = "/api/snapshots/generate",
    request_body = GenerateSnapshotRequest,
    responses(
        (status = 200, description = "Snapshot generated successfully", body = SnapshotResponse),
        (status = 500, description = "Snapshot generation failed")
    ),
    tag = "Snapshots"
)]
pub async fn generate_snapshot(
    State(state): State<SnapshotAppState>,
    Json(request): Json<GenerateSnapshotRequest>,
) -> Result<Json<SnapshotResponse>, SnapshotError> {
    info!(
        "Generating snapshot for epoch {} (submit: {})",
        request.epoch, request.submit_to_contract
    );

    // Use the comprehensive snapshot service to handle all requirements
    match state
        .snapshot_service
        .generate_and_submit_snapshot(request.epoch)
        .await
    {
        Ok(result) => {
            let hash = result.hash.clone();
            let response = SnapshotResponse {
                epoch: result.epoch,
                timestamp: result.timestamp.to_rfc3339(),
                hash: result.hash,
                schema_version: 1, // From SCHEMA_VERSION
                anchor_count: result.anchor_count,
                corridor_count: result.corridor_count,
                submission: result.submission_result.map(|sr| SubmissionInfo {
                    transaction_hash: sr.transaction_hash,
                    ledger: sr.ledger,
                    contract_timestamp: sr.timestamp,
                }),
            };

            info!(
                "Successfully generated snapshot for epoch {}: hash={}, anchors={}, corridors={}, submitted={}",
                result.epoch,
                hash,
                result.anchor_count,
                result.corridor_count,
                result.verification_successful
            );

            Ok(Json(response))
        }
        Err(e) => {
            error!(
                "Failed to generate snapshot for epoch {}: {}",
                request.epoch, e
            );
            Err(SnapshotError::GenerationFailed(e.to_string()))
        }
    }
}

/// Health check for contract service
///
/// GET /api/snapshots/contract/health
#[utoipa::path(
    get,
    path = "/api/snapshots/contract/health",
    responses(
        (status = 200, description = "Contract service health status", body = ContractHealthResponse),
        (status = 500, description = "Config error"),
        (status = 503, description = "Connection error")
    ),
    tag = "Snapshots"
)]
pub async fn contract_health_check(
    State(state): State<SnapshotAppState>,
) -> Result<Json<ContractHealthResponse>, SnapshotError> {
    let contract_service = state
        .contract_service
        .as_ref()
        .ok_or_else(|| SnapshotError::ConfigError("Contract service not configured".to_string()))?;

    let is_healthy = contract_service
        .health_check()
        .await
        .map_err(|e| SnapshotError::ConnectionError(e.to_string()))?;

    Ok(Json(ContractHealthResponse {
        status: if is_healthy { "healthy" } else { "unhealthy" },
        timestamp: Utc::now().to_rfc3339(),
    }))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ContractHealthResponse {
    pub status: &'static str,
    pub timestamp: String,
}

/// Error types for snapshot operations
#[derive(Debug)]
pub enum SnapshotError {
    GenerationFailed(String),
    GenerationError(String),
    HashingError(String),
    SubmissionError(String),
    ConnectionError(String),
    ConfigError(String),
}

impl IntoResponse for SnapshotError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            Self::GenerationFailed(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::GenerationError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::HashingError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::SubmissionError(msg) => (StatusCode::BAD_GATEWAY, msg),
            Self::ConnectionError(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            Self::ConfigError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (
            status,
            Json(serde_json::json!({
                "error": message,
                "timestamp": Utc::now().to_rfc3339()
            })),
        )
            .into_response()
    }
}

// ---------------------------------------------------------------------------
// Public verification (#2597)
// ---------------------------------------------------------------------------

/// State for the public, read-only snapshot routes.
#[derive(Clone)]
pub struct SnapshotVerifyState {
    pub pool: SqlitePool,
    /// `None` when `SNAPSHOT_CONTRACT_ID` is unset; verification then reports
    /// `chain_unavailable` instead of failing.
    pub contract: Option<Arc<ContractService>>,
}

/// Public routes, mounted at `/api/v1/snapshots`. No auth: verification is
/// meant to be checkable by anyone, and both routes are read-only.
pub fn verify_routes(pool: SqlitePool) -> Router {
    let contract = match ContractService::read_only_from_env() {
        Ok(service) => Some(Arc::new(service)),
        Err(e) => {
            warn!("Snapshot verification will not check the chain: {e}");
            None
        }
    };
    verify_routes_with_state(SnapshotVerifyState { pool, contract })
}

pub fn verify_routes_with_state(state: SnapshotVerifyState) -> Router {
    Router::new()
        .route("/{id}", get(get_snapshot_data))
        .route("/{id}/verify", get(verify_snapshot))
        .with_state(state)
}

async fn find_snapshot(pool: &SqlitePool, raw_id: &str) -> Result<StoredSnapshot, ApiError> {
    let snapshot_ref = SnapshotRef::parse(raw_id)
        .map_err(|e| ApiError::bad_request("INVALID_SNAPSHOT_ID", e.0))?;
    load_stored_snapshot(pool, &snapshot_ref)
        .await?
        .ok_or_else(|| ApiError::not_found("SNAPSHOT_NOT_FOUND", "Snapshot not found"))
}

/// Canonical JSON of a snapshot, byte-for-byte as it was hashed.
///
/// GET /api/v1/snapshots/{id}
///
/// Third parties hash this body themselves (`sha256`) and compare it with the
/// on-chain record; see `docs/SNAPSHOT_ANCHORING.md`.
#[utoipa::path(
    get,
    path = "/api/v1/snapshots/{id}",
    params(("id" = String, Path, description = "Epoch number or snapshot UUID")),
    responses(
        (status = 200, description = "Canonical snapshot JSON", content_type = "application/json"),
        (status = 400, description = "Invalid snapshot id"),
        (status = 404, description = "Snapshot not found")
    ),
    tag = "Snapshots"
)]
pub async fn get_snapshot_data(
    State(state): State<SnapshotVerifyState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let stored = find_snapshot(&state.pool, &id).await?;

    // Return the stored bytes untouched: re-serialising through serde would
    // change the hash.
    let mut response = (StatusCode::OK, stored.data).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Ok(v) = HeaderValue::from_str(&stored.hash) {
        headers.insert("x-snapshot-hash", v);
    }
    if let Ok(v) = HeaderValue::from_str(&stored.epoch.to_string()) {
        headers.insert("x-snapshot-epoch", v);
    }
    Ok(response)
}

/// Verify a stored snapshot against its recomputed hash and on-chain record.
///
/// GET /api/v1/snapshots/{id}/verify
///
/// Always `200` once the snapshot exists: the verdict is in `verified` and
/// `status`, so a tampered or unanchored snapshot is a result, not an error.
#[utoipa::path(
    get,
    path = "/api/v1/snapshots/{id}/verify",
    params(("id" = String, Path, description = "Epoch number or snapshot UUID")),
    responses(
        (status = 200, description = "Verification result", body = VerificationReport),
        (status = 400, description = "Invalid snapshot id"),
        (status = 404, description = "Snapshot not found")
    ),
    tag = "Snapshots"
)]
pub async fn verify_snapshot(
    State(state): State<SnapshotVerifyState>,
    Path(id): Path<String>,
) -> Result<Json<VerificationReport>, ApiError> {
    let stored = find_snapshot(&state.pool, &id).await?;

    let lookup = match &state.contract {
        None => OnChainLookup::NotConfigured,
        Some(contract) => match contract.get_snapshot_record(stored.epoch).await {
            Ok(Some(record)) => OnChainLookup::Found(record),
            Ok(None) => OnChainLookup::NotAnchored,
            Err(e) => {
                warn!("On-chain lookup failed for epoch {}: {e}", stored.epoch);
                OnChainLookup::Unavailable(e.to_string())
            }
        },
    };

    let report = build_report(&stored, lookup);
    if !report.checks.stored_hash_matches_data {
        warn!(
            "Snapshot {} (epoch {}) failed integrity check: stored={} recomputed={}",
            report.snapshot_id, report.epoch, report.stored_hash, report.recomputed_hash
        );
    }
    Ok(Json(report))
}
