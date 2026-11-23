use crate::database::Database;
use crate::models::{ApiAnalyticsOverview, EndpointStat, StatusStat};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// GET /api/admin/analytics/overview - Get API analytics overview
#[utoipa::path(
    get,
    path = "/api/admin/analytics/overview",
    responses(
        (status = 200, description = "API analytics overview", body = ApiAnalyticsOverview),
        (status = 500, description = "Internal server error")
    ),
    tag = "Analytics"
)]
#[allow(clippy::cast_precision_loss)]
pub async fn get_analytics_overview(State(db): State<Arc<Database>>) -> Json<ApiAnalyticsOverview> {
    // 1. Total Requests
    let total_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_usage_stats")
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);

    // 2. Avg Response Time
    let avg_response_time_ms: f64 =
        sqlx::query_scalar("SELECT AVG(response_time_ms) FROM api_usage_stats")
            .fetch_one(db.pool())
            .await
            .unwrap_or(0.0);

    // 3. Error Rate (4xx and 5xx)
    let error_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM api_usage_stats WHERE status_code >= 400")
            .fetch_one(db.pool())
            .await
            .unwrap_or(0);

    let error_rate = if total_requests > 0 {
        (error_count as f64 / total_requests as f64) * 100.0
    } else {
        0.0
    };

    // 4. Top Endpoints
    let top_endpoints = sqlx::query_as::<_, EndpointStat>(
        "SELECT endpoint, method, COUNT(*) as count, AVG(response_time_ms) as avg_response_time_ms
         FROM api_usage_stats
         GROUP BY endpoint, method
         ORDER BY count DESC
         LIMIT 10",
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();

    // 5. Status Distribution
    let status_distribution = sqlx::query_as::<_, StatusStat>(
        "SELECT status_code, COUNT(*) as count
         FROM api_usage_stats
         GROUP BY status_code
         ORDER BY count DESC",
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();

    Json(ApiAnalyticsOverview {
        total_requests,
        avg_response_time_ms,
        error_rate,
        top_endpoints,
        status_distribution,
    })
}

/// Default retention window for sampled request logs (in days).
pub const DEFAULT_LOG_RETENTION_DAYS: i64 = 30;

/// A single sampled, redacted request log entry owned by an API key.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct RequestLogEntry {
    pub id: i64,
    pub api_key_id: String,
    pub method: String,
    pub route: String,
    pub status_code: i64,
    pub latency_ms: f64,
    pub created_at: String,
}

/// Aggregated usage for a single route.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct RouteAggregate {
    pub route: String,
    pub method: String,
    pub count: i64,
    pub avg_latency_ms: f64,
    pub error_count: i64,
}

/// Aggregated usage for a single status code.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct StatusAggregate {
    pub status_code: i64,
    pub count: i64,
}

/// Filters accepted by the developer portal request-log endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct RequestLogFilters {
    /// Route filter (exact match).
    pub route: Option<String>,
    /// Status code filter (exact match).
    pub status_code: Option<i64>,
    /// Only include logs created at or after this RFC3339 timestamp.
    pub since: Option<String>,
    /// Only include logs created at or before this RFC3339 timestamp.
    pub until: Option<String>,
    /// Max rows to return (defaults to 100, capped at 500).
    pub limit: Option<i64>,
}

/// Response payload for the per-key request log listing.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RequestLogsResponse {
    pub api_key_id: String,
    pub logs: Vec<RequestLogEntry>,
}

/// Response payload for the per-key aggregates.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RequestLogAggregates {
    pub api_key_id: String,
    pub by_route: Vec<RouteAggregate>,
    pub by_status: Vec<StatusAggregate>,
}

/// Standard error shape used by the developer portal endpoints.
#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub error: String,
}

/// Resolve the API key id from the request headers. The middleware attaches the
/// authenticated key id so logs are only ever visible to the key owner.
fn api_key_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-api-key-id")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

