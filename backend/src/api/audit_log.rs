use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::admin_audit_log::AdminAuditLogger;

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogEntryResponse {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub action: String,
    pub resource: String,
    pub user_id: String,
    pub status: String,
    pub ip_address: Option<String>,
    pub session_id: Option<String>,
    pub event_type: Option<String>,
}

/// GET /admin/audit-log - Query audit log with filters
#[utoipa::path(
    get,
    path = "/admin/audit-log",
    params(AuditLogQuery),
    responses(
        (status = 200, description = "Audit log entries returned"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Admin"
)]
pub async fn query_audit_log(
    State(logger): State<Arc<AdminAuditLogger>>,
    Query(params): Query<AuditLogQuery>,
) -> Result<Response, StatusCode> {
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0).max(0);

    let entries = logger
        .query_audit_log(
            params.user_id.as_deref(),
            params.action.as_deref(),
            params.status.as_deref(),
            limit,
            offset,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = json!({
        "entries": entries
            .into_iter()
            .map(|e| json!({
                "id": e.id,
                "timestamp": e.timestamp,
                "action": e.action,
                "resource": e.resource,
                "user_id": e.user_id,
                "status": e.status,
                "ip_address": e.ip_address,
                "session_id": e.session_id,
                "event_type": e.event_type,
            }))
            .collect::<Vec<_>>(),
        "count": entries.len(),
        "limit": limit,
        "offset": offset,
    });

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /admin/audit-log/verify-integrity - Verify audit log integrity
#[utoipa::path(
    post,
    path = "/admin/audit-log/verify-integrity",
    responses(
        (status = 200, description = "Integrity check results"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Admin"
)]
pub async fn verify_audit_log_integrity(
    State(logger): State<Arc<AdminAuditLogger>>,
) -> Result<Response, StatusCode> {
    let result = logger
        .verify_integrity()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = json!({
        "is_valid": result.is_valid,
        "total_entries": result.total_entries,
        "invalid_entries": result.invalid_entries,
        "message": result.message,
    });

    Ok((StatusCode::OK, Json(response)).into_response())
}

pub fn routes(logger: Arc<AdminAuditLogger>) -> Router {
    Router::new()
        .route("/admin/audit-log", get(query_audit_log))
        .route("/admin/audit-log/verify-integrity", axum::routing::post(verify_audit_log_integrity))
        .with_state(logger)
}
