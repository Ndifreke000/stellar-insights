use crate::database::Database;
use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Middleware to track API usage analytics
pub async fn api_analytics_middleware(
    State(db): State<Arc<Database>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path().to_string();

    // Extract user_id if available (requires auth_middleware to be applied before)
    let user_id = req
        .extensions()
        .get::<crate::auth_middleware::AuthUser>()
        .map(|u| u.user_id.clone());

    // Extract the API key id if available so request logs can be scoped per key.
    // Only the key identifier is stored; the raw key/secret is never persisted.
    let api_key_id = req
        .extensions()
        .get::<crate::auth_middleware::ApiKeyIdentity>()
        .map(|k| k.key_id.clone())
        .or_else(|| {
            req.headers()
                .get("x-api-key-id")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        });

    let response = next.run(req).await;

    let duration = start.elapsed().as_millis() as i32;
    let status = i32::from(response.status().as_u16());

    // Save to database asynchronously
    let db_clone = Arc::clone(&db);
    tokio::spawn(async move {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();

        let result = sqlx::query(
            "INSERT INTO api_usage_stats (id, endpoint, method, status_code, response_time_ms, user_id, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&path)
        .bind(method.to_string())
        .bind(status)
        .bind(duration)
        .bind(&user_id)
        .bind(timestamp)
        .execute(db_clone.pool())
        .await;

        if let Err(e) = result {
            tracing::error!("Failed to record API usage stat: {}", e);
        }

        // Store a sampled, redacted request log entry per API key. Only the
        // method, route, status, latency and timestamp are kept; query strings,
        // headers and bodies are intentionally dropped to avoid leaking secrets.
        if let Some(key_id) = api_key_id {
            let log_result = sqlx::query(
                "INSERT INTO api_request_logs (id, api_key_id, method, route, status_code, latency_ms, timestamp)
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(Uuid::new_v4().to_string())
            .bind(key_id)
            .bind(method.to_string())
            .bind(&path)
            .bind(status)
            .bind(duration)
            .bind(timestamp)
            .execute(db_clone.pool())
            .await;

            if let Err(e) = log_result {
                tracing::error!("Failed to record API request log: {}", e);
            }
        }
    });

    response
}
