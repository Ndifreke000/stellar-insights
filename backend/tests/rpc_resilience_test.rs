use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use payraider_backend::api::anchors::{get_anchor_metrics_with_fallback, AnchorMetrics};
use payraider_backend::cache::{CacheConfig, CacheManager};
use payraider_backend::rpc::circuit_breaker::{
    rpc_circuit_breaker, CallError, CircuitBreaker, CircuitBreakerConfig, ManualClock,
    SharedCircuitBreaker,
};
use payraider_backend::rpc::error::{with_retry, RetryConfig, RpcError};
use payraider_backend::rpc::stellar::StellarRpcClient;

fn test_circuit_breaker_with_clock(
    failure_threshold: u32,
    timeout: Duration,
    clock: Arc<ManualClock>,
) -> SharedCircuitBreaker {
    Arc::new(CircuitBreaker::with_clock(
        "test",
        CircuitBreakerConfig {
            failure_threshold,
            success_threshold: 1,
            timeout_duration: timeout,
            half_open_max_calls: 1,
        },
        clock,
    ))
}

fn test_circuit_breaker(failure_threshold: u32, timeout: Duration) -> SharedCircuitBreaker {
    test_circuit_breaker_with_clock(failure_threshold, timeout, Arc::new(ManualClock::new()))
}

#[tokio::test(flavor = "current_thread")]
async fn test_rpc_retry_on_failure() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = Arc::clone(&call_count);

    let join = tokio::spawn(async move {
        with_retry(
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
        .await
    });

    // Wait for the task to complete with retries
    // In current_thread mode, the task will run to completion
    let result: Result<String, RpcError> = join.await.expect("join with_retry task");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_circuit_breaker_opens_on_failures() {
    // Deterministic: the breaker reads time from a manually advanced clock.
    let clock = Arc::new(ManualClock::new());
    let circuit_breaker =
        test_circuit_breaker_with_clock(2, Duration::from_millis(100), clock.clone());

    // Fail twice to trip the circuit
    let result1: Result<String, CallError<RpcError>> = circuit_breaker
        .call(async { Err(RpcError::NetworkError("fail".to_string())) })
        .await;
    assert!(matches!(result1, Err(CallError::Inner(_))));

    let result2: Result<String, CallError<RpcError>> = circuit_breaker
        .call(async { Err(RpcError::NetworkError("fail".to_string())) })
        .await;
    assert!(matches!(result2, Err(CallError::Inner(_))));

    // Circuit should now be open
    let result3: Result<String, CallError<RpcError>> = circuit_breaker
        .call(async { Ok("success".to_string()) })
        .await;
    assert!(matches!(result3, Err(CallError::Rejected)));

    // Wait for the circuit to transition to half-open
    clock.advance(Duration::from_millis(150));

    // In half-open state, success should close the circuit
    let result4: Result<String, CallError<RpcError>> = circuit_breaker
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

    // Open the circuit with repeated failures
    let mut tripped = false;
    for _ in 0..128 {
        let r: Result<(), CallError<RpcError>> = circuit_breaker
            .call(async { Err(RpcError::NetworkError("fail".to_string())) })
            .await;
        if matches!(r, Err(CallError::Rejected)) {
            tripped = true;
            break;
        }
    }
    assert!(
        tripped,
        "expected global circuit breaker to open after repeated failures"
    );

    // Set up fallback data in cache
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

    // Verify fallback works when circuit is open
    let metrics = get_anchor_metrics_with_fallback(anchor_id, Arc::new(client), cache)
        .await
        .unwrap();

    assert_eq!(metrics.anchor_id, anchor_id);
    assert_eq!(metrics.total_payments, fallback.total_payments);

    // Best-effort: nudge shared breaker toward closed so other tests are not starved.
    // In half-open state, one success closes the circuit.
    for _ in 0..8 {
        let _: Result<(), CallError<RpcError>> = circuit_breaker.call(async { Ok(()) }).await;
    }
}
