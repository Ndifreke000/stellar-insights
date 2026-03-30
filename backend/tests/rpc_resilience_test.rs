use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use failsafe::futures::CircuitBreaker as _;
use failsafe::{backoff, failure_policy, Config};
use tokio::time::sleep;
use uuid::Uuid;

use stellar_insights_backend::api::anchors::{get_anchor_metrics_with_fallback, AnchorMetrics};
use stellar_insights_backend::cache::{CacheConfig, CacheManager};
use stellar_insights_backend::rpc::circuit_breaker::{
    rpc_circuit_breaker, CircuitBreaker, SharedCircuitBreaker,
};
use stellar_insights_backend::rpc::error::{with_retry, RetryConfig, RpcError};
use stellar_insights_backend::rpc::stellar::StellarRpcClient;

fn test_circuit_breaker(failure_threshold: u32, timeout: Duration) -> SharedCircuitBreaker {
    let backoff = backoff::constant(timeout);
    let policy = failure_policy::consecutive_failures(failure_threshold, backoff);
    let breaker: CircuitBreaker = Config::new().failure_policy(policy).build();
    Arc::new(breaker)
}

#[tokio::test]
async fn test_rpc_retry_on_failure() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = Arc::clone(&call_count);

    let result: Result<String, RpcError> = with_retry(
        move || {
            let call_count = Arc::clone(&call_count_clone);
            async move {
                let current = call_count.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 3 {
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
        test_circuit_breaker(5, Duration::from_secs(30)),
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_circuit_breaker_opens_on_failures() {
    let circuit_breaker = test_circuit_breaker(2, Duration::from_millis(1000));
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        CircuitBreakerConfig {
            failure_threshold: 2,
            timeout_duration: Duration::from_millis(100),
            ..Default::default()
        },
        "test",
    ));

    let result1: Result<String, failsafe::Error<RpcError>> = circuit_breaker
        .call(async { Err(RpcError::NetworkError("fail".to_string())) })
        .await;
    assert!(matches!(result1, Err(failsafe::Error::Inner(_))));

    let result2: Result<String, failsafe::Error<RpcError>> = circuit_breaker
        .call(async { Err(RpcError::NetworkError("fail".to_string())) })
        .await;
    assert!(matches!(result2, Err(failsafe::Error::Inner(_))));

    let result3: Result<String, failsafe::Error<RpcError>> = circuit_breaker
        .call(async { Ok("success".to_string()) })
        .await;
    assert!(matches!(result3, Err(failsafe::Error::Rejected)));

    sleep(Duration::from_millis(1100)).await;
    // Wait for recovery timeout with generous margin
    sleep(Duration::from_millis(300)).await;

    let result4: Result<String, failsafe::Error<RpcError>> = circuit_breaker
        .call(async { Ok("recovered".to_string()) })
        .await;
    assert_eq!(result4.unwrap(), "recovered");
}

#[tokio::test]
async fn test_circuit_breaker_fallback() {
    let anchor_id = Uuid::new_v4();
    let client = StellarRpcClient::new_with_defaults(true);
    let cache = Arc::new(CacheManager::new_in_memory_for_tests(CacheConfig::default()));

    let circuit_breaker = rpc_circuit_breaker();
    while matches!(
        circuit_breaker
            .call(async { Err::<(), RpcError>(RpcError::NetworkError("fail".to_string())) })
            .await,
        Err(failsafe::Error::Inner(_))
    ) {}

    let fallback = AnchorMetrics {
        anchor_id,
        total_payments: 10,
        successful_payments: 8,
        failed_payments: 2,
        total_volume: 12345.6,
    };
    cache
        .set(&format!("anchor_metrics:{}", anchor_id), &fallback, 60)
        .await
        .unwrap();

    let metrics = get_anchor_metrics_with_fallback(anchor_id, Arc::new(client), cache)
        .await
        .unwrap();

    assert_eq!(metrics.anchor_id, anchor_id);
    assert_eq!(metrics.total_payments, fallback.total_payments);
}
