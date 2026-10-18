//! Admin endpoints for contract event backfill.
//!
//! # Endpoints
//!
//! | Method | Path                    | Description                          |
//! |--------|-------------------------|--------------------------------------|
//! | POST   | `/admin/backfill`       | Start a backfill for a ledger range  |
//! | GET    | `/admin/backfill/status`| Poll current backfill progress       |
//! | POST   | `/admin/backfill/resume`| Resume from persisted cursor, gap-check vs head |

use crate::jobs::backfill::{BackfillJob, BackfillRequest, BackfillState};
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info, warn};

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

/// Request body for `POST /admin/backfill/resume`.
///
/// `head_ledger` is the current network head (from RPC/Horizon). `retention`
/// is the number of ledgers the upstream source can still serve; a gap larger
/// than this cannot be backfilled and will raise an alert instead.
#[derive(Debug, Deserialize)]
pub struct ResumeRequest {
    /// Current network head ledger.
    pub head_ledger: u32,
    /// Number of ledgers the upstream source retains (defaults to 17280 ≈ 24h).
    #[serde(default = "default_retention")]
    pub retention: u32,
    /// Optional contract filter forwarded to the backfill job.
    pub contract_id: Option<String>,
    /// Optional per-ledger delay forwarded to the backfill job.
    #[serde(default)]
    pub delay_ms: Option<u64>,
}

fn default_retention() -> u32 {
    17_280
}

/// POST /admin/backfill/resume
///
/// Resumes ingestion from the persisted cursor (`last_ledger`). Compares the
/// cursor against the supplied network head and:
///
/// - if there is no gap, returns `200 OK` with `gap: 0`;
/// - if the gap fits within `retention`, starts a backfill for the gap and
///   returns `202 Accepted`;
/// - if the gap exceeds `retention`, emits an alert and returns `409 Conflict`
///   so operators know the gap cannot be automatically recovered.
///
/// # Request body
///
/// ```json
/// { "head_ledger": 5000, "retention": 17280 }
/// ```
pub async fn resume_backfill(
    State(job): State<Arc<BackfillJob>>,
    Json(req): Json<ResumeRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let last = job.last_ledger().await;
    let from_ledger = last.saturating_add(1);

    if req.head_ledger < from_ledger {
        return Ok((
            StatusCode::OK,
            Json(json!({
                "message": "No gap detected",
                "last_ledger": last,
                "head_ledger": req.head_ledger,
                "gap": 0,
            })),
        ));
    }

    let gap = req.head_ledger - last;
    if gap > req.retention {
        warn!(
            last_ledger = last,
            head_ledger = req.head_ledger,
            gap,
            retention = req.retention,
            "Ledger gap exceeds retention; cannot backfill automatically"
        );
        return Err((
            StatusCode::CONFLICT,
            Json(json!({
                "error": "gap exceeds retention; manual backfill required",
                "last_ledger": last,
                "head_ledger": req.head_ledger,
                "gap": gap,
                "retention": req.retention,
            })),
        ));
    }

    info!(
        last_ledger = last,
        head_ledger = req.head_ledger,
        gap,
        "Backfilling ledger gap from persisted cursor"
    );

    let backfill_req = BackfillRequest {
        from_ledger,
        to_ledger: req.head_ledger,
        contract_id: req.contract_id,
        delay_ms: req.delay_ms,
    };

    job.start(backfill_req).await.map_err(|e| {
        let msg = e.to_string();
        error!(error = %msg, "Failed to resume backfill");

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
            "message": "Gap backfill started",
            "last_ledger": last,
            "head_ledger": req.head_ledger,
            "gap": gap,
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
        .route("/backfill/resume", post(resume_backfill))
        .with_state(job)
}
