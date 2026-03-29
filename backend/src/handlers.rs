use axum::{extract::State, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::cache::CacheManager;
use crate::database::Database;
use crate::error::ApiResult;
use crate::rpc::StellarRpcClient;
use crate::state::AppState;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HealthChecks,
}

#[derive(Serialize, Debug, Clone)]
pub struct HealthChecks {
    pub database: ComponentHealth,
    pub cache: ComponentHealth,
    pub rpc: ComponentHealth,
}

#[derive(Serialize, Debug, Clone)]
pub struct ComponentHealth {
    pub healthy: bool,
    pub response_time_ms: Option<u64>,
    pub message: Option<String>,
}

/// Check database health
async fn check_database(db: &Arc<Database>) -> ComponentHealth {
    let start = Instant::now();
    match sqlx::query("SELECT 1").fetch_one(db.pool()).await {
        Ok(_) => ComponentHealth {
            healthy: true,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            healthy: false,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("Database connection failed: {}", e)),
        },
    }
}

/// Check cache health
async fn check_cache(cache: &Arc<CacheManager>) -> ComponentHealth {
    let start = Instant::now();
    match cache.ping().await {
        Ok(_) => ComponentHealth {
            healthy: true,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            healthy: false,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("Cache connection failed: {}", e)),
        },
    }
}

/// Check RPC health
async fn check_rpc(rpc: &Arc<StellarRpcClient>) -> ComponentHealth {
    let start = Instant::now();
    match rpc.check_health().await {
        Ok(_) => ComponentHealth {
            healthy: true,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            healthy: false,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("RPC connection failed: {}", e)),
        },
    }
}

/// Detailed health check endpoint
pub async fn health_check(State(app_state): State<AppState>) -> Json<HealthStatus> {
    let db_health = check_database(&app_state.db).await;
    let cache_health = check_cache(&app_state.cache).await;
    let rpc_health = check_rpc(&app_state.rpc_client).await;

    let overall_status = if db_health.healthy && cache_health.healthy && rpc_health.healthy {
        "healthy"
    } else if db_health.healthy && cache_health.healthy {
        "degraded"
    } else {
        "unhealthy"
    };

    let start_epoch = app_state.server_start_time.load(Ordering::Relaxed);
    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let uptime_seconds = now_epoch.saturating_sub(start_epoch);

    let health_status = HealthStatus {
        status: overall_status.to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        checks: HealthChecks {
            database: db_health,
            cache: cache_health,
            rpc: rpc_health,
        },
    };

    Json(health_status)
}

/// GET /api/admin/pool-metrics - Return current database pool metrics
pub fn get_pool_metrics(
    State(app_state): State<AppState>,
) -> Json<crate::database::PoolMetrics> {
    Json(app_state.db.pool_metrics())
}

/// GET /metrics - Prometheus metrics endpoint (all registered metrics via global registry)
pub async fn get_prometheus_metrics() -> impl IntoResponse {
    crate::observability::metrics::metrics_handler()
}

#[cfg(test)]
fn render_pool_metrics_prometheus(metrics: &crate::database::PoolMetrics) -> String {
    format!(
        "# HELP stellar_insights_db_pool_size Database pool size\n\
# TYPE stellar_insights_db_pool_size gauge\n\
stellar_insights_db_pool_size {}\n\
# HELP stellar_insights_db_pool_idle Database pool idle connections\n\
# TYPE stellar_insights_db_pool_idle gauge\n\
stellar_insights_db_pool_idle {}\n\
# HELP stellar_insights_db_pool_active Database pool active connections\n\
# TYPE stellar_insights_db_pool_active gauge\n\
stellar_insights_db_pool_active {}\n",
        metrics.size, metrics.idle, metrics.active
    )
}

/// Database pool metrics endpoint
pub async fn pool_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = state.db.pool_metrics();
    Json(metrics)
}

pub async fn ingestion_status(
    State(app_state): State<AppState>,
) -> ApiResult<Json<crate::ingestion::IngestionStatus>> {
    let status = app_state.ingestion.get_ingestion_status().await?;
    Ok(Json(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_pool_metrics_prometheus() {
        let metrics = crate::database::PoolMetrics::new(12, 3, 9);
        let rendered = render_pool_metrics_prometheus(&metrics);

        assert!(rendered.contains("stellar_insights_db_pool_size 12"));
        assert!(rendered.contains("stellar_insights_db_pool_idle 3"));
        assert!(rendered.contains("stellar_insights_db_pool_active 9"));
        assert!(rendered.contains("# TYPE stellar_insights_db_pool_size gauge"));
    }
}
