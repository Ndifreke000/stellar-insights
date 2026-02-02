---
title: "[ENHANCEMENT] Insufficient Error Handling and Retry Logic for RPC Failures"
labels: enhancement, high, rpc, reliability, backend
assignees: ''
---

## 💡 Description

The current RPC error handling silently falls back to empty arrays when API calls fail. There's basic exponential backoff but no sophisticated retry logic, circuit breakers, or error categorization.

## 📊 Current Behavior

```rust
let payments = match rpc_client.fetch_payments(200, None).await {
    Ok(p) => p,
    Err(e) => {
        tracing::error!("Failed to fetch payments from RPC: {}", e);
        return Ok(vec![]);  // Silent failure
    }
};
```

**Problems:**
- Silent failures hide issues
- No distinction between transient and permanent errors
- No circuit breaker for failing endpoints
- No alerting on repeated failures
- Users see empty data without explanation
- No structured error logging

## ✅ Expected Behavior

- Categorize errors (network, rate limit, server error, etc.)
- Implement smart retry with backoff
- Add circuit breaker pattern
- Return meaningful errors to frontend
- Log structured error data
- Alert on repeated failures
- Provide fallback strategies

## 📁 Affected Files

- `backend/src/rpc/stellar.rs` (all fetch methods)
- `backend/src/api/corridors_cached.rs`
- `backend/src/api/anchors_cached.rs`
- New file: `backend/src/rpc/error.rs`
- New file: `backend/src/rpc/circuit_breaker.rs`

## 📋 Acceptance Criteria

- [ ] Create custom error types for RPC failures
- [ ] Implement circuit breaker pattern
- [ ] Add error categorization
- [ ] Implement smart retry logic
- [ ] Add structured error logging
- [ ] Return errors to frontend when appropriate
- [ ] Add metrics for error rates
- [ ] Document error handling strategy
- [ ] Add tests for error scenarios

## 🚀 Implementation Steps

### 1. Create Custom Error Types

```rust
// backend/src/rpc/error.rs
#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Rate limit exceeded. Retry after {retry_after:?}")]
    RateLimitError { retry_after: Option<Duration> },
    
    #[error("Server error: {status} - {message}")]
    ServerError { status: u16, message: String },
    
    #[error("Failed to parse response: {0}")]
    ParseError(String),
    
    #[error("Request timeout after {0:?}")]
    TimeoutError(Duration),
    
    #[error("Circuit breaker open")]
    CircuitBreakerOpen,
}

impl RpcError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            RpcError::NetworkError(_) | 
            RpcError::TimeoutError(_) |
            RpcError::ServerError { status: 500..=599, .. }
        )
    }
    
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            RpcError::RateLimitError { retry_after } => *retry_after,
            _ => None,
        }
    }
}
```

### 2. Implement Circuit Breaker

```rust
// backend/src/rpc/circuit_breaker.rs
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    config: CircuitBreakerConfig,
}

#[derive(Debug, Clone)]
enum CircuitState {
    Closed { failure_count: u32 },
    Open { opened_at: Instant },
    HalfOpen { success_count: u32 },
}

pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,      // 5 failures to open
    pub success_threshold: u32,      // 2 successes to close
    pub timeout_duration: Duration,  // 30 seconds
    pub half_open_max_calls: u32,   // 3 test calls
}

impl CircuitBreaker {
    pub async fn call<F, T>(&self, f: F) -> Result<T, RpcError>
    where
        F: Future<Output = Result<T, RpcError>>,
    {
        // Check if circuit is open
        if self.is_open().await {
            return Err(RpcError::CircuitBreakerOpen);
        }
        
        // Execute request
        match f.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) if e.is_retryable() => {
                self.on_failure().await;
                Err(e)
            }
            Err(e) => Err(e),
        }
    }
    
    async fn is_open(&self) -> bool {
        let state = self.state.lock().await;
        matches!(*state, CircuitState::Open { .. })
    }
    
    async fn on_success(&self) {
        let mut state = self.state.lock().await;
        *state = match *state {
            CircuitState::HalfOpen { success_count } => {
                if success_count + 1 >= self.config.success_threshold {
                    CircuitState::Closed { failure_count: 0 }
                } else {
                    CircuitState::HalfOpen { success_count: success_count + 1 }
                }
            }
            _ => CircuitState::Closed { failure_count: 0 },
        };
    }
    
    async fn on_failure(&self) {
        let mut state = self.state.lock().await;
        *state = match *state {
            CircuitState::Closed { failure_count } => {
                if failure_count + 1 >= self.config.failure_threshold {
                    CircuitState::Open { opened_at: Instant::now() }
                } else {
                    CircuitState::Closed { failure_count: failure_count + 1 }
                }
            }
            _ => *state,
        };
    }
}
```

### 3. Implement Smart Retry Logic

```rust
pub async fn retry_with_backoff<F, T>(
    mut f: F,
    max_retries: u32,
) -> Result<T, RpcError>
where
    F: FnMut() -> Pin<Box<dyn Future<Output = Result<T, RpcError>>>>,
{
    let mut attempt = 0;
    let mut backoff = Duration::from_millis(100);
    
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && attempt < max_retries => {
                attempt += 1;
                
                if let Some(retry_after) = e.retry_after() {
                    tokio::time::sleep(retry_after).await;
                } else {
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                }
                
                tracing::warn!(
                    "Retrying request (attempt {}/{}) after error: {}",
                    attempt, max_retries, e
                );
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 4. Update RPC Client

```rust
impl StellarRpcClient {
    pub async fn fetch_payments(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Payment>, RpcError> {
        self.circuit_breaker.call(async {
            retry_with_backoff(|| {
                Box::pin(self.fetch_payments_internal(limit, cursor))
            }, 3).await
        }).await
    }
}
```

## ⚙️ Configuration

```toml
# backend/.env
RPC_MAX_RETRIES=3
RPC_INITIAL_BACKOFF_MS=100
RPC_MAX_BACKOFF_MS=5000
RPC_CIRCUIT_BREAKER_FAILURE_THRESHOLD=5
RPC_CIRCUIT_BREAKER_SUCCESS_THRESHOLD=2
RPC_CIRCUIT_BREAKER_TIMEOUT_SECONDS=30
```

## 📊 Metrics to Track

```rust
lazy_static! {
    static ref RPC_ERRORS: IntCounterVec = register_int_counter_vec!(
        "rpc_errors_total",
        "Total RPC errors by type",
        &["error_type", "endpoint"]
    ).unwrap();
    
    static ref CIRCUIT_BREAKER_STATE: IntGaugeVec = register_int_gauge_vec!(
        "circuit_breaker_state",
        "Circuit breaker state (0=closed, 1=open, 2=half-open)",
        &["endpoint"]
    ).unwrap();
}
```

## 🧪 Test Cases

- [ ] Retry on network error
- [ ] No retry on parse error
- [ ] Circuit breaker opens after threshold
- [ ] Circuit breaker closes after successes
- [ ] Rate limit error handling
- [ ] Timeout error handling
- [ ] Structured error logging
- [ ] Error metrics collection

## 🏷️ Priority

**High** - Improves reliability and debuggability

## 🔗 Related Issues

- Improves #1 (Horizon API Parser) error handling
- Works with #9 (Rate Limiting)
