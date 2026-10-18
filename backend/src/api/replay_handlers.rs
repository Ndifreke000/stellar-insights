//! API Handlers for Contract Event Replay System
//!
//! Mounted at `/api/v1/replay` (see `routes()` below and
//! `crate::api::v1::routes`) behind the same `auth_middleware` the rest of
//! the mutating "protected" routes use — starting, deleting or bulk-
//! cleaning up replay sessions is an operational/debugging action, not a
//! public read.
//!
//! These handlers previously existed but were never mounted into any
//! router (see issue #2660): the whole module was unreachable dead code.
//! Wiring them up surfaced two bugs that were presumably why no one had:
//! 1. Every handler took `State<Arc<AppState>>`, but `AppState` (see
//!    `crate::state`) is already `Clone` and used bare (`State<AppState>`)
//!    everywhere else in this codebase — `Arc<AppState>` can't be
//!    extracted from a router built with `.with_state(app_state)`, so this
//!    would have failed to compile the moment anyone tried to route to it.
//! 2. `start_replay` was a plain `fn`, not `async fn`, so it doesn't
//!    implement `axum::Handler` (which requires the handler to return a
//!    `Future`) despite being used exactly like the other, genuinely async
//!    handlers below it.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
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
        EventFilter,
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

/// Query parameters for checkpoint cleanup.
///
/// `cleanup_checkpoints` used to extract this as a bare `Query<i64>`, which
/// `serde_urlencoded` (what axum's `Query` extractor uses under the hood)
/// cannot deserialize a query string like `?days=30` into — it needs a
/// struct/map target, the same reason `ListReplaysQuery` above exists for
/// `list_replays` rather than a bare `Query<usize>`.
#[derive(Debug, Deserialize)]
pub struct CleanupCheckpointsQuery {
    pub days: i64,
}

/// Start a new replay
pub async fn start_replay(
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
    Query(params): Query<CleanupCheckpointsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    info!(
        "Cleaning up checkpoints older than {} days",
        params.days
    );

    let checkpoint_manager = CheckpointManager::new(state.db.pool().clone());

    let deleted = checkpoint_manager
        .cleanup_old(params.days)
        .await
        .map_err(|e| ApiError::internal("INTERNAL_ERROR", e.to_string()))?;

    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "message": format!("Deleted {} old checkpoints", deleted)
    })))
}

/// Replay control routes, mounted at `/api/v1/replay` by
/// `crate::api::v1::routes` behind `auth_middleware` (see that call site for
/// why: these start background jobs and delete data, so they get the same
/// gate as the rest of the "protected" write routes).
pub fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/", post(start_replay).get(list_replays))
        .route("/{session_id}", get(get_replay_status).delete(delete_replay))
        .route("/{session_id}/checkpoints", get(list_checkpoints))
        .route("/checkpoints/cleanup", delete(cleanup_checkpoints))
        .with_state(app_state)
}
