//! Integration tests for cost-based rate limiting.

use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use stellar_insights_backend::{
    middleware::cost_rate_limiter::{CostRateLimiter, extract_client_key, get_client_tier},
    models::cost::{CostConfig, CostHeaders},
    state::AppState,
};

/// Mock AppState for testing
fn mock_app_state() -> AppState {
    // For testing, we just need the cost_rate_limiter field
    AppState::new(
        std::sync::Arc::new(stellar_insights_backend::database::Database::new(
            sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap(),
        )),
        std::sync::Arc::new(stellar_insights_backend::cache::CacheManager::new_in_memory_for_tests(
            stellar_insights_backend::cache::CacheConfig::default(),
        )),
        std::sync::Arc::new(stellar_insights_backend::websocket::WsState::new()),
        std::sync::Arc::new(stellar_insights_backend::ingestion::DataIngestionService::new(
            std::sync::Arc::new(stellar_insights_backend::rpc::StellarRpcClient::new(
                "https://horizon.stellar.org",
            )),
            std::sync::Arc::new(stellar_insights_backend::database::Database::new(
                sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap(),
            )),
        )),
        std::sync::Arc::new(stellar_insights_backend::rpc::StellarRpcClient::new(
            "https://horizon.stellar.org",
        )),
    )
}

#[tokio::test]
async fn test_cost_estimate_headers() {
    let config = CostConfig::default();
    let state = stellar_insights_backend::models::cost::ClientCostState::new(100.0, stellar_insights_backend::models::cost::ClientTier::Authenticated);
    
    let cost_headers = CostHeaders::from_state(&state, 40.0);
    
    assert_eq!(cost_headers.cost_estimate, 40.0);
    assert_eq!(cost_headers.remaining_budget, 60.0);
    assert_eq!(cost_headers.total_budget, 1000.0);
    assert_eq!(cost_headers.tier, "Authenticated");
}

#[tokio::test]
async fn test_cost_calculation() {
    let config = CostConfig::default();
    
    // Single corridor, 30 days = 10 + 30 = 40
    let cost = config.export_corridor_single_cost(30);
    assert!((cost - 40.0).abs() < f64::EPSILON);
    
    // Single corridor, 365 days = 10 + 365 = 375
    let cost = config.export_corridor_single_cost(365);
    assert!((cost - 375.0).abs() < f64::EPSILON);
    
    // All corridors, 1 day = 10000 + 1 = 10001
    let cost = config.export_corridor_all_cost(1);
    assert!((cost - 10001.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn test_client_budgets() {
    let config = CostConfig::default();
    
    assert_eq!(
        stellar_insights_backend::models::cost::ClientTier::Anonymous.budget(&config),
        100.0
    );
    assert_eq!(
        stellar_insights_backend::models::cost::ClientTier::Authenticated.budget(&config),
        1000.0
    );
    assert_eq!(
        stellar_insights_backend::models::cost::ClientTier::Premium.budget(&config),
        10000.0
    );
}

#[tokio::test]
async fn test_cost_rate_limiter_debit() {
    let limiter = CostRateLimiter::new();
    
    // Debit within budget
    let result = limiter
        .debit("test-client", stellar_insights_backend::models::cost::ClientTier::Authenticated, 100.0)
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap());
    
    // Debit beyond remaining budget
    let result = limiter
        .debit("test-client", stellar_insights_backend::models::cost::ClientTier::Authenticated, 1000.0)
        .await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[tokio::test]
async fn test_client_key_extraction() {
    assert_eq!(
        extract_client_key(Some("api_key_123"), None, None),
        "apikey:api_key_123"
    );
    assert_eq!(
        extract_client_key(None, Some("user_456"), None),
        "user:user_456"
    );
    assert_eq!(
        extract_client_key(None, None, Some("192.168.1.1")),
        "ip:192.168.1.1"
    );
    assert_eq!(extract_client_key(None, None, None), "anonymous");
}

#[tokio::test]
async fn test_client_tier() {
    assert_eq!(
        get_client_tier(Some("api_key"), None),
        stellar_insights_backend::models::cost::ClientTier::Authenticated
    );
    assert_eq!(
        get_client_tier(None, Some("user_id")),
        stellar_insights_backend::models::cost::ClientTier::Authenticated
    );
    assert_eq!(
        get_client_tier(None, None),
        stellar_insights_backend::models::cost::ClientTier::Anonymous
    );
}

#[tokio::test]
async fn test_budget_insufficient_error_response() {
    let limiter = CostRateLimiter::new();
    let app_state = mock_app_state();
    
    // Exhaust anonymous budget (100 units)
    let result = limiter
        .debit("anonymous-client", stellar_insights_backend::models::cost::ClientTier::Anonymous, 100.0)
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap());
    
    // Try to debit more
    let result = limiter
        .debit("anonymous-client", stellar_insights_backend::models::cost::ClientTier::Anonymous, 50.0)
        .await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}
