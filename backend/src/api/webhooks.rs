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
pub async fn register_webhook(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
    Json(request): Json<CreateWebhookRequest>,
) -> Result<Response, WebhookApiError> {
    // Validate URL
    if !request.url.starts_with("https://") && !request.url.starts_with("http://") {
        return Err(WebhookApiError::BadRequest(
            "Webhook URL must be valid HTTP(S)".to_string(),
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
            event_types: w.event_types.split(',').map(|s| s.to_string()).collect(),
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

/// POST /api/webhooks/:id/test - Send test payload to webhook
pub async fn test_webhook(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
    Path(webhook_id): Path<String>,
) -> Result<Response, WebhookApiError> {
    let service = WebhookService::new(db);

    // Get webhook
    let webhook = service
        .get_webhook(&webhook_id)
        .await
        .map_err(|e| WebhookApiError::ServerError(e.to_string()))?
        .ok_or_else(|| WebhookApiError::NotFound("Webhook not found".to_string()))?;

    // Verify ownership
    if webhook.user_id != auth_user.user_id {
        return Err(WebhookApiError::Forbidden);
    }

    // Create test payload
    let test_payload = json!({
        "event": "test",
        "timestamp": chrono::Utc::now().timestamp(),
        "data": {
            "message": "This is a test webhook delivery"
        }
    });

    // Send test delivery (simplified - doesn't actually send, just validates)
    // In real implementation, would fire off async HTTP request with retry logic
    tracing::info!(
        "Test webhook delivery for webhook_id={}: {}",
        webhook_id,
        test_payload
    );

    Ok((
        StatusCode::OK,
        Json(json!({"message": "Test webhook prepared", "payload": test_payload})),
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
            WebhookApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            WebhookApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            WebhookApiError::Forbidden => (
                StatusCode::FORBIDDEN,
                "You don't have permission to access this webhook".to_string(),
            ),
            WebhookApiError::ServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(json!({"error": message}))).into_response()
    }
}

/// Create webhook routes
pub fn routes(db: SqlitePool) -> Router {
    Router::new()
        .route("/api/webhooks", post(register_webhook).get(list_webhooks))
        .route("/api/webhooks/:id", delete(delete_webhook))
        .route("/api/webhooks/:id/test", post(test_webhook))
        .with_state(db)
}
