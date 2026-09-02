//! Integration tests for previously unmounted modules and endpoints (#2219):
//! - corridor_alerts
//! - admin_ip_whitelist
//! - audit_log
//! - twofa
//! - settlement_distribution
//! - failed_payments

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::util::ServiceExt;

use payraider_backend::api::{admin_ip_whitelist, audit_log, corridor_alerts, failed_payments, settlement_distribution, twofa};
use payraider_backend::admin_audit_log::AdminAuditLogger;
use payraider_backend::admin_ip_whitelist::IpWhitelistService;
use payraider_backend::cache::{CacheConfig, CacheManager};
use payraider_backend::crypto::CryptoService;
use payraider_backend::database::Database;
use payraider_backend::ingestion::DataIngestionService;
use payraider_backend::rpc::StellarRpcClient;
use payraider_backend::state::AppState;
use payraider_backend::twofa::TwoFAService;
use payraider_backend::websocket::WsState;

const TEST_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS corridor_alert_configs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    corridor_key TEXT,
    name TEXT NOT NULL,
    success_rate_threshold REAL,
    latency_threshold_ms REAL,
    liquidity_threshold_usd REAL,
    success_rate_drop_pct REAL DEFAULT 10.0,
    latency_increase_pct REAL DEFAULT 50.0,
    liquidity_drop_pct REAL DEFAULT 30.0,
    cooldown_seconds INTEGER DEFAULT 300,
    notify_email BOOLEAN NOT NULL DEFAULT 0,
    notify_webhook BOOLEAN NOT NULL DEFAULT 0,
    notify_in_app BOOLEAN NOT NULL DEFAULT 1,
    notify_slack BOOLEAN NOT NULL DEFAULT 0,
    notify_telegram BOOLEAN NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    last_triggered_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS corridor_performance_snapshots (
    id TEXT PRIMARY KEY,
    corridor_key TEXT NOT NULL,
    source_asset_code TEXT NOT NULL,
    source_asset_issuer TEXT NOT NULL,
    destination_asset_code TEXT NOT NULL,
    destination_asset_issuer TEXT NOT NULL,
    success_rate REAL NOT NULL,
    avg_settlement_latency_ms REAL NOT NULL,
    liquidity_depth_usd REAL NOT NULL,
    volume_usd REAL NOT NULL,
    total_transactions INTEGER NOT NULL DEFAULT 0,
    successful_transactions INTEGER NOT NULL DEFAULT 0,
    failed_transactions INTEGER NOT NULL DEFAULT 0,
    snapshot_time TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS corridor_alert_events (
    id TEXT PRIMARY KEY,
    config_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    corridor_key TEXT NOT NULL,
    alert_type TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'warning',
    message TEXT NOT NULL,
    old_value REAL,
    new_value REAL,
    threshold_value REAL,
    acknowledged BOOLEAN NOT NULL DEFAULT 0,
    acknowledged_at TEXT,
    triggered_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS admin_ip_whitelist (
    id TEXT PRIMARY KEY,
    ip_or_cidr TEXT NOT NULL,
    description TEXT,
    added_by_user_id TEXT,
    added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS admin_audit_log (
    id TEXT PRIMARY KEY,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    user_id TEXT NOT NULL,
    status TEXT NOT NULL,
    details TEXT NOT NULL,
    hash TEXT NOT NULL,
    session_id TEXT,
    device_user_agent TEXT,
    ip_address TEXT,
    event_type VARCHAR(50)
);

CREATE TABLE IF NOT EXISTS corridor_metrics (
    id TEXT PRIMARY KEY,
    corridor_key TEXT NOT NULL,
    date TEXT NOT NULL,
    total_transactions INTEGER DEFAULT 0,
    successful_transactions INTEGER DEFAULT 0,
    failed_transactions INTEGER DEFAULT 0,
    total_volume_usd REAL DEFAULT 0,
    avg_latency_ms REAL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS user_2fa_secrets (
    user_id TEXT PRIMARY KEY,
    encrypted_secret TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL,
    enrolled_at TIMESTAMP,
    backup_codes_generated_at TIMESTAMP
);

CREATE TABLE IF NOT EXISTS user_2fa_backup_codes (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    hashed_code TEXT NOT NULL UNIQUE,
    used_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
"#;

async fn setup_test_pool() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("in-memory pool");
    sqlx::query(TEST_SCHEMA)
        .execute(&pool)
        .await
        .expect("test schema created");
    pool
}

async fn make_test_app_state(pool: SqlitePool) -> AppState {
    if std::env::var("STELLAR_RPC_URL_MAINNET").is_err() {
        std::env::set_var("STELLAR_RPC_URL_MAINNET", "https://rpc.example.com");
    }
    if std::env::var("STELLAR_HORIZON_URL_MAINNET").is_err() {
        std::env::set_var("STELLAR_HORIZON_URL_MAINNET", "https://horizon.example.com");
    }
    let db = Arc::new(Database::new(pool));
    let ws_state = Arc::new(WsState::new());
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let ingestion = Arc::new(DataIngestionService::new(rpc_client.clone(), db.clone()));
    let cache = Arc::new(CacheManager::new(CacheConfig::default()).await.unwrap());
    AppState::new(db, cache, ws_state, ingestion, rpc_client)
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).expect("valid JSON")
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_corridor_alerts_snapshots_and_summary_endpoints() {
    let pool = setup_test_pool().await;
    let app_state = make_test_app_state(pool).await;

    let app = Router::new()
        .nest("/api/v1/corridor-alerts", corridor_alerts::routes(app_state.clone()));

    // Test snapshots endpoint
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/corridor-alerts/snapshots")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body.is_array());

    // Test summary endpoint
    let resp_summary = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/corridor-alerts/summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp_summary.status(), StatusCode::OK);
    let body_summary = json_body(resp_summary).await;
    assert!(body_summary.is_array());
}

#[tokio::test]
async fn test_admin_ip_whitelist_endpoints() {
    let pool = setup_test_pool().await;
    let service = Arc::new(IpWhitelistService::new(pool));

    let app = Router::new()
        .nest("/api/v1/admin/ip-whitelist", admin_ip_whitelist::routes(service));

    // 1. Check list initially empty
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/ip-whitelist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["count"], 0);

    // 2. Add an IP to whitelist
    let add_req = json!({
        "ip_or_cidr": "192.168.1.100",
        "description": "Internal test node"
    });
    let resp_add = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/ip-whitelist")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&add_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp_add.status(), StatusCode::CREATED);

    // 3. Verify IP check endpoint allows whitelisted IP
    let check_req = json!({ "ip": "192.168.1.100" });
    let resp_check = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/ip-whitelist/check")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&check_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp_check.status(), StatusCode::OK);
    let body_check = json_body(resp_check).await;
    assert_eq!(body_check["access"], "allowed");
    assert_eq!(body_check["is_whitelisted"], true);
}

#[tokio::test]
async fn test_admin_audit_log_endpoints() {
    let pool = setup_test_pool().await;
    let logger = Arc::new(AdminAuditLogger::new(pool));

    // Log an action
    logger
        .log_action(
            "test_action",
            "test_resource",
            "admin_user",
            "success",
            json!({"detail": "sample"}),
            None,
        )
        .await
        .unwrap();

    let app = Router::new()
        .nest("/api/v1/admin/audit-log", audit_log::routes(logger));

    // 1. Query audit log
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/audit-log")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["count"], 1);
    assert_eq!(body["entries"][0]["action"], "test_action");

    // 2. Verify integrity check
    let resp_verify = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/audit-log/verify-integrity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp_verify.status(), StatusCode::OK);
    let body_verify = json_body(resp_verify).await;
    assert_eq!(body_verify["is_valid"], true);
    assert_eq!(body_verify["total_entries"], 1);
}

#[tokio::test]
async fn test_settlement_distribution_endpoint() {
    let pool = setup_test_pool().await;
    let app_state = make_test_app_state(pool).await;

    let app = Router::new()
        .nest("/api/v1/analytics", settlement_distribution::routes(app_state));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/analytics/settlement-distribution")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body.get("corridors").is_some());
    assert!(body.get("trend").is_some());
    assert!(body.get("network_p50_ms").is_some());
}

#[tokio::test]
async fn test_failed_payments_endpoint() {
    let pool = setup_test_pool().await;
    let app_state = make_test_app_state(pool).await;

    let app = Router::new()
        .nest("/api/v1/analytics", failed_payments::routes(app_state));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/analytics/failed-payments")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body.get("breakdown").is_some());
    assert!(body.get("insights").is_some());
    assert!(body.get("top_failing_corridors").is_some());
}

#[tokio::test]
async fn test_twofa_routes_mounted() {
    let pool = setup_test_pool().await;
    let crypto = CryptoService::new_for_tests();
    let service = Arc::new(TwoFAService::new(pool, crypto));

    let app = Router::new()
        .nest("/api/v1/auth/2fa", twofa::routes(service));

    // Confirm router routes exist: post to backup code without auth returns 401 or expected status
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/2fa/regenerate-backup")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body.get("backup_codes").is_some());
}
