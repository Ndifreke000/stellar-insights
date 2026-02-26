//! API Handlers for Contract Event Replay System

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::{
    error::ApiError,
    replay::{
        checkpoint::CheckpointManager,
        config::{ReplayConfig, ReplayMode, ReplayRange},
        engine::ReplayEngine,
        event_processor::CompositeEventProcessor,
        state_builder::StateBuilder,
        storage::{EventStorage, ReplayStorage},
        EventFilter, ReplayMetadata,
    },
    state::AppState,
};

/// Request to start a replay
#[derive(Debug, Deserialize)]
pub struct StartReplayRequest {
    /// Replay mode
    pub mode: Option<String>,
    /// Start ledger
    pub start_ledger: Option<u64>,
    /// End ledger
    pub end_ledger: Option<u64>,
    /// Checkpoint ID to resume from
    pub checkpoint_id: Option<String>,
    /// Contract IDs to filter
    pub contract_ids: Option<Vec<String>>,
    /// Event types to filter
    pub event_types: Option<Vec<String>>,
    /// Network filter
    pub network: Option<String>,
    /// Batch size
    pub batch_size: Option<usize>,
    /// Dry run mode
    pub dry_run: Option<bool>,
    /// Verbose logging
    pub verbose: Option<bool>,
}

/// Response for replay operations
#[derive(Debug, Serialize)]
pub struct ReplayResponse {
    pub session_id: String,
    pub status: String,
    pub message: String,
}

/// Query parameters for listing replays
#[derive(Debug, Deserialize)]
pub struct ListReplaysQuery {
    pub limit: Option<usize>,
}

/// Start a new replay
pub async fn start_replay(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartReplayRequest>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Starting replay with request: {:?}", req);

    // Parse replay mode
    let mode = match req.mode.as_deref() {
        Some("full") => ReplayMode::Full,
        Some("incremental") => ReplayMode::Incremental,
        Some("verification") => ReplayMode::Verification,
        Some("debug") => ReplayMode::Debug,
        _ => ReplayMode::Full,
    };

    // Determine replay range
    let range = if let Some(checkpoint_id) = req.checkpoint_id {
        ReplayRange::FromCheckpoint { checkpoint_id }
    } else if let (Some(start), Some(end)) = (req.start_ledger, req.end_ledger) {
        ReplayRange::FromTo { start, end }
    } else if let Some(start) = req.start_ledger {
        ReplayRange::From { start }
    } else if let Some(end) = req.end_ledger {
        ReplayRange::To { end }
    } else {
        ReplayRange::All
    };

    // Create event filter
    let filter = EventFilter {
        contract_ids: req.contract_ids,
        event_types: req.event_types,
        network: req.network,
    };

    // Build configuration
    let mut config = ReplayConfig::new()
        .with_mode(mode)
        .with_range(range)
        .with_filter(filter);

    if let Some(batch_size) = req.batch_size {
        config = config.with_batch_size(batch_size);
    }

    if req.dry_run.unwrap_or(false) {
        config = config.dry_run();
    }

    if req.verbose.unwrap_or(false) {
        config = config.verbose();
    }

    // Create replay components
    let event_storage = Arc::new(EventStorage::new(state.db.pool().clone()));
    let replay_storage = Arc::new(ReplayStorage::new(state.db.pool().clone()));
    let checkpoint_manager = Arc::new(CheckpointManager::new(state.db.pool().clone()));
    let processor = Arc::new(CompositeEventProcessor::new());
    let state_builder = Arc::new(tokio::sync::RwLock::new(StateBuilder::new(
        state.db.pool().clone(),
    )));

    // Create replay engine
    let engine = ReplayEngine::new(
        config,
        event_storage,
        replay_storage,
        checkpoint_manager,
        processor,
        state_builder,
    )
    .map_err(|e| ApiError::internal("INTERNAL_ERROR", e.to_string()))?;

    // Start replay in background
    let engine_clone = Arc::new(engine);
    tokio::spawn(async move {
        match engine_clone.start().await {
            Ok(metadata) => {
                info!("Replay completed: {:?}", metadata);
            }
            Err(e) => {
                error!("Replay failed: {}", e);
            }
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(ReplayResponse {
            session_id: "replay-session".to_string(), // Would use actual session ID
            status: "started".to_string(),
            message: "Replay started successfully".to_string(),
        }),
    ))
}

/// Get replay status
pub async fn get_replay_status(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Getting replay status for session: {}", session_id);

    let replay_storage = ReplayStorage::new(state.db.pool().clone());

    let metadata = replay_storage
        .load_metadata(&session_id)
        .await
        .map_err(|e| ApiError::internal("INTERNAL_ERROR", e.to_string()))?
        .ok_or_else(|| ApiError::not_found("NOT_FOUND", "Replay session not found".to_string()))?;

    Ok(Json(metadata))
}

/// List all replay sessions
pub async fn list_replays(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListReplaysQuery>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Listing replay sessions");

    let replay_storage = ReplayStorage::new(state.db.pool().clone());

    let sessions = replay_storage
        .list_sessions(query.limit)
        .await
        .map_err(|e| ApiError::internal("INTERNAL_ERROR", e.to_string()))?;

    Ok(Json(sessions))
}

/// Get checkpoints for a session
pub async fn list_checkpoints(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Listing checkpoints for session: {}", session_id);

    let checkpoint_manager = CheckpointManager::new(state.db.pool().clone());

    let checkpoints = checkpoint_manager
        .list_for_session(&session_id)
        .await
        .map_err(|e| ApiError::internal("INTERNAL_ERROR", e.to_string()))?;

    Ok(Json(checkpoints))
}

/// Delete a replay session
pub async fn delete_replay(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Deleting replay session: {}", session_id);

    let replay_storage = ReplayStorage::new(state.db.pool().clone());

    replay_storage
        .delete_session(&session_id)
        .await
        .map_err(|e| ApiError::internal("INTERNAL_ERROR", e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(ReplayResponse {
            session_id,
            status: "deleted".to_string(),
            message: "Replay session deleted successfully".to_string(),
        }),
    ))
}

/// Cleanup old checkpoints
pub async fn cleanup_checkpoints(
    State(state): State<Arc<AppState>>,
    Query(days): Query<i64>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Cleaning up checkpoints older than {} days", days);

    let checkpoint_manager = CheckpointManager::new(state.db.pool().clone());

    let deleted = checkpoint_manager
        .cleanup_old(days)
        .await
        .map_err(|e| ApiError::internal("INTERNAL_ERROR", e.to_string()))?;

    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "message": format!("Deleted {} old checkpoints", deleted)
    })))
}
