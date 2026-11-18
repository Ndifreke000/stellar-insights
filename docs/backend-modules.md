# Backend Modules Documentation

This document provides comprehensive documentation for key backend modules including configuration options, usage examples, and API references.

## Table of Contents

- [Event Indexer](#event-indexer)
- [Circuit Breaker](#circuit-breaker)
- [RPC Response Size Limits](#rpc-response-size-limits)
- [Vault Module](#vault-module)

---

## Event Indexer

The event indexer service manages contract events with flexible querying and analytics capabilities.

### Overview

The `EventIndexer` provides:
- **Event Storage**: Store and retrieve contract events with metadata
- **Query Interface**: Flexible filtering and sorting options  
- **Verification Tracking**: Monitor event verification status
- **Statistics**: Aggregate event data for analytics

### Key Components

#### EventOrderBy Enum

Controls the sort order of event query results.

```rust
#[derive(Debug, Clone)]
pub enum EventOrderBy {
    /// Sort by insertion time, oldest first
    CreatedAtAsc,
    /// Sort by insertion time, newest first (default)
    CreatedAtDesc,
    /// Sort by ledger sequence number, ascending
    LedgerAsc,
    /// Sort by ledger sequence number, descending
    LedgerDesc,
    /// Sort by epoch number, ascending
    EpochAsc,
    /// Sort by epoch number, descending
    EpochDesc,
}
```

#### EventQuery Struct

Flexible query builder for event filtering:

```rust
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    pub contract_id: Option<String>,
    pub event_type: Option<String>,
    pub epoch: Option<u64>,
    pub hash: Option<String>,
    pub ledger_range: Option<(u64, u64)>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub verification_status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub order_by: Option<EventOrderBy>,
}
```

### Usage Examples

#### Basic Event Query

```rust,no_run
use payraider_backend::services::event_indexer::{EventIndexer, EventQuery, EventOrderBy};

let indexer = EventIndexer::new(db);

// Query recent events for a contract
let query = EventQuery {
    contract_id: Some("contract_123".to_string()),
    limit: Some(100),
    order_by: Some(EventOrderBy::CreatedAtDesc),
    ..Default::default()
};

let events = indexer.query_events(query).await?;
```

#### Advanced Filtering

```rust,no_run
// Query events in specific ledger range with verification
let query = EventQuery {
    ledger_range: Some((100000, 101000)),
    verification_status: Some("verified".to_string()),
    event_type: Some("transfer".to_string()),
    order_by: Some(EventOrderBy::LedgerAsc),
    limit: Some(50),
    offset: Some(0),
};

let events = indexer.query_events(query).await?;
```

#### Event Indexing

```rust,no_run
use payraider_backend::services::event_indexer::IndexedEvent;
use chrono::Utc;

let event = IndexedEvent {
    id: "event_123".to_string(),
    contract_id: "contract_456".to_string(),
    event_type: "transfer".to_string(),
    epoch: Some(12345),
    hash: Some("hash_789".to_string()),
    timestamp: Some(1640995200),
    ledger: 123456,
    transaction_hash: "tx_abc123".to_string(),
    created_at: Utc::now(),
    verification_status: Some("pending".to_string()),
};

indexer.index_event(event).await?;
```

---

## Circuit Breaker

`backend/src/rpc/circuit_breaker.rs` holds the **only** circuit breaker
implementation. Stellar RPC/Horizon, webhook dispatch (`WebhookDispatcher`,
`WebhookChannel`), email (`EmailService`) and the Telegram client all use it.
The old `features/circuit_breaker_pattern.rs` copy and the `failsafe`
dependency are gone. `rpc/rate_limiter.rs` only limits request rate and has no
breaker state.

### API

```rust,no_run
use payraider_backend::rpc::circuit_breaker::{CallError, CircuitBreaker, CircuitBreakerConfig};

let breaker = CircuitBreaker::new("my_integration", CircuitBreakerConfig::default());
// async
let r: Result<T, CallError<E>> = breaker.call(async { do_request().await }).await;
// blocking
let r: Result<T, CallError<E>> = breaker.call_sync(|| do_blocking_request());
match r {
    Ok(v) => {}
    Err(CallError::Rejected) => {}  // circuit open, call not attempted
    Err(CallError::Inner(e)) => {}  // call ran and failed
}
```

`CircuitBreaker::with_clock` takes a `Clock` (`ManualClock` in tests) so
state-machine tests are deterministic.

### Configuration

| Field | Default | Description |
|-------|---------|-------------|
| `failure_threshold` | 5 | Consecutive failures (Closed) before opening |
| `success_threshold` | 2 | Successful half-open probes required to close |
| `timeout_duration` | 30s | Time the circuit stays open before probing |
| `half_open_max_calls` | 1 | Concurrent probes admitted while half-open |

The RPC breaker reads `RPC_CIRCUIT_BREAKER_FAILURE_THRESHOLD`,
`RPC_CIRCUIT_BREAKER_SUCCESS_THRESHOLD` and `RPC_CIRCUIT_BREAKER_TIMEOUT_SECONDS`.

### State-transition table

| From | Event | To |
|------|-------|----|
| Closed | success | Closed (failure count reset) |
| Closed | failure, consecutive < threshold | Closed |
| Closed | failure, consecutive == threshold | Open |
| Open | call before `timeout_duration` | Open (rejected) |
| Open | call after `timeout_duration` | HalfOpen (admitted as probe) |
| HalfOpen | probe slots exhausted | HalfOpen (rejected) |
| HalfOpen | success, successes < `success_threshold` | HalfOpen |
| HalfOpen | success, successes == `success_threshold` | Closed |
| HalfOpen | any failure | Open (timer restarted) |

### Metrics

- `circuit_breaker_transitions_total{breaker,from,to}`: one count per transition
- `circuit_breaker_state{endpoint}`: 0=closed, 1=open, 2=half-open

---

## RPC Response Size Limits

`backend/src/rpc/bounded_body.rs` keeps provider responses from using unbounded
memory. Responses are not read with `.json()` / `.text()` any more:

| Limit | Bytes | Used for |
|-------|-------|----------|
| `BodyLimit::Single` | 1 MiB | single Horizon resources, small JSON-RPC results |
| `BodyLimit::Page` | 8 MiB | Horizon collection pages (`_embedded.records`) |
| `BodyLimit::RpcBatch` | 16 MiB | `getLedgers` and similar large JSON-RPC results |
| `BodyLimit::ErrorBody` | 64 KiB | error bodies (truncated, only logged) |

- If `Content-Length` is over the limit, the response is rejected before the body is read. Otherwise the body is read chunk by chunk and the read stops once the total passes the limit.
- Horizon pages are decoded record by record with a serde visitor (no `Value` tree). A page can hold at most `MAX_RECORDS_PER_PAGE` (1000) records.
- An oversize response returns `RpcError::ResponseTooLarge` (metric label `response_too_large`). It does not count as a circuit-breaker failure.
- `fetch_payments`, `fetch_trades` and `fetch_account_payments` halve the page size and retry when a page is too large. They stop at 10 records.
- Fuzzing: `cd backend && cargo +nightly fuzz run horizon_page`. A proptest in `bounded_body.rs` covers the same decoder in CI.

---

## Vault Module

The vault module provides secure credential management with HashiCorp Vault integration.

### Overview

Features comprehensive secret management:
- **Static Secrets**: KV v2 store for API keys, configuration
- **Dynamic Credentials**: Automatic database credential generation and renewal
- **Lease Management**: Track and renew expiring credentials
- **Audit Logging**: Monitor secret access patterns

### Configuration

#### Environment Variables

Required environment variables:

```bash
export VAULT_ADDR="https://vault.example.com"
export VAULT_TOKEN="your-vault-token"
export DB_ROLE="stellar-app"
export VAULT_NAMESPACE="optional-namespace"  # Optional
```

#### VaultConfig Struct

```rust
#[derive(Clone, Debug)]
pub struct VaultConfig {
    pub vault_addr: String,
    pub vault_token: String,
    pub vault_namespace: Option<String>,
    pub db_role: String,
}
```

### Usage Examples

#### Initialize Vault Client

```rust,no_run
use payraider_backend::vault::{init_vault, VaultClient, VaultConfig};

// From environment (recommended for production)
let vault_client = init_vault().await?;

// Or with explicit config (for testing)
let config = VaultConfig::new(
    "https://vault.example.com".to_string(),
    "your-vault-token".to_string(),
    "stellar-app".to_string(),
);
let client = VaultClient::new(config).await?;
```

#### Read Static Secrets

```rust,no_run
// Read specific field
let api_key = client.read_secret("api/keys/stellar", Some("private_key")).await?;

// Read entire secret
let config = client.read_secret("app/config", None).await?;
```

#### Database Credentials

```rust,no_run
use payraider_backend::vault::DatabaseCredentials;

// Generate temporary database credentials
let creds: DatabaseCredentials = client.get_database_credentials("read-write").await?;

println!("Database Credentials:");
println!("  Username: {}", creds.username);
println!("  Password: [REDACTED]");
println!("  TTL: {} seconds", creds.ttl);

// Credentials are automatically renewed by lease manager
// No manual renewal required
```

#### DatabaseCredentials Structure

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseCredentials {
    /// Database username (typically includes random identifier)
    pub username: String,
    /// Database password (temporary, expires with lease)
    pub password: String,
    /// Time-to-live in seconds before credentials expire
    pub ttl: u64,
}
```

### Error Handling

The vault module provides comprehensive error types:

```rust,no_run
use payraider_backend::vault::VaultError;

match vault_operation {
    Ok(result) => println!("Success: {}", result),
    Err(VaultError::ConfigError(msg)) => println!("Config error: {}", msg),
    Err(VaultError::VaultUnavailable) => println!("Vault unavailable"),
    Err(VaultError::NoDataInSecret) => println!("Secret not found"),
    Err(VaultError::ClientError(msg)) => println!("HTTP error: {}", msg),
}
```

### Best Practices

1. **Use Environment Variables**: Store Vault configuration in environment, not code
2. **Least Privilege**: Grant minimal required permissions to Vault tokens
3. **Token Rotation**: Implement regular token rotation for production
4. **Namespace Usage**: Use namespaces to isolate different applications
5. **Error Handling**: Always handle Vault errors gracefully with retry logic
6. **Lease Monitoring**: Monitor lease expirations to prevent credential issues

### Integration with Circuit Breaker

Combine vault with circuit breaker for resilience:

```rust,no_run
use payraider_backend::rpc::circuit_breaker::rpc_circuit_breaker;
use payraider_backend::vault::init_vault;

let vault_client = init_vault().await?;
let breaker = rpc_circuit_breaker();

// Protected secret access
let secret = breaker.call(|| {
    vault_client.read_secret("api/key", None)
}).await?;
```

---

## Configuration Reference

### Environment Variables

| Variable | Required | Default | Description |
|----------|-----------|---------|-------------|
| `VAULT_ADDR` | Yes | - | Vault server URL |
| `VAULT_TOKEN` | Yes | - | Vault authentication token |
| `VAULT_NAMESPACE` | No | None | Optional namespace path |
| `DB_ROLE` | No | `stellar-app` | Database role for credentials |

### Circuit Breaker Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `failure_threshold` | 5 | Consecutive failures before opening |
| `timeout_duration` | 30s | Time circuit stays open |
| `success_threshold` | 2 | Successful half-open probes required to close |

### Event Query Options

| Field | Type | Description |
|-------|-------|-------------|
| `contract_id` | `Option<String>` | Filter by contract ID |
| `event_type` | `Option<String>` | Filter by event type |
| `epoch` | `Option<u64>` | Filter by epoch number |
| `hash` | `Option<String>` | Filter by transaction hash |
| `ledger_range` | `Option<(u64, u64)>` | Filter by ledger range |
| `time_range` | `Option<(DateTime, DateTime)>` | Filter by time range |
| `verification_status` | `Option<String>` | Filter by verification status |
| `limit` | `Option<i64>` | Maximum results to return |
| `offset` | `Option<i64>` | Number of results to skip |
| `order_by` | `Option<EventOrderBy>` | Sort order of results |

---

## Testing

All modules include comprehensive test coverage. Run tests with:

```bash
# Event indexer tests
cargo test services::event_indexer

# Circuit breaker tests  
cargo test rpc::circuit_breaker

# Vault module tests
cargo test vault

# All backend tests
cargo test --workspace
```

## Troubleshooting

### Common Issues

1. **Vault Connection Errors**
   - Verify `VAULT_ADDR` is accessible
   - Check token validity and permissions
   - Ensure network connectivity

2. **Circuit Breaker Always Open**
   - Check failure threshold configuration
   - Verify underlying service health
   - Monitor error logs for patterns

3. **Event Query Performance**
   - Add appropriate database indexes
   - Use pagination for large result sets
   - Consider time-based filtering for recent data

### Monitoring

Key metrics to monitor:
- Circuit breaker state changes
- Vault API response times
- Event query performance
- Database connection pool usage
- Lease renewal success rates
