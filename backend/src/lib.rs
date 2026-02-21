pub mod analytics;
pub mod api;
pub mod auth;
pub mod auth_middleware;
pub mod broadcast;
pub mod cache;
pub mod cache_invalidation;
pub mod cache_middleware;
pub mod database;
pub mod db;
pub mod request_signing_middleware;
// pub mod email;  // Commented out - missing lettre dependency
pub mod env_config;
pub mod handlers;
pub mod http_cache;
pub mod ingestion;
pub mod ml;
pub mod ml_handlers;
pub mod models;
pub mod muxed;
pub mod network;
pub mod openapi;
pub mod rate_limit;
pub mod request_id;
pub mod services;
pub mod shutdown;
pub mod snapshot;
pub mod snapshot_handlers;
pub mod state;
pub mod vault;
pub mod webhooks;
pub mod websocket;

pub mod rpc;
pub mod rpc_handlers;

#[cfg(test)]
mod ml_tests;
