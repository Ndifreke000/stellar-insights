/// Webhook API endpoints
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use sqlx::SqlitePool;

use crate::auth_middleware::AuthUser;
use crate::webhooks::{CreateWebhookRequest, WebhookResponse, WebhookService};

/// POST /api/webhooks - Register a new webhook
#[utoipa::path(
    post,
    path = "/api/webhooks",
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
    // Validate URL scheme
    if !request.url.starts_with("https://") && !request.url.starts_with("http://") {
        return Err(WebhookApiError::BadRequest(
            "Webhook URL must be valid HTTP(S)".to_string(),
        ));
    }

    // SSRF protection: block private/internal URLs (SEC-008)
    if let Ok(url) = url::Url::parse(&request.url) {
        if let Some(host) = url.host_str() {
            let host_lower = host.to_lowercase();
            // Block localhost and loopback
            if host_lower == "localhost"
                || host_lower == "127.0.0.1"
                || host_lower == "::1"
                || host_lower == "0.0.0.0"
                || host_lower.ends_with(".local")
                || host_lower.ends_with(".internal")
            {
                return Err(WebhookApiError::BadRequest(
                    "Webhook URL must not point to localhost or internal addresses".to_string(),
                ));
            }
            // Block AWS metadata endpoint
            if host_lower == "169.254.169.254" || host_lower == "metadata.google.internal" {
                return Err(WebhookApiError::BadRequest(
                    "Webhook URL must not point to cloud metadata endpoints".to_string(),
                ));
            }
            // Block common private IP ranges
            if let Ok(ip) = host.parse::<std::net::IpAddr>() {
                let is_private = match ip {
                    std::net::IpAddr::V4(v4) => {
                        v4.is_loopback()
                            || v4.is_private()
                            || v4.is_link_local()
                            || v4.octets()[0] == 169 && v4.octets()[1] == 254
                    }
                    std::net::IpAddr::V6(v6) => v6.is_loopback(),
                };
                if is_private {
                    return Err(WebhookApiError::BadRequest(
                        "Webhook URL must not point to private or reserved IP addresses"
                            .to_string(),
                    ));
                }
            }
        }
    } else {
        return Err(WebhookApiError::BadRequest(
            "Webhook URL is not a valid URL".to_string(),
        ));
    }

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

/// GET /api/webhooks - List webhooks for authenticated user
#[utoipa::path(
    get,
    path = "/api/webhooks",
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

/// DELETE /api/webhooks/:id - Delete/deactivate webhook
#[utoipa::path(
    delete,
    path = "/api/webhooks/{id}",
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

/// GET /api/webhooks/:id - Get a single webhook by ID
#[utoipa::path(
    get,
    path = "/api/webhooks/{id}",
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

/// POST /api/webhooks/:id/test - Queue a test event for delivery
#[utoipa::path(
    post,
    path = "/api/webhooks/{id}/test",
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

/// Create webhook routes
pub fn routes(db: SqlitePool) -> Router {
    Router::new()
        .route("/api/webhooks", post(register_webhook).get(list_webhooks))
        .route("/api/webhooks/:id", get(get_webhook).delete(delete_webhook))
        .route("/api/webhooks/:id/test", post(test_webhook))
        .with_state(db)
}
