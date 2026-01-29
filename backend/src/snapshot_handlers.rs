//! HTTP handlers for snapshot generation and submission

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::database::Database;
use crate::services::contract::ContractService;
use crate::services::snapshot::SnapshotService;
use crate::snapshot::schema::AnalyticsSnapshot;

/// Response for snapshot generation
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
pub struct SubmissionInfo {
    pub transaction_hash: String,
    pub ledger: u64,
    pub contract_timestamp: u64,
}

/// Request for snapshot generation
#[derive(Debug, Deserialize)]
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
}

/// Generate a snapshot (optionally submit to contract)
/// 
/// POST /api/snapshots/generate
pub async fn generate_snapshot(
    State(state): State<SnapshotAppState>,
    Json(request): Json<GenerateSnapshotRequest>,
) -> Result<Json<SnapshotResponse>, SnapshotError> {
    info!(
        "Generating snapshot for epoch {} (submit: {})",
        request.epoch, request.submit_to_contract
    );

    // Create a snapshot with current timestamp
    // In a real implementation, this would fetch metrics from the database
    let snapshot = AnalyticsSnapshot::new(request.epoch, Utc::now());
    
    // TODO: Populate snapshot with actual metrics from database
    // Example:
    // let anchors = state.db.get_all_anchors().await?;
    // for anchor in anchors {
    //     let metrics = compute_anchor_metrics(&anchor).await?;
    //     snapshot.add_anchor_metrics(metrics);
    // }

    let anchor_count = snapshot.anchor_metrics.len();
    let corridor_count = snapshot.corridor_metrics.len();
    let timestamp = snapshot.timestamp.to_rfc3339();

    // Determine if we should submit to contract
    let should_submit = request.submit_to_contract && state.contract_service.is_some();

    let (_hash_bytes, hash_hex, version, submission) = if should_submit {
        let contract_service = state
            .contract_service
            .as_ref()
            .expect("Contract service should be available");

        // Hash and submit to contract
        let (hash_bytes, hash_hex, version, submission_result) =
            SnapshotService::version_hash_and_submit(snapshot, contract_service.as_ref())
                .await
                .map_err(|e| {
                    error!("Failed to submit snapshot: {}", e);
                    SnapshotError::SubmissionError(e.to_string())
                })?;

        let submission = Some(SubmissionInfo {
            transaction_hash: submission_result.transaction_hash,
            ledger: submission_result.ledger,
            contract_timestamp: submission_result.timestamp,
        });

        (hash_bytes, hash_hex, version, submission)
    } else {
        // Just hash without submitting
        let (hash_bytes, hash_hex, version) = SnapshotService::version_and_hash(snapshot)
            .map_err(|e| {
                error!("Failed to hash snapshot: {}", e);
                SnapshotError::HashingError(e.to_string())
            })?;

        (hash_bytes, hash_hex, version, None)
    };

    info!(
        "✓ Snapshot generated for epoch {} (hash: {}, anchors: {}, corridors: {})",
        request.epoch, hash_hex, anchor_count, corridor_count
    );

    if submission.is_some() {
        info!("✓ Snapshot submitted to contract successfully");
    }

    Ok(Json(SnapshotResponse {
        epoch: request.epoch,
        timestamp,
        hash: hash_hex,
        schema_version: version,
        anchor_count,
        corridor_count,
        submission,
    }))
}

/// Health check for contract service
/// 
/// GET /api/snapshots/contract/health
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

#[derive(Debug, Serialize)]
pub struct ContractHealthResponse {
    pub status: &'static str,
    pub timestamp: String,
}

/// Error types for snapshot operations
#[derive(Debug)]
pub enum SnapshotError {
    GenerationError(String),
    HashingError(String),
    SubmissionError(String),
    ConnectionError(String),
    ConfigError(String),
}

impl IntoResponse for SnapshotError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            SnapshotError::GenerationError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            SnapshotError::HashingError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            SnapshotError::SubmissionError(msg) => (StatusCode::BAD_GATEWAY, msg),
            SnapshotError::ConnectionError(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            SnapshotError::ConfigError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
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