/// GET /api/developer/analytics/logs - Sampled, redacted request logs for the
/// authenticated API key, scoped to the key owner.
#[utoipa::path(
    get,
    path = "/api/developer/analytics/logs",
    responses(
        (status = 200, description = "Sampled request logs", body = RequestLogsResponse),
        (status = 401, description = "Missing API key"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Analytics"
)]
pub async fn get_request_logs(
    State(db): State<Arc<Database>>,
    headers: axum::http::HeaderMap,
    Query(filters): Query<RequestLogFilters>,
) -> Result<Json<RequestLogsResponse>, (StatusCode, Json<ApiError>)> {
    let api_key_id = api_key_from_headers(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "missing api key".to_string(),
            }),
        )
    })?;

    let limit = filters.limit.unwrap_or(100).clamp(1, 500);

    let logs = sqlx::query_as::<_, RequestLogEntry>(
        "SELECT id, api_key_id, method, route, status_code, latency_ms, created_at
         FROM api_request_logs
         WHERE api_key_id = ?
           AND (? IS NULL OR route = ?)
           AND (? IS NULL OR status_code = ?)
           AND (? IS NULL OR created_at >= ?)
           AND (? IS NULL OR created_at <= ?)
         ORDER BY created_at DESC
         LIMIT ?",
    )
    .bind(&api_key_id)
    .bind(&filters.route)
    .bind(&filters.route)
    .bind(filters.status_code)
    .bind(filters.status_code)
    .bind(&filters.since)
    .bind(&filters.since)
    .bind(&filters.until)
    .bind(&filters.until)
    .bind(limit)
    .fetch_all(db.pool())
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(RequestLogsResponse { api_key_id, logs }))
}

/// GET /api/developer/analytics/aggregates - Per-key aggregates by route and
/// status code, scoped to the authenticated key owner.
#[utoipa::path(
    get,
    path = "/api/developer/analytics/aggregates",
    responses(
        (status = 200, description = "Aggregates by route and status", body = RequestLogAggregates),
        (status = 401, description = "Missing API key"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Analytics"
)]
pub async fn get_request_log_aggregates(
    State(db): State<Arc<Database>>,
    headers: axum::http::HeaderMap,
    Query(filters): Query<RequestLogFilters>,
) -> Result<Json<RequestLogAggregates>, (StatusCode, Json<ApiError>)> {
    let api_key_id = api_key_from_headers(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "missing api key".to_string(),
            }),
        )
    })?;

    let by_route = sqlx::query_as::<_, RouteAggregate>(
        "SELECT route, method, COUNT(*) as count,
                AVG(latency_ms) as avg_latency_ms,
                SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) as error_count
         FROM api_request_logs
         WHERE api_key_id = ?
           AND (? IS NULL OR route = ?)
           AND (? IS NULL OR status_code = ?)
           AND (? IS NULL OR created_at >= ?)
           AND (? IS NULL OR created_at <= ?)
         GROUP BY route, method
         ORDER BY count DESC",
    )
    .bind(&api_key_id)
    .bind(&filters.route)
    .bind(&filters.route)
    .bind(filters.status_code)
    .bind(filters.status_code)
    .bind(&filters.since)
    .bind(&filters.since)
    .bind(&filters.until)
    .bind(&filters.until)
    .fetch_all(db.pool())
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    let by_status = sqlx::query_as::<_, StatusAggregate>(
        "SELECT status_code, COUNT(*) as count
         FROM api_request_logs
         WHERE api_key_id = ?
           AND (? IS NULL OR route = ?)
           AND (? IS NULL OR status_code = ?)
           AND (? IS NULL OR created_at >= ?)
           AND (? IS NULL OR created_at <= ?)
         GROUP BY status_code
         ORDER BY count DESC",
    )
    .bind(&api_key_id)
    .bind(&filters.route)
    .bind(&filters.route)
    .bind(filters.status_code)
    .bind(filters.status_code)
    .bind(&filters.since)
    .bind(&filters.since)
    .bind(&filters.until)
    .bind(&filters.until)
    .fetch_all(db.pool())
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(RequestLogAggregates {
        api_key_id,
        by_route,
        by_status,
    }))
}

/// Prune sampled request logs older than the retention window. Returns the
/// number of rows removed. Safe to call periodically (e.g. from a cron task).
pub async fn prune_request_logs(db: &Database, retention_days: i64) -> Result<u64, sqlx::Error> {
    let days = retention_days.max(1);
    let result = sqlx::query(
        "DELETE FROM api_request_logs
         WHERE created_at < datetime('now', ?)",
    )
    .bind(format!("-{days} days"))
    .execute(db.pool())
    .await?;

    Ok(result.rows_affected())
}

pub fn routes(db: Arc<Database>) -> Router {
    Router::new()
        .route("/api/admin/analytics/overview", get(get_analytics_overview))
        .route("/api/developer/analytics/logs", get(get_request_logs))
        .route(
            "/api/developer/analytics/aggregates",
            get(get_request_log_aggregates),
        )
        .with_state(db)
}
