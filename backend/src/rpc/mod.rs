//! # RPC module – error handling strategy
//!
//! - **Error categorization**: Errors are classified in `RpcError` (network, rate limit,
//!   server 5xx, parse, timeout, circuit breaker open). Use `is_retryable()` and
//!   `retry_after()` for retry logic.
//! - **Retry**: Only retryable errors are retried with exponential backoff (see `retry` module).
//!   Rate limit errors can use `Retry-After` when present.
//! - **Circuit breaker**: After a configurable failure threshold, the circuit opens and calls
//!   fail fast with `RpcError::CircuitBreakerOpen`. After a timeout, the circuit goes
//!   half-open and allows probe requests.
//! - **Metrics**: `rpc_errors_total` (by error_type and endpoint) and `circuit_breaker_state`
//!   (0=closed, 1=open, 2=half-open) are recorded for alerting and dashboards.
//! - **Frontend**: API handlers convert `RpcError` to `ApiError` with user-facing messages
//!   so the frontend can show a clear error instead of empty data.

pub mod circuit_breaker;
pub mod error;
pub mod metrics;
pub mod retry;
pub mod stellar;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
pub use error::RpcError;
pub use retry::retry_with_backoff;
pub use stellar::{
    Asset, FeeBumpTransactionInfo, GetLedgersResult, HealthResponse, HorizonTransaction, InnerTransaction,
    LedgerInfo, OrderBook, OrderBookEntry, Payment, Price, RpcLedger, StellarRpcClient, Trade,
};
