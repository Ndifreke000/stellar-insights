use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::cache::CacheManager;
use crate::database::Database;
use crate::rpc::StellarRpcClient;
use crate::state::AppState;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HealthChecks,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthChecks {
    pub database: ComponentHealth,
    pub cache: ComponentHealth,
    pub rpc: ComponentHealth,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComponentHealth {
    pub status: String, // "healthy", "degraded", "unhealthy"
    pub response_time_ms: Option<u64>,
    pub message: Option<String>,
}

/// Check database health
async fn check_database_health(db: &Arc<Database>) -> ComponentHealth {
    let start = Instant::now();

    match sqlx::query("SELECT 1").fetch_one(db.pool()).await {
        Ok(_) => ComponentHealth {
            status: "healthy".to_string(),
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            status: "unhealthy".to_string(),
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("Database error: {}", e)),
        },
    }
}

/// Check cache health
async fn check_cache_health(cache: &Arc<CacheManager>) -> ComponentHealth {
    let start = Instant::now();

    match cache.ping().await {
        Ok(_) => ComponentHealth {
            status: "healthy".to_string(),
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            status: "degraded".to_string(),
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("Cache error: {}", e)),
        },
    }
}

/// Check RPC health
async fn check_rpc_health(rpc: &Arc<StellarRpcClient>) -> ComponentHealth {
    let start = Instant::now();

    match rpc.check_health().await {
        Ok(_) => ComponentHealth {
            status: "healthy".to_string(),
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            status: "degraded".to_string(),
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("RPC error: {}", e)),
        },
    }
}

/// Detailed health check endpoint
pub async fn health_check(State(app_state): State<AppState>) -> impl IntoResponse {
    let start_time = Instant::now();

    // Check database
    let db_health = check_database_health(&app_state.db).await;

    // Check cache
    let cache_health = check_cache_health(&app_state.cache).await;

    // Check RPC
    let rpc_health = check_rpc_health(&app_state.rpc_client).await;

    // Overall status
    let overall_status = if db_health.status == "healthy"
        && cache_health.status != "unhealthy"
        && rpc_health.status != "unhealthy"
    {
        "healthy"
    } else if db_health.status == "unhealthy" {
        "unhealthy"
    } else {
        "degraded"
    };

    let health_status = HealthStatus {
        status: overall_status.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: start_time.elapsed().as_secs(),
        checks: HealthChecks {
            database: db_health,
            cache: cache_health,
            rpc: rpc_health,
        },
    };

    Json(health_status)
}

/// Database pool metrics endpoint
pub async fn pool_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = state.db.pool_metrics();
    Json(metrics)
}

pub async fn ingestion_status(
    State(app_state): State<AppState>,
) -> crate::error::ApiResult<Json<crate::ingestion::IngestionStatus>> {
    let status = app_state.ingestion.get_ingestion_status().await?;
    Ok(Json(status))
}
