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

    let response = next.run(req).await;

    let duration = start.elapsed().as_millis() as i32;
    let status = response.status().as_u16() as i32;

    // Save to database asynchronously
    let db_clone = Arc::clone(&db);
    tokio::spawn(async move {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();

        let result = sqlx::query(
            "INSERT INTO api_usage_stats (id, endpoint, method, status_code, response_time_ms, user_id, timestamp) 
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(path)
        .bind(method.to_string())
        .bind(status)
        .bind(duration)
        .bind(user_id)
        .bind(timestamp)
        .execute(db_clone.pool())
        .await;

        if let Err(e) = result {
            tracing::error!("Failed to record API usage stat: {}", e);
        }
    });

    response
}
