/// Webhook API endpoints
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::json;
use sqlx::SqlitePool;

use crate::auth_middleware::AuthUser;
use crate::webhooks::{CreateWebhookRequest, WebhookResponse, WebhookService};
use axum::extract::Query;

/// POST /webhooks - Register a new webhook
#[utoipa::path(
    post,
    path = "/api/v1/webhooks",
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "Webhook registered successfully"),
        (status = 400, description = "Invalid webhook URL or missing event types"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Webhooks"
)]
pub async fn register_webhook(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
    Json(request): Json<CreateWebhookRequest>,
) -> Result<Response, WebhookApiError> {
    // Validate webhook URL using centralized validation
    crate::validation::validate_webhook_url(&request.url)
        .map_err(|e| WebhookApiError::BadRequest(e.to_string()))?;

    // Validate event types
    if request.event_types.is_empty() {
        return Err(WebhookApiError::BadRequest(
            "At least one event type is required".to_string(),
        ));
    }

    let service = WebhookService::new(db);
    let response = service
        .register_webhook(&auth_user.user_id, request)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// GET /webhooks - List webhooks for authenticated user
#[utoipa::path(
    get,
    path = "/api/v1/webhooks",
    responses(
        (status = 200, description = "List of webhooks"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Webhooks"
)]
pub async fn list_webhooks(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
) -> Result<Response, WebhookApiError> {
    let service = WebhookService::new(db);
    let webhooks = service
        .list_webhooks(&auth_user.user_id)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?;

    let response: Vec<WebhookResponse> = webhooks
        .into_iter()
        .map(|w| WebhookResponse {
            id: w.id,
            url: w.url,
            event_types: w
                .event_types
                .split(',')
                .map(std::string::ToString::to_string)
                .collect(),
            filters: w
                .filters
                .as_ref()
                .and_then(|f| serde_json::from_str(f).ok()),
            is_active: w.is_active,
            created_at: w.created_at,
        })
        .collect();

    Ok((StatusCode::OK, Json(json!({"webhooks": response}))).into_response())
}

/// DELETE /webhooks/:id - Delete/deactivate webhook
#[utoipa::path(
    delete,
    path = "/api/v1/webhooks/{id}",
    params(
        ("id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Webhook deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Webhook not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Webhooks"
)]
pub async fn delete_webhook(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
    Path(webhook_id): Path<String>,
) -> Result<Response, WebhookApiError> {
    let service = WebhookService::new(db);
    let deleted = service
        .delete_webhook(&webhook_id, &auth_user.user_id)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?;

    if !deleted {
        return Err(WebhookApiError::NotFound("Webhook not found".to_string()));
    }

    Ok((
        StatusCode::OK,
        Json(json!({"message": "Webhook deleted successfully"})),
    )
        .into_response())
}

/// GET /webhooks/:id - Get a single webhook by ID
#[utoipa::path(
    get,
    path = "/api/v1/webhooks/{id}",
    params(
        ("id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Webhook details", body = WebhookResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not owner"),
        (status = 404, description = "Webhook not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Webhooks"
)]
pub async fn get_webhook(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
    Path(webhook_id): Path<String>,
) -> Result<Response, WebhookApiError> {
    let service = WebhookService::new(db);
    let webhook = service
        .get_webhook(&webhook_id)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?
        .ok_or_else(|| WebhookApiError::NotFound("Webhook not found".to_string()))?;

    if webhook.user_id != auth_user.user_id {
        return Err(WebhookApiError::Forbidden);
    }

    let response = WebhookResponse {
        id: webhook.id,
        url: webhook.url,
        event_types: webhook
            .event_types
            .split(',')
            .map(std::string::ToString::to_string)
            .collect(),
        filters: webhook
            .filters
            .as_ref()
            .and_then(|f| serde_json::from_str(f).ok()),
        is_active: webhook.is_active,
        created_at: webhook.created_at,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /webhooks/:id/test - Queue a test event for delivery
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/{id}/test",
    params(
        ("id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Test event queued"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not owner"),
        (status = 404, description = "Webhook not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Webhooks"
)]
pub async fn test_webhook(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
    Path(webhook_id): Path<String>,
) -> Result<Response, WebhookApiError> {
    let service = WebhookService::new(db);

    // Get webhook and verify ownership
    let webhook = service
        .get_webhook(&webhook_id)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?
        .ok_or_else(|| WebhookApiError::NotFound("Webhook not found".to_string()))?;

    if webhook.user_id != auth_user.user_id {
        return Err(WebhookApiError::Forbidden);
    }

    let test_payload = json!({
        "message": "This is a test webhook delivery",
        "webhook_id": webhook_id,
    });

    let event_id = service
        .create_webhook_event(&webhook_id, "test", test_payload.clone())
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "message": "Test event queued for delivery",
            "event_id": event_id,
            "payload": test_payload
        })),
    )
        .into_response())
}

/// Webhook API Error types
#[derive(Debug)]
pub enum WebhookApiError {
    NotFound(String),
    BadRequest(String),
    Forbidden,
    ServerError(String),
}

impl IntoResponse for WebhookApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "You don't have permission to access this webhook".to_string(),
            ),
            Self::ServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(json!({"error": message}))).into_response()
    }
}

