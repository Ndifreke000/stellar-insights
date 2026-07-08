pub mod admin_audit_log;
pub mod alerts;
pub mod analytics;
pub mod api;
pub mod api_analytics_middleware;
pub mod api_deprecation_middleware;
pub mod api_v1_middleware;
pub mod deprecation_middleware;
pub mod distributed_lock;
pub mod monitor;

pub mod auth;
pub mod auth_middleware;
pub mod backup;
pub mod broadcast;
pub mod cache;
pub mod cache_invalidation;
// cache_middleware removed in favor of cache helper APIs
pub mod crypto;
pub mod database;

pub mod db;
pub mod email;
pub mod env_config;
pub mod error;
pub mod features;
pub mod handlers; // Core handlers (pool_metrics, health_check, ingestion_status)
pub mod health_check_enhanced; // Enhanced health check with mobile support
pub mod http_cache; // HTTP caching layer (ETag/conditional responses)
pub mod ingestion;
pub mod ip_whitelist_middleware;
pub mod jobs;
pub mod logging;
pub mod ml;
pub mod models;
pub mod muxed;
pub mod request_signing_middleware;

pub mod multi_network;
pub mod network;
pub mod observability;
pub mod openapi;
pub mod pagination;
pub mod payload_limit;
pub mod rate_limit;
pub mod replay;
pub mod request_id;
pub mod services;
pub mod shutdown;
pub mod snapshot;
pub mod state;
pub mod validation;
pub mod vault;
pub mod webhooks;
pub mod websocket;

pub mod middleware;
pub mod rpc;

/// Tests that read or mutate process-global env vars (`STELLAR_NETWORK`,
/// `STELLAR_RPC_URL_MAINNET`, etc.) race under `cargo test`'s default
/// multi-threaded runner, since `std::env::set_var`/`remove_var` affect the
/// whole process. Acquiring this lock serializes them against each other and
/// ensures the mainnet RPC vars are set before `NetworkConfig::for_network`
/// panics on a fresh/parallel run where no other test has set them yet.
#[cfg(test)]
pub(crate) fn lock_env_test() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if std::env::var("STELLAR_RPC_URL_MAINNET").is_err() {
        std::env::set_var("STELLAR_RPC_URL_MAINNET", "https://rpc.example.com");
    }
    if std::env::var("STELLAR_HORIZON_URL_MAINNET").is_err() {
        std::env::set_var("STELLAR_HORIZON_URL_MAINNET", "https://horizon.example.com");
    }
    guard
}
