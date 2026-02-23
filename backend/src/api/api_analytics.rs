use axum::{
    extract::State,
    response::Response,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use crate::database::Database;
use crate::models::{ApiAnalyticsOverview, EndpointStat, StatusStat};

/// Handler for GET /api/admin/analytics/overview
pub async fn get_analytics_overview(
    State(db): State<Arc<Database>>,
) -> Json<ApiAnalyticsOverview> {
    // 1. Total Requests
    let total_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_usage_stats")
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);

    // 2. Avg Response Time
    let avg_response_time_ms: f64 = sqlx::query_scalar("SELECT AVG(response_time_ms) FROM api_usage_stats")
        .fetch_one(db.pool())
        .await
        .unwrap_or(0.0);

    // 3. Error Rate (4xx and 5xx)
    let error_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_usage_stats WHERE status_code >= 400")
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
         LIMIT 10"
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();

    // 5. Status Distribution
    let status_distribution = sqlx::query_as::<_, StatusStat>(
        "SELECT status_code, COUNT(*) as count 
         FROM api_usage_stats 
         GROUP BY status_code 
         ORDER BY count DESC"
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

pub fn routes(db: Arc<Database>) -> Router {
    Router::new()
        .route("/api/admin/analytics/overview", get(get_analytics_overview))
        .with_state(db)
}
