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
use crate::services::snapshot::{SnapshotService, SnapshotGenerationResult};
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
    pub snapshot_service: Arc<SnapshotService>,
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

    // Use the comprehensive snapshot service to handle all requirements
    match state.snapshot_service.generate_and_submit_snapshot(request.epoch).await {
        Ok(result) => {
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
                result.hash,
                result.anchor_count,
                result.corridor_count,
                result.verification_successful
            );

            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to generate snapshot for epoch {}: {}", request.epoch, e);
            Err(SnapshotError::GenerationFailed(e.to_string()))
        }
    }
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
            SnapshotError::GenerationFailed(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
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
