use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use failsafe::futures::CircuitBreaker as _;
use failsafe::{backoff, failure_policy, Config};
use tokio::time;
use uuid::Uuid;

use payraider_backend::api::anchors::{get_anchor_metrics_with_fallback, AnchorMetrics};
use payraider_backend::cache::{CacheConfig, CacheManager};
use payraider_backend::rpc::circuit_breaker::{
    rpc_circuit_breaker, CircuitBreaker, SharedCircuitBreaker,
};
use payraider_backend::rpc::error::{with_retry, RetryConfig, RpcError};
use payraider_backend::rpc::stellar::StellarRpcClient;

fn test_circuit_breaker(failure_threshold: u32, timeout: Duration) -> SharedCircuitBreaker {
    let backoff = backoff::constant(timeout);
    let policy = failure_policy::consecutive_failures(failure_threshold, backoff);
    let breaker: CircuitBreaker = Config::new().failure_policy(policy).build();
    Arc::new(breaker)
}

#[tokio::test(start_paused = true)]
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

    tokio::task::yield_now().await;
    // `with_retry` uses `tokio::time::sleep`; advance past backoffs (1ms + 2ms + …).
    time::advance(Duration::from_secs(1)).await;

    let result: Result<String, RpcError> = join.await.expect("join with_retry task");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_circuit_breaker_opens_on_failures() {
    // Two consecutive Inner failures open the circuit; backoff duration comes from the policy.
    // Failsafe uses `std::time::Instant` (`clock::now()`), not Tokio's paused clock, so
    // `start_paused` + `time::advance` does not elapse the open window — use wall-clock sleep.
    let circuit_breaker = test_circuit_breaker(2, Duration::from_millis(100));

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

    time::sleep(Duration::from_millis(200)).await;

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
    let mut tripped = false;
    for _ in 0..128 {
        let r: Result<(), failsafe::Error<RpcError>> = circuit_breaker
            .call(async { Err(RpcError::NetworkError("fail".to_string())) })
            .await;
        if matches!(r, Err(failsafe::Error::Rejected)) {
            tripped = true;
            break;
        }
    }
    assert!(
        tripped,
        "expected global circuit breaker to open after repeated failures"
    );

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

    // Best-effort: nudge shared breaker toward closed so other tests are not starved.
    for _ in 0..8 {
        let _: Result<(), failsafe::Error<RpcError>> = circuit_breaker.call(async { Ok(()) }).await;
    }
}
