//! Admin endpoints for contract event backfill.
//!
//! # Endpoints
//!
//! | Method | Path                    | Description                          |
//! |--------|-------------------------|--------------------------------------|
//! | POST   | `/admin/backfill`       | Start a backfill for a ledger range  |
//! | GET    | `/admin/backfill/status`| Poll current backfill progress       |

use crate::jobs::backfill::{BackfillJob, BackfillRequest, BackfillState};
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};

/// POST /admin/backfill
///
/// Starts a backfill job for the given ledger range. Returns immediately;
/// poll `/admin/backfill/status` to track progress.
///
/// # Request body
///
/// ```json
/// {
///   "from_ledger": 1000,
///   "to_ledger":   2000,
///   "contract_id": "optional-contract-id",
///   "delay_ms":    250
/// }
/// ```
///
/// # Responses
///
/// - `202 Accepted` — job started successfully
/// - `400 Bad Request` — invalid range or a job is already running
/// - `500 Internal Server Error` — unexpected failure
pub async fn start_backfill(
    State(job): State<Arc<BackfillJob>>,
    Json(req): Json<BackfillRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    info!(
        from = req.from_ledger,
        to = req.to_ledger,
        "Received backfill request"
    );

    job.start(req).await.map_err(|e| {
        let msg = e.to_string();
        error!(error = %msg, "Failed to start backfill");

        let status = if msg.contains("already in progress")
            || msg.contains("must be <=")
            || msg.contains("exceeds maximum")
        {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        (status, Json(json!({ "error": msg })))
    })?;

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "message": "Backfill started",
            "status_url": "/admin/backfill/status"
        })),
    ))
}

/// GET /admin/backfill/status
///
/// Returns the current (or last completed) backfill state.
///
/// # Response
///
/// ```json
/// {
///   "status": "running",
///   "from_ledger": 1000,
///   "to_ledger": 2000,
///   "current_ledger": 1450,
///   "events_indexed": 312,
///   "ledgers_processed": 450,
///   "ledgers_total": 1001,
///   "gaps_detected": 3,
///   "started_at": "2024-01-01T00:00:00Z",
///   "finished_at": null,
///   "error": null
/// }
/// ```
pub async fn get_backfill_status(State(job): State<Arc<BackfillJob>>) -> Json<BackfillState> {
    Json(job.status().await)
}

/// Build the admin backfill router.
///
/// Mount this at `/admin` in the main router:
///
/// ```rust,ignore
/// app.nest("/admin", backfill::routes(backfill_job));
/// ```
pub fn routes(job: Arc<BackfillJob>) -> Router {
    Router::new()
        .route("/backfill", post(start_backfill))
        .route("/backfill/status", get(get_backfill_status))
        .with_state(job)
}
