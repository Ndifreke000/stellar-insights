use axum::{
    extract::{ws::WebSocket, Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;

use crate::{
    alerts::AlertManager,
    auth_middleware::AuthUser,
    error::{ApiError, ApiResult},
    models::alerts::{CreateAlertRuleRequest, SnoozeAlertRequest, UpdateAlertRuleRequest},
    state::AppState,
};

const VALID_METRIC_TYPES: &[&str] = &["success_rate", "latency", "liquidity", "volume"];
const VALID_CONDITIONS: &[&str] = &["above", "below", "equals"];
const MAX_CORRIDOR_ID_LEN: usize = 256;

fn validate_metric_type(metric_type: &str) -> ApiResult<()> {
    if !VALID_METRIC_TYPES.contains(&metric_type) {
        return Err(ApiError::bad_request(
            "INVALID_METRIC_TYPE",
            format!("metric_type must be one of: {}", VALID_METRIC_TYPES.join(", ")),
        ));
    }
    Ok(())
}

fn validate_condition(condition: &str) -> ApiResult<()> {
    if !VALID_CONDITIONS.contains(&condition) {
        return Err(ApiError::bad_request(
            "INVALID_CONDITION",
            format!("condition must be one of: {}", VALID_CONDITIONS.join(", ")),
        ));
    }
    Ok(())
}

fn validate_threshold(threshold: f64) -> ApiResult<()> {
    if !threshold.is_finite() {
        return Err(ApiError::bad_request(
            "INVALID_THRESHOLD",
            "threshold must be a finite number",
        ));
    }
    if threshold < 0.0 {
        return Err(ApiError::bad_request(
            "INVALID_THRESHOLD",
            "threshold must be non-negative",
        ));
    }
    Ok(())
}

fn validate_corridor_id(corridor_id: &Option<String>) -> ApiResult<()> {
    if let Some(id) = corridor_id {
        if id.is_empty() {
            return Err(ApiError::bad_request(
                "INVALID_CORRIDOR_ID",
                "corridor_id must not be empty if provided",
            ));
        }
        if id.len() > MAX_CORRIDOR_ID_LEN {
            return Err(ApiError::bad_request(
                "INVALID_CORRIDOR_ID",
                format!("corridor_id must not exceed {MAX_CORRIDOR_ID_LEN} characters"),
            ));
        }
    }
    Ok(())
}

// Route configuration
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/rules", get(list_rules).post(create_rule))
        .route("/rules/:id", put(update_rule).delete(delete_rule))
        .route("/history", get(list_history))
        .route("/history/:id/read", post(mark_history_read))
        .route("/history/:id/dismiss", post(dismiss_history))
        .route("/history/:id/snooze", post(snooze_rule_from_history)) // snoozes the underlying rule
}

// Rule Handlers

/// GET /api/alerts/rules - List all alert rules for the authenticated user
#[utoipa::path(
    get,
    path = "/api/alerts/rules",
    responses(
        (status = 200, description = "List of alert rules", body = Vec<AlertRule>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Alerts"
)]
async fn list_rules(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> ApiResult<impl IntoResponse> {
    let rules = state
        .db
        .get_alert_rules_for_user(&auth_user.user_id)
        .await?;
    Ok(Json(rules))
}

/// POST /api/alerts/rules - Create a new alert rule
#[utoipa::path(
    post,
    path = "/api/alerts/rules",
    request_body = CreateAlertRuleRequest,
    responses(
        (status = 201, description = "Alert rule created", body = AlertRule),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Alerts"
)]
async fn create_rule(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateAlertRuleRequest>,
) -> ApiResult<impl IntoResponse> {
    validate_corridor_id(&payload.corridor_id)?;
    validate_metric_type(&payload.metric_type)?;
    validate_condition(&payload.condition)?;
    validate_threshold(payload.threshold)?;
    let rule = state
        .db
        .create_alert_rule(&auth_user.user_id, payload)
        .await?;
    Ok((StatusCode::CREATED, Json(rule)))
}

/// PUT /api/alerts/rules/{id} - Update an existing alert rule
#[utoipa::path(
    put,
    path = "/api/alerts/rules/{id}",
    params(
        ("id" = String, Path, description = "Alert rule ID")
    ),
    request_body = UpdateAlertRuleRequest,
    responses(
        (status = 200, description = "Alert rule updated", body = AlertRule),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Alert rule not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Alerts"
)]
async fn update_rule(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAlertRuleRequest>,
) -> ApiResult<impl IntoResponse> {
    validate_corridor_id(&payload.corridor_id)?;
    if let Some(ref mt) = payload.metric_type {
        validate_metric_type(mt)?;
    }
    if let Some(ref cond) = payload.condition {
        validate_condition(cond)?;
    }
    if let Some(t) = payload.threshold {
        validate_threshold(t)?;
    }
    let rule = state
        .db
        .update_alert_rule(&id, &auth_user.user_id, payload)
        .await?;
    Ok(Json(rule))
}

