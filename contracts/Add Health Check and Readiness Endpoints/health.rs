use axum::{extract::State, http::StatusCode, response::Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Instant};

/// Shared application state containing dependency clients.
/// Adjust field types to match your actual application state.
pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub redis_client: redis::Client,
    pub rpc_url: String,
    pub version: String,
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckResult {
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Up,
    Down,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: ServiceStatus,
    pub timestamp: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadinessResponse {
    pub status: ServiceStatus,
    pub timestamp: String,
    pub version: String,
    pub checks: HashMap<String, CheckResult>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /health
///
/// Liveness probe — confirms the process is running and able to serve requests.
/// Returns 200 as long as the HTTP server itself is alive.
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<HealthResponse>) {
    let response = HealthResponse {
        status: ServiceStatus::Healthy,
        timestamp: Utc::now().to_rfc3339(),
        version: state.version.clone(),
    };
    (StatusCode::OK, Json(response))
}

/// GET /ready
///
/// Readiness probe — confirms all downstream dependencies (database, Redis, RPC)
/// are reachable.  Returns 200 only when every check passes; 503 otherwise.
pub async fn readiness_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<ReadinessResponse>) {
    let mut checks: HashMap<String, CheckResult> = HashMap::new();
    let mut all_up = true;

    // ── Database ──────────────────────────────────────────────────────────────
    let db_result = check_database(&state.db_pool).await;
    if matches!(db_result.status, CheckStatus::Down) {
        all_up = false;
    }
    checks.insert("database".to_string(), db_result);

    // ── Redis ─────────────────────────────────────────────────────────────────
    let redis_result = check_redis(&state.redis_client).await;
    if matches!(redis_result.status, CheckStatus::Down) {
        all_up = false;
    }
    checks.insert("redis".to_string(), redis_result);

    // ── RPC endpoint ──────────────────────────────────────────────────────────
    let rpc_result = check_rpc(&state.rpc_url).await;
    if matches!(rpc_result.status, CheckStatus::Down) {
        all_up = false;
    }
    checks.insert("rpc".to_string(), rpc_result);

    let (http_status, service_status) = if all_up {
        (StatusCode::OK, ServiceStatus::Healthy)
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, ServiceStatus::Unhealthy)
    };

    let response = ReadinessResponse {
        status: service_status,
        timestamp: Utc::now().to_rfc3339(),
        version: state.version.clone(),
        checks,
    };

    (http_status, Json(response))
}

// ── Dependency checks ─────────────────────────────────────────────────────────

async fn check_database(pool: &sqlx::PgPool) -> CheckResult {
    let start = Instant::now();
    match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => CheckResult {
            status: CheckStatus::Up,
            latency_ms: Some(start.elapsed().as_millis()),
            error: None,
        },
        Err(e) => CheckResult {
            status: CheckStatus::Down,
            latency_ms: Some(start.elapsed().as_millis()),
            error: Some(e.to_string()),
        },
    }
}

async fn check_redis(client: &redis::Client) -> CheckResult {
    let start = Instant::now();
    match client.get_async_connection().await {
        Ok(mut conn) => {
            use redis::AsyncCommands;
            match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                Ok(_) => CheckResult {
                    status: CheckStatus::Up,
                    latency_ms: Some(start.elapsed().as_millis()),
                    error: None,
                },
                Err(e) => CheckResult {
                    status: CheckStatus::Down,
                    latency_ms: Some(start.elapsed().as_millis()),
                    error: Some(e.to_string()),
                },
            }
        }
        Err(e) => CheckResult {
            status: CheckStatus::Down,
            latency_ms: Some(start.elapsed().as_millis()),
            error: Some(e.to_string()),
        },
    }
}

async fn check_rpc(rpc_url: &str) -> CheckResult {
    let start = Instant::now();
    // A lightweight JSON-RPC call (`net_version`) is used to probe the node
    // without causing any state changes.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "net_version",
        "params": [],
        "id": 1
    });

    match client.post(rpc_url).json(&payload).send().await {
        Ok(resp) if resp.status().is_success() => CheckResult {
            status: CheckStatus::Up,
            latency_ms: Some(start.elapsed().as_millis()),
            error: None,
        },
        Ok(resp) => CheckResult {
            status: CheckStatus::Down,
            latency_ms: Some(start.elapsed().as_millis()),
            error: Some(format!("unexpected status: {}", resp.status())),
        },
        Err(e) => CheckResult {
            status: CheckStatus::Down,
            latency_ms: Some(start.elapsed().as_millis()),
            error: Some(e.to_string()),
        },
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, routing::get, Router};
    use tower::ServiceExt; // for `oneshot`

    /// Build a minimal router wired up with mock/test state.
    ///
    /// In a real test suite you would spin up Docker containers (testcontainers
    /// crate) for Postgres and Redis, or use mocks.  Here we validate the
    /// routing layer and response shape with a real (or in-memory) pool.
    fn test_router(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/health", get(health_check))
            .route("/ready", get(readiness_check))
            .with_state(state)
    }

    #[tokio::test]
    async fn health_returns_200() {
        // Provide a real PgPool / redis::Client via env vars in CI,
        // or swap for mocks in unit tests.
        let state = Arc::new(AppState {
            db_pool: create_test_pool().await,
            redis_client: create_test_redis(),
            rpc_url: std::env::var("RPC_URL").unwrap_or_else(|_| "http://localhost:8545".into()),
            version: "0.1.0-test".into(),
        });

        let app = test_router(state);
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: HealthResponse = serde_json::from_slice(&body).unwrap();
        assert!(matches!(json.status, ServiceStatus::Healthy));
    }

    #[tokio::test]
    async fn readiness_returns_structured_json() {
        let state = Arc::new(AppState {
            db_pool: create_test_pool().await,
            redis_client: create_test_redis(),
            rpc_url: std::env::var("RPC_URL").unwrap_or_else(|_| "http://localhost:8545".into()),
            version: "0.1.0-test".into(),
        });

        let app = test_router(state);
        let response = app
            .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // We only assert the body is valid JSON with the expected keys;
        // the actual status (200 vs 503) depends on whether deps are running.
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ReadinessResponse = serde_json::from_slice(&body).unwrap();
        assert!(json.checks.contains_key("database"));
        assert!(json.checks.contains_key("redis"));
        assert!(json.checks.contains_key("rpc"));
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    async fn create_test_pool() -> sqlx::PgPool {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/testdb".into());
        sqlx::PgPool::connect(&url)
            .await
            .expect("failed to connect to test database")
    }

    fn create_test_redis() -> redis::Client {
        let url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".into());
        redis::Client::open(url).expect("failed to create redis client")
    }
}
