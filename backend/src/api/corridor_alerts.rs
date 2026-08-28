//! Corridor Performance Alert API handlers.
//!
//! Provides endpoints for managing corridor-specific alert configurations,
//! viewing performance snapshots, and tracking alert events.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use std::sync::Arc;

use crate::{
    auth_middleware::AuthUser,
    error::{ApiError, ApiResult},
    models::corridor_alerts::{
        CorridorAlertConfig, CorridorAlertEvent, CorridorPerformanceSnapshot,
        CorridorPerformanceSummary, CorridorPerformanceTimeline,
        CreateCorridorAlertConfigRequest, UpdateCorridorAlertConfigRequest,
    },
    services::corridor_performance_monitor::CorridorPerformanceMonitor,
    state::AppState,
    validation::ValidatedJson,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // Corridor alert config management
        .route("/configs", get(list_configs).post(create_config))
        .route(
            "/configs/{id}",
            get(get_config).put(update_config).delete(delete_config),
        )
        // Corridor performance snapshots
        .route("/snapshots", get(list_all_latest_snapshots))
        .route("/snapshots/{corridor_key}", get(get_corridor_snapshots))
        .route(
            "/snapshots/{corridor_key}/timeline",
            get(get_corridor_timeline),
        )
        // Corridor performance summary
        .route("/summary", get(get_performance_summary))
        .route("/summary/{corridor_key}", get(get_corridor_summary))
        // Alert events
        .route("/events", get(list_events))
        .route("/events/{corridor_key}", get(list_events_for_corridor))
        .route("/events/{id}/acknowledge", post(acknowledge_event))
        .route("/unread-count", get(get_unread_count))
}

// ---- Config Handlers ----

