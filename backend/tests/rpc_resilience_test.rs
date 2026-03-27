use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use stellar_insights_backend::api::anchors::{get_anchor_metrics_with_rpc, AnchorMetrics};
use stellar_insights_backend::models::asset_verification::VerificationResult;
use stellar_insights_backend::rpc::stellar::StellarRpcClient;
use stellar_insights_backend::rpc::{CircuitBreaker, CircuitBreakerConfig};
use stellar_insights_backend::rpc::error::{with_retry, RetryConfig, RpcError};

#[tokio::test]
async fn test_rpc_retry_on_failure() {
    // Test that with_retry retries on transient failures
    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = Arc::clone(&call_count);
    
    let result: Result<String, RpcError> = with_retry(
        move || {
            let call_count = Arc::clone(&call_count_clone);
            async move {
                let current = call_count.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 3 {
                    // Fail first 2 attempts
                    Err(RpcError::NetworkError("transient failure".to_string()))
                } else {
                    Ok("success".to_string())
                }
            }
        },
        RetryConfig {
            max_attempts: 5,
            base_delay_ms: 1,
            max_delay_ms: 100,
        },
        Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default(), "test")),
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
    assert_eq!(call_count.load(Ordering::SeqCst), 3); // Should have called 3 times: 2 failures + 1 success
}

#[tokio::test]
async fn test_circuit_breaker_opens_on_failures() {
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        CircuitBreakerConfig {
            failure_threshold: 2,
            timeout_duration: Duration::from_millis(1000),
            ..Default::default()
        },
        "test",
    ));

    // First failure
    let result1: Result<String, RpcError> = circuit_breaker.call(|| async { Err(RpcError::NetworkError("fail".to_string())) }).await;
    assert!(result1.is_err());

    // Second failure - should open circuit
    let result2: Result<String, RpcError> = circuit_breaker.call(|| async { Err(RpcError::NetworkError("fail".to_string())) }).await;
    assert!(result2.is_err());

    // Third call should fail fast due to open circuit
    let result3: Result<String, RpcError> = circuit_breaker.call(|| async { Ok("success".to_string()) }).await;
    assert!(result3.is_err());

    // Wait for recovery timeout
    sleep(Duration::from_millis(1100)).await;

    // Should allow call again
    let result4: Result<String, RpcError> = circuit_breaker.call(|| async { Ok("recovered".to_string()) }).await;
    assert!(result4.is_ok());
}

#[tokio::test]
async fn test_anchor_metrics_with_retry() {
    let client = StellarRpcClient::new_with_defaults(true);
    let anchor_id = Uuid::new_v4();

    // Test the new function with retry
    let metrics = get_anchor_metrics_with_rpc(
        anchor_id,
        Arc::new(client),
    )
    .await;

    assert!(metrics.is_ok());
    let metrics = metrics.unwrap();
    assert_eq!(metrics.anchor_id, anchor_id);
    assert!(metrics.total_payments > 0);
}