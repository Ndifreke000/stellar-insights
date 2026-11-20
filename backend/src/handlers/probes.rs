//! Kubernetes probes (#2828).
//!
//! `/livez` only says the process is serving requests, so a pod is restarted
//! only when it is actually wedged. `/readyz` gates traffic on *critical*
//! dependencies (database, ingestion lag). Optional dependencies (Redis
//! cache, Stellar RPC) only set `degraded`, so a Redis blip no longer takes
//! every pod out of rotation at once. `/health` stays the detailed report.

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ReadinessStatus {
    pub ready: bool,
    pub degraded: bool,
    /// Names of failing critical dependencies.
    pub failing_critical: Vec<&'static str>,
    /// Names of failing optional dependencies.
    pub failing_optional: Vec<&'static str>,
}

/// Results of the individual dependency checks behind `/readyz`.
#[derive(Debug, Clone, Copy)]
pub struct DependencyChecks {
    pub database: bool,
    pub ingestion_within_lag: bool,
    pub cache: bool,
    pub rpc: bool,
}

/// Classify dependency results: critical failures make the pod unready,
/// optional failures only mark it degraded.
pub fn evaluate_readiness(checks: DependencyChecks) -> ReadinessStatus {
    let critical = [
        ("database", checks.database),
        ("ingestion_lag", checks.ingestion_within_lag),
    ];
    let optional = [("cache", checks.cache), ("rpc", checks.rpc)];
    let failing = |deps: &[(&'static str, bool)]| -> Vec<&'static str> {
        deps.iter()
            .filter(|(_, ok)| !ok)
            .map(|(name, _)| *name)
            .collect()
    };
    let failing_critical = failing(&critical);
    let failing_optional = failing(&optional);
    ReadinessStatus {
        ready: failing_critical.is_empty(),
        degraded: !failing_optional.is_empty(),
        failing_critical,
        failing_optional,
    }
}

/// Liveness: the process is up and the router is answering.
pub async fn livez() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "alive" })),
    )
}

/// Readiness: 200 when critical dependencies are healthy (even if degraded),
/// 503 otherwise.
pub async fn readyz(State(app_state): State<AppState>) -> (StatusCode, Json<ReadinessStatus>) {
    let database = sqlx::query("SELECT 1")
        .fetch_one(app_state.db.pool())
        .await
        .is_ok();
    let cache = app_state.cache.ping().await.is_ok();
    let rpc = app_state.rpc_client.check_health().await.is_ok();
    let freshness =
        crate::health_check_enhanced::EnhancedHealthChecker::check_ingestion_lag_enhanced(
            &app_state.db,
            app_state.network_context.network,
            crate::health_check_enhanced::DEFAULT_MAX_INGESTION_LAG_SECONDS,
        )
        .await;
    let ingestion_within_lag = freshness
        .lag_seconds
        .map_or(true, |s| s <= freshness.max_lag_seconds);

    let status = evaluate_readiness(DependencyChecks {
        database,
        ingestion_within_lag,
        cache,
        rpc,
    });
    let code = if status.ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_OK: DependencyChecks = DependencyChecks {
        database: true,
        ingestion_within_lag: true,
        cache: true,
        rpc: true,
    };

    #[test]
    fn all_healthy_is_ready_and_not_degraded() {
        let s = evaluate_readiness(ALL_OK);
        assert!(s.ready && !s.degraded);
        assert!(s.failing_critical.is_empty() && s.failing_optional.is_empty());
    }

    #[test]
    fn redis_down_is_ready_but_degraded() {
        let s = evaluate_readiness(DependencyChecks {
            cache: false,
            ..ALL_OK
        });
        assert!(s.ready);
        assert!(s.degraded);
        assert_eq!(s.failing_optional, vec!["cache"]);
    }

    #[test]
    fn rpc_down_is_ready_but_degraded() {
        let s = evaluate_readiness(DependencyChecks {
            rpc: false,
            ..ALL_OK
        });
        assert!(s.ready && s.degraded);
    }

    #[test]
    fn database_down_is_not_ready() {
        let s = evaluate_readiness(DependencyChecks {
            database: false,
            ..ALL_OK
        });
        assert!(!s.ready);
        assert_eq!(s.failing_critical, vec!["database"]);
    }

    #[test]
    fn excessive_ingestion_lag_is_not_ready() {
        let s = evaluate_readiness(DependencyChecks {
            ingestion_within_lag: false,
            cache: false,
            ..ALL_OK
        });
        assert!(!s.ready && s.degraded);
        assert_eq!(s.failing_critical, vec!["ingestion_lag"]);
    }
}