/// GET /api/corridor-alerts/configs - List all alert configs for the authenticated user
#[utoipa::path(
    get,
    path = "/api/corridor-alerts/configs",
    responses(
        (status = 200, description = "List of corridor alert configs", body = Vec<CorridorAlertConfig>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn list_configs(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> ApiResult<impl IntoResponse> {
    let configs = state
        .db
        .get_corridor_alert_configs_for_user(&auth_user.user_id)
        .await?;
    Ok(Json(configs))
}

/// POST /api/corridor-alerts/configs - Create a new corridor alert config
#[utoipa::path(
    post,
    path = "/api/corridor-alerts/configs",
    request_body = CreateCorridorAlertConfigRequest,
    responses(
        (status = 201, description = "Config created", body = CorridorAlertConfig),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn create_config(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(payload): ValidatedJson<CreateCorridorAlertConfigRequest>,
) -> ApiResult<impl IntoResponse> {
    let config = state
        .db
        .create_corridor_alert_config(&auth_user.user_id, payload)
        .await?;
    Ok((StatusCode::CREATED, Json(config)))
}

/// GET /api/corridor-alerts/configs/{id} - Get a specific corridor alert config
#[utoipa::path(
    get,
    path = "/api/corridor-alerts/configs/{id}",
    params(("id" = String, Path, description = "Config ID")),
    responses(
        (status = 200, description = "Config found", body = CorridorAlertConfig),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn get_config(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let config = state
        .db
        .get_corridor_alert_config_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("Corridor alert config not found"))?;

    if config.user_id != auth_user.user_id {
        return Err(ApiError::forbidden("Access denied"));
    }

    Ok(Json(config))
}

/// PUT /api/corridor-alerts/configs/{id} - Update a corridor alert config
#[utoipa::path(
    put,
    path = "/api/corridor-alerts/configs/{id}",
    params(("id" = String, Path, description = "Config ID")),
    request_body = UpdateCorridorAlertConfigRequest,
    responses(
        (status = 200, description = "Config updated", body = CorridorAlertConfig),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn update_config(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<UpdateCorridorAlertConfigRequest>,
) -> ApiResult<impl IntoResponse> {
    let config = state
        .db
        .update_corridor_alert_config(&id, &auth_user.user_id, payload)
        .await?;
    Ok(Json(config))
}

/// DELETE /api/corridor-alerts/configs/{id} - Delete a corridor alert config
#[utoipa::path(
    delete,
    path = "/api/corridor-alerts/configs/{id}",
    params(("id" = String, Path, description = "Config ID")),
    responses(
        (status = 204, description = "Config deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn delete_config(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    state
        .db
        .delete_corridor_alert_config(&id, &auth_user.user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- Snapshot Handlers ----

/// GET /api/corridor-alerts/snapshots - Latest snapshots for all corridors
#[utoipa::path(
    get,
    path = "/api/corridor-alerts/snapshots",
    responses(
        (status = 200, description = "Latest snapshots", body = Vec<CorridorPerformanceSnapshot>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn list_all_latest_snapshots(
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let snapshots = state.db.get_latest_snapshots_all_corridors().await?;
    Ok(Json(snapshots))
}

/// GET /api/corridor-alerts/snapshots/{corridor_key} - Snapshots for a specific corridor
#[utoipa::path(
    get,
    path = "/api/corridor-alerts/snapshots/{corridor_key}",
    params(("corridor_key" = String, Path, description = "Corridor key")),
    responses(
        (status = 200, description = "Corridor snapshots", body = Vec<CorridorPerformanceSnapshot>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn get_corridor_snapshots(
    State(state): State<AppState>,
    Path(corridor_key): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let snapshots = state
        .db
        .get_snapshots_for_corridor(&corridor_key, 100)
        .await?;
    Ok(Json(snapshots))
}

/// GET /api/corridor-alerts/snapshots/{corridor_key}/timeline - Timeline with snapshots + alerts
#[utoipa::path(
    get,
    path = "/api/corridor-alerts/snapshots/{corridor_key}/timeline",
    params(("corridor_key" = String, Path, description = "Corridor key")),
    responses(
        (status = 200, description = "Corridor timeline", body = CorridorPerformanceTimeline),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn get_corridor_timeline(
    State(state): State<AppState>,
    Path(corridor_key): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let snapshots = state
        .db
        .get_snapshots_for_corridor(&corridor_key, 100)
        .await?;
    let alerts = state
        .db
        .get_corridor_alert_events_for_corridor(&corridor_key, 50)
        .await?;

    Ok(Json(CorridorPerformanceTimeline {
        corridor_key,
        snapshots,
        alerts,
    }))
}

// ---- Summary Handlers ----

/// GET /api/corridor-alerts/summary - Performance summary for all corridors
#[utoipa::path(
    get,
    path = "/api/corridor-alerts/summary",
    responses(
        (status = 200, description = "Performance summary", body = Vec<CorridorPerformanceSummary>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn get_performance_summary(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> ApiResult<impl IntoResponse> {
    let snapshots = state.db.get_latest_snapshots_all_corridors().await?;
    let events_24h = state
        .db
        .get_alert_events_24h_for_user(&auth_user.user_id)
        .await?;

    let mut summaries = Vec::new();

    for snapshot in &snapshots {
        let previous = state
            .db
            .get_previous_snapshot_for_corridor(&snapshot.corridor_key)
            .await?;

        let alert_count = events_24h
            .iter()
            .filter(|e| e.corridor_key == snapshot.corridor_key)
            .count() as i64;

        let success_rate_trend = match &previous {
            Some(p) if p.success_rate > 0.0 => {
                ((snapshot.success_rate - p.success_rate) / p.success_rate) * 100.0
            }
            _ => 0.0,
        };

        let latency_trend = match &previous {
            Some(p) if p.avg_settlement_latency_ms > 0.0 => {
                ((snapshot.avg_settlement_latency_ms - p.avg_settlement_latency_ms)
                    / p.avg_settlement_latency_ms)
                    * 100.0
            }
            _ => 0.0,
        };

        let liquidity_trend = match &previous {
            Some(p) if p.liquidity_depth_usd > 0.0 => {
                ((snapshot.liquidity_depth_usd - p.liquidity_depth_usd) / p.liquidity_depth_usd)
                    * 100.0
            }
            _ => 0.0,
        };

        let status = if snapshot.success_rate < 0.8 || alert_count > 5 {
            "critical"
        } else if snapshot.success_rate < 0.9 || alert_count > 2 {
            "warning"
        } else {
            "healthy"
        };

        summaries.push(CorridorPerformanceSummary {
            corridor_key: snapshot.corridor_key.clone(),
            current_success_rate: snapshot.success_rate,
            previous_success_rate: previous.as_ref().map(|p| p.success_rate),
            current_latency_ms: snapshot.avg_settlement_latency_ms,
            previous_latency_ms: previous.as_ref().map(|p| p.avg_settlement_latency_ms),
            current_liquidity_usd: snapshot.liquidity_depth_usd,
            previous_liquidity_usd: previous.as_ref().map(|p| p.liquidity_depth_usd),
            success_rate_trend,
            latency_trend,
            liquidity_trend,
            alert_count_24h: alert_count,
            status: status.to_string(),
        });
    }

    Ok(Json(summaries))
}

/// GET /api/corridor-alerts/summary/{corridor_key} - Performance summary for one corridor
#[utoipa::path(
    get,
    path = "/api/corridor-alerts/summary/{corridor_key}",
    params(("corridor_key" = String, Path, description = "Corridor key")),
    responses(
        (status = 200, description = "Corridor performance summary", body = CorridorPerformanceSummary),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn get_corridor_summary(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(corridor_key): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let snapshot = state
        .db
        .get_latest_snapshot_for_corridor(&corridor_key)
        .await?
        .ok_or_else(|| ApiError::not_found("No performance data for this corridor"))?;

    let previous = state
        .db
        .get_previous_snapshot_for_corridor(&corridor_key)
        .await?;

    let events_24h = state
        .db
        .get_alert_events_24h_for_user(&auth_user.user_id)
        .await?;

    let alert_count = events_24h
        .iter()
        .filter(|e| e.corridor_key == corridor_key)
        .count() as i64;

    let success_rate_trend = match &previous {
        Some(p) if p.success_rate > 0.0 => {
            ((snapshot.success_rate - p.success_rate) / p.success_rate) * 100.0
        }
        _ => 0.0,
    };

    let latency_trend = match &previous {
        Some(p) if p.avg_settlement_latency_ms > 0.0 => {
            ((snapshot.avg_settlement_latency_ms - p.avg_settlement_latency_ms)
                / p.avg_settlement_latency_ms)
                * 100.0
        }
        _ => 0.0,
    };

    let liquidity_trend = match &previous {
        Some(p) if p.liquidity_depth_usd > 0.0 => {
            ((snapshot.liquidity_depth_usd - p.liquidity_depth_usd) / p.liquidity_depth_usd)
                * 100.0
        }
        _ => 0.0,
    };

    let status = if snapshot.success_rate < 0.8 || alert_count > 5 {
        "critical"
    } else if snapshot.success_rate < 0.9 || alert_count > 2 {
        "warning"
    } else {
        "healthy"
    };

    Ok(Json(CorridorPerformanceSummary {
        corridor_key: snapshot.corridor_key.clone(),
        current_success_rate: snapshot.success_rate,
        previous_success_rate: previous.as_ref().map(|p| p.success_rate),
        current_latency_ms: snapshot.avg_settlement_latency_ms,
        previous_latency_ms: previous.as_ref().map(|p| p.avg_settlement_latency_ms),
        current_liquidity_usd: snapshot.liquidity_depth_usd,
        previous_liquidity_usd: previous.as_ref().map(|p| p.liquidity_depth_usd),
        success_rate_trend,
        latency_trend,
        liquidity_trend,
        alert_count_24h: alert_count,
        status: status.to_string(),
    }))
}

// ---- Event Handlers ----

/// GET /api/corridor-alerts/events - List alert events for the authenticated user
#[utoipa::path(
    get,
    path = "/api/corridor-alerts/events",
    responses(
        (status = 200, description = "Alert events", body = Vec<CorridorAlertEvent>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn list_events(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> ApiResult<impl IntoResponse> {
    let events = state
        .db
        .get_corridor_alert_events_for_user(&auth_user.user_id, 100)
        .await?;
    Ok(Json(events))
}

/// GET /api/corridor-alerts/events/{corridor_key} - List alert events for a specific corridor
#[utoipa::path(
    get,
    path = "/api/corridor-alerts/events/{corridor_key}",
    params(("corridor_key" = String, Path, description = "Corridor key")),
    responses(
        (status = 200, description = "Alert events", body = Vec<CorridorAlertEvent>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn list_events_for_corridor(
    State(state): State<AppState>,
    Path(corridor_key): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let events = state
        .db
        .get_corridor_alert_events_for_corridor(&corridor_key, 100)
        .await?;
    Ok(Json(events))
}

/// POST /api/corridor-alerts/events/{id}/acknowledge - Acknowledge an alert event
#[utoipa::path(
    post,
    path = "/api/corridor-alerts/events/{id}/acknowledge",
    params(("id" = String, Path, description = "Event ID")),
    responses(
        (status = 200, description = "Event acknowledged"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn acknowledge_event(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    state
        .db
        .acknowledge_corridor_alert_event(&id, &auth_user.user_id)
        .await?;
    Ok(StatusCode::OK)
}

/// GET /api/corridor-alerts/unread-count - Get unread alert count
#[utoipa::path(
    get,
    path = "/api/corridor-alerts/unread-count",
    responses(
        (status = 200, description = "Unread count"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridor Alerts"
)]
async fn get_unread_count(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> ApiResult<impl IntoResponse> {
    let count = state
        .db
        .get_unacknowledged_count_for_user(&auth_user.user_id)
        .await?;
    Ok(Json(serde_json::json!({ "count": count })))
}