/// DELETE /api/alerts/rules/{id} - Delete an alert rule
#[utoipa::path(
    delete,
    path = "/api/alerts/rules/{id}",
    params(
        ("id" = String, Path, description = "Alert rule ID")
    ),
    responses(
        (status = 204, description = "Alert rule deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Alert rule not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Alerts"
)]
async fn delete_rule(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    state.db.delete_alert_rule(&id, &auth_user.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// History Handlers

/// GET /api/alerts/history - List alert history for the authenticated user
#[utoipa::path(
    get,
    path = "/api/alerts/history",
    responses(
        (status = 200, description = "List of alert history entries", body = Vec<AlertHistory>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Alerts"
)]
async fn list_history(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> ApiResult<impl IntoResponse> {
    // default limit
    let history = state
        .db
        .get_alert_history_for_user(&auth_user.user_id, 100)
        .await?;
    Ok(Json(history))
}

/// POST /api/alerts/history/{id}/read - Mark an alert history entry as read
#[utoipa::path(
    post,
    path = "/api/alerts/history/{id}/read",
    params(
        ("id" = String, Path, description = "Alert history ID")
    ),
    responses(
        (status = 200, description = "Alert history marked as read"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Alert history not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Alerts"
)]
async fn mark_history_read(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    state
        .db
        .mark_alert_history_read(&id, &auth_user.user_id)
        .await?;
    Ok(StatusCode::OK)
}

/// POST /api/alerts/history/{id}/dismiss - Dismiss an alert history entry
#[utoipa::path(
    post,
    path = "/api/alerts/history/{id}/dismiss",
    params(
        ("id" = String, Path, description = "Alert history ID")
    ),
    responses(
        (status = 200, description = "Alert history dismissed"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Alert history not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Alerts"
)]
async fn dismiss_history(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    state
        .db
        .dismiss_alert_history(&id, &auth_user.user_id)
        .await?;
    Ok(StatusCode::OK)
}

/// POST /api/alerts/history/{id}/snooze - Snooze the underlying rule from an alert history entry
#[utoipa::path(
    post,
    path = "/api/alerts/history/{id}/snooze",
    params(
        ("id" = String, Path, description = "Alert history ID")
    ),
    request_body = SnoozeAlertRequest,
    responses(
        (status = 200, description = "Rule snoozed", body = AlertRule),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Alert history not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Alerts"
)]
async fn snooze_rule_from_history(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<SnoozeAlertRequest>,
) -> ApiResult<impl IntoResponse> {
    // Reject snooze times in the past
    if payload.snoozed_until <= chrono::Utc::now() {
        return Err(ApiError::bad_request(
            "INVALID_SNOOZE_TIME",
            "snoozed_until must be a future timestamp",
        ));
    }
    // Id passed here is the rule's ID since we are snoozing the rule
    let rule = state
        .db
        .snooze_alert_rule(&id, &auth_user.user_id, payload)
        .await?;
    Ok(Json(rule))
}
// WebSocket Handler for real-time alerts

pub fn alert_websocket_handler(
    ws: WebSocketUpgrade,
    State(alert_manager): State<Arc<AlertManager>>,
) -> Response {
    ws.on_upgrade(|socket| handle_alert_socket(socket, alert_manager))
}

async fn handle_alert_socket(socket: WebSocket, alert_manager: Arc<AlertManager>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = alert_manager.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(alert) = rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&alert) {
                if sender
                    .send(axum::extract::ws::Message::Text(msg))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    let mut recv_task =
        tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}
