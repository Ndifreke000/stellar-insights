use crate::database::Database;
use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
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

    // Extract the API key id if the request was authenticated with one, so usage
    // can be aggregated per key for the self-serve dashboard.
    let api_key_id = req
        .extensions()
        .get::<crate::auth_middleware::ApiKeyAuth>()
        .map(|k| k.api_key_id.clone());

    let response = next.run(req).await;

    let duration = start.elapsed().as_millis() as i32;
    let status = i32::from(response.status().as_u16());

    // Save to database asynchronously
    let db_clone = Arc::clone(&db);
    tokio::spawn(async move {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();

        let result = sqlx::query(
            "INSERT INTO api_usage_stats (id, endpoint, method, status_code, response_time_ms, user_id, api_key_id, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(path)
        .bind(method.to_string())
        .bind(status)
        .bind(duration)
        .bind(user_id)
        .bind(api_key_id)
        .bind(timestamp)
        .execute(db_clone.pool())
        .await;

        if let Err(e) = result {
            tracing::error!("Failed to record API usage stat: {}", e);
        }
    });

    response
}

/// Number of days of history returned by the usage dashboard endpoint.
pub const USAGE_WINDOW_DAYS: i64 = 30;

/// Per-day usage aggregate for a single API key.
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyDailyUsage {
    /// Day bucket in `YYYY-MM-DD` (UTC).
    pub date: String,
    pub request_count: i64,
    pub error_count: i64,
}

/// Aggregated usage for one API key over the dashboard window.
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyUsage {
    pub api_key_id: String,
    pub request_count: i64,
    pub error_count: i64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub daily: Vec<ApiKeyDailyUsage>,
}

/// Aggregate request logs per API key per day for the last `USAGE_WINDOW_DAYS`.
///
/// Only rows recorded with an `api_key_id` are considered. Errors are any
/// response with a status code `>= 400`. Days with no traffic are omitted.
pub async fn aggregate_api_key_usage(
    db: &Database,
    api_key_id: &str,
) -> Result<ApiKeyUsage, sqlx::Error> {
    let since = Utc::now() - Duration::days(USAGE_WINDOW_DAYS);

    let rows: Vec<(String, i64, i64, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT date(timestamp) AS day,
                COUNT(*) AS request_count,
                SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) AS error_count,
                MAX(timestamp) AS last_used_at
         FROM api_usage_stats
         WHERE api_key_id = ? AND timestamp >= ?
         GROUP BY day
         ORDER BY day ASC",
    )
    .bind(api_key_id)
    .bind(since)
    .fetch_all(db.pool())
    .await?;

    let mut request_count = 0i64;
    let mut error_count = 0i64;
    let mut last_used_at: Option<DateTime<Utc>> = None;
    let mut daily = Vec::with_capacity(rows.len());

    for (date, requests, errors, last) in rows {
        request_count += requests;
        error_count += errors;
        if let Some(ts) = last {
            last_used_at = Some(match last_used_at {
                Some(current) if current >= ts => current,
                _ => ts,
            });
        }
        daily.push(ApiKeyDailyUsage {
            date,
            request_count: requests,
            error_count: errors,
        });
    }

    Ok(ApiKeyUsage {
        api_key_id: api_key_id.to_string(),
        request_count,
        error_count,
        last_used_at,
        daily,
    })
}