/// GET /webhooks/:id/delivery-logs - Get delivery logs for a webhook
#[utoipa::path(
    get,
    path = "/api/v1/webhooks/{id}/delivery-logs",
    params(
        ("id" = String, Path, description = "Webhook ID"),
        ("limit" = u32, Query, description = "Maximum number of logs to return"),
        ("offset" = u32, Query, description = "Number of logs to skip")
    ),
    responses(
        (status = 200, description = "Delivery logs retrieved successfully", body = Vec<DeliveryLogResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not owner"),
        (status = 404, description = "Webhook not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Webhooks"
)]
pub async fn get_delivery_logs(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
    Path(webhook_id): Path<String>,
    query: Query<DeliveryLogsQuery>,
) -> Result<Response, WebhookApiError> {
    let service = WebhookService::new(db);

    // Verify webhook ownership
    let webhook = service
        .get_webhook(&webhook_id)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?
        .ok_or_else(|| WebhookApiError::NotFound("Webhook not found".to_string()))?;

    if webhook.user_id != auth_user.user_id {
        return Err(WebhookApiError::Forbidden);
    }

    // Clamp the page size: an unbounded client-supplied limit would let a
    // single request pull every delivery log row into memory, and limit=0
    // would report has_more=true forever.
    let limit = query.limit.clamp(1, MAX_DELIVERY_LOGS_LIMIT) as usize;

    // Get delivery logs
    let logs = service
        .get_delivery_logs(&webhook_id, limit, query.offset as usize)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?;

    let has_more = logs.len() >= limit;
    let response: Vec<DeliveryLogResponse> = logs
        .into_iter()
        .map(|l| DeliveryLogResponse {
            id: l.id,
            webhook_id: l.webhook_id,
            event_id: l.event_id,
            delivery_id: l.delivery_id,
            status: l.status,
            http_status: l.http_status,
            response_body: l.response_body,
            error_message: l.error_message,
            signature: l.signature,
            event_version: l.event_version,
            timestamp: l.timestamp,
            created_at: l.created_at,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(json!({
            "delivery_logs": response,
            "has_more": has_more
        })),
    )
        .into_response())
}

/// Delivery logs query parameters
#[derive(Debug, serde::Deserialize)]
pub struct DeliveryLogsQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default = "default_offset")]
    pub offset: u32,
}

fn default_limit() -> u32 {
    50
}

/// Largest page of delivery logs a single request may fetch.
const MAX_DELIVERY_LOGS_LIMIT: u32 = 200;

fn default_offset() -> u32 {
    0
}

/// Delivery log response
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DeliveryLogResponse {
    pub id: String,
    pub webhook_id: String,
    pub event_id: Option<String>,
    pub delivery_id: String,
    pub status: String,
    pub http_status: Option<i32>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub signature: Option<String>,
    pub event_version: String,
    pub timestamp: String,
    pub created_at: String,
}

/// Enable a webhook that was auto-disabled
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/{id}/enable",
    params(
        ("id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Webhook enabled successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not owner"),
        (status = 404, description = "Webhook not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Webhooks"
)]
pub async fn enable_webhook(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
    Path(webhook_id): Path<String>,
) -> Result<Response, WebhookApiError> {
    let service = WebhookService::new(db);

    // Verify webhook ownership
    let webhook = service
        .get_webhook(&webhook_id)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?
        .ok_or_else(|| WebhookApiError::NotFound("Webhook not found".to_string()))?;

    if webhook.user_id != auth_user.user_id {
        return Err(WebhookApiError::Forbidden);
    }

    // Check if webhook is already active
    if webhook.is_active {
        return Err(WebhookApiError::BadRequest(
            "Webhook is already active".to_string(),
        ));
    }

    // Enable the webhook
    let enabled = service
        .enable_webhook(&webhook_id, &auth_user.user_id)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?;

    if !enabled {
        return Err(WebhookApiError::NotFound("Webhook not found".to_string()));
    }

    Ok((
        StatusCode::OK,
        Json(json!({
            "message": "Webhook enabled successfully",
            "webhook_id": webhook_id
        })),
    )
        .into_response())
}

/// Rotate webhook secrets
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/{id}/rotate-secrets",
    params(
        ("id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Secrets rotated successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not owner"),
        (status = 404, description = "Webhook not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Webhooks"
)]
pub async fn rotate_secrets(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
    Path(webhook_id): Path<String>,
) -> Result<Response, WebhookApiError> {
    let service = WebhookService::new(db);

    // Verify webhook ownership
    let webhook = service
        .get_webhook(&webhook_id)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?
        .ok_or_else(|| WebhookApiError::NotFound("Webhook not found".to_string()))?;

    if webhook.user_id != auth_user.user_id {
        return Err(WebhookApiError::Forbidden);
    }

    // Rotate secrets if needed
    let rotated = service
        .rotate_secrets_if_needed(&webhook_id)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "message": if rotated { "Secrets rotated successfully" } else { "No rotation needed" },
            "webhook_id": webhook_id,
            "rotated": rotated
        })),
    )
        .into_response())
}

/// Create webhook routes
pub fn routes(db: SqlitePool) -> Router {
    Router::new()
        .route("/", post(register_webhook).get(list_webhooks))
        .route("/{id}", get(get_webhook).delete(delete_webhook))
        .route("/{id}/test", post(test_webhook))
        .route("/{id}/delivery-logs", get(get_delivery_logs))
        .route("/{id}/enable", post(enable_webhook))
        .route("/{id}/rotate-secrets", post(rotate_secrets))
        .with_state(db)
}
