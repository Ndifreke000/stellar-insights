---
title: "[ENHANCEMENT] Missing Observability - Metrics, Tracing, and Structured Logging"
labels: enhancement, high, observability, monitoring, backend
assignees: ''
---

## 💡 Description

The backend lacks comprehensive observability infrastructure. There's basic logging but no metrics collection, distributed tracing, or structured logging for production debugging.

## 📊 Current State

- Basic `tracing` crate usage
- No metrics (Prometheus, StatsD)
- No distributed tracing (Jaeger, Zipkin)
- Inconsistent log formatting
- No request ID tracking
- No performance monitoring
- Difficult to debug production issues

## ✅ Expected Behavior

- Prometheus metrics endpoint at `/metrics`
- Distributed tracing with OpenTelemetry
- Structured JSON logging
- Request ID propagation
- Performance metrics
- Business metrics (corridors tracked, RPC calls, etc.)
- Grafana-ready dashboards

## 📁 Affected Files

- `backend/src/main.rs` (add metrics middleware)
- New file: `backend/src/observability/metrics.rs`
- New file: `backend/src/observability/tracing.rs`
- New file: `backend/src/observability/logging.rs`
- All API handlers (add instrumentation)

## 📊 Metrics to Track

### HTTP Metrics
- Request duration by endpoint
- Request count by endpoint and status
- Active connections
- Request size / response size

### RPC Metrics
- RPC call duration by method
- RPC call count by method and status
- RPC error rate by type
- Circuit breaker state

### Cache Metrics
- Cache hit/miss rates
- Cache size
- Cache eviction count
- Cache operation duration

### Business Metrics
- Active corridors count
- Total anchors tracked
- Payments processed
- Data freshness (time since last update)

## 📋 Acceptance Criteria

- [ ] Add Prometheus metrics endpoint at `/metrics`
- [ ] Implement OpenTelemetry tracing
- [ ] Add structured JSON logging
- [ ] Create request ID middleware
- [ ] Add metrics for all key operations
- [ ] Add tracing spans to all handlers
- [ ] Create example Grafana dashboards
- [ ] Document observability setup
- [ ] Add health metrics
- [ ] Test metrics collection

## 🚀 Implementation Steps

### 1. Add Dependencies

```toml
[dependencies]
prometheus = "0.13"
opentelemetry = "0.20"
opentelemetry-prometheus = "0.13"
tracing-opentelemetry = "0.21"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
```

### 2. Setup Metrics

```rust
// backend/src/observability/metrics.rs
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec,
    HistogramVec, IntCounterVec, IntGaugeVec,
};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Total HTTP requests",
        &["method", "endpoint", "status"]
    ).unwrap();
    
    pub static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration",
        &["method", "endpoint"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
    ).unwrap();
    
    pub static ref RPC_CALLS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "rpc_calls_total",
        "Total RPC calls",
        &["method", "status"]
    ).unwrap();
    
    pub static ref RPC_CALL_DURATION: HistogramVec = register_histogram_vec!(
        "rpc_call_duration_seconds",
        "RPC call duration",
        &["method"],
        vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0]
    ).unwrap();
    
    pub static ref CACHE_OPERATIONS: IntCounterVec = register_int_counter_vec!(
        "cache_operations_total",
        "Cache operations",
        &["operation", "result"]
    ).unwrap();
    
    pub static ref ACTIVE_CORRIDORS: IntGaugeVec = register_int_gauge_vec!(
        "active_corridors",
        "Number of active corridors",
        &[]
    ).unwrap();
}
```

### 3. Add Metrics Middleware

```rust
use axum::{
    middleware::{self, Next},
    response::Response,
    extract::Request,
};
use std::time::Instant;

pub async fn metrics_middleware(
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    
    let response = next.run(req).await;
    
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();
    
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[&method, &path, &status])
        .inc();
    
    HTTP_REQUEST_DURATION
        .with_label_values(&[&method, &path])
        .observe(duration);
    
    response
}
```

### 4. Setup Structured Logging

```rust
// backend/src/observability/logging.rs
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true)
        )
        .init();
}
```

### 5. Add Tracing Spans

```rust
use tracing::{instrument, info, warn};

#[instrument(skip(db, cache, rpc_client))]
pub async fn list_corridors(
    State((db, cache, rpc_client)): State<(Arc<Database>, Arc<CacheManager>, Arc<StellarRpcClient>)>,
    Query(params): Query<ListCorridorsQuery>,
) -> ApiResult<Json<Vec<CorridorResponse>>> {
    info!("Fetching corridors with params: {:?}", params);
    
    // ... implementation
    
    info!("Returning {} corridors", corridors.len());
    Ok(Json(corridors))
}
```

### 6. Add Metrics Endpoint

```rust
use axum::routing::get;
use prometheus::{Encoder, TextEncoder};

async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

// In main.rs
let app = Router::new()
    .route("/metrics", get(metrics_handler))
    .route("/api/corridors", get(list_corridors))
    .layer(middleware::from_fn(metrics_middleware));
```

## 📊 Example Grafana Dashboard

```json
{
  "dashboard": {
    "title": "Stellar Insights Backend",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [{
          "expr": "rate(http_requests_total[5m])"
        }]
      },
      {
        "title": "Request Duration (p95)",
        "targets": [{
          "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))"
        }]
      },
      {
        "title": "RPC Call Success Rate",
        "targets": [{
          "expr": "rate(rpc_calls_total{status=\"success\"}[5m]) / rate(rpc_calls_total[5m])"
        }]
      },
      {
        "title": "Cache Hit Rate",
        "targets": [{
          "expr": "rate(cache_operations_total{result=\"hit\"}[5m]) / rate(cache_operations_total[5m])"
        }]
      }
    ]
  }
}
```

## ⚙️ Configuration

```bash
# backend/.env
RUST_LOG=info,stellar_insights_backend=debug
LOG_FORMAT=json  # or "pretty" for development
METRICS_ENABLED=true
TRACING_ENABLED=true
JAEGER_ENDPOINT=http://localhost:14268/api/traces
```

## 🧪 Test Cases

- [ ] Metrics endpoint returns valid Prometheus format
- [ ] HTTP metrics are recorded correctly
- [ ] RPC metrics are recorded correctly
- [ ] Cache metrics are recorded correctly
- [ ] Tracing spans are created
- [ ] Structured logs are formatted correctly
- [ ] Request IDs are propagated

## 📚 Documentation Needed

- Setup guide for Prometheus
- Setup guide for Grafana
- Example dashboard JSON
- Metrics reference
- Logging best practices

## 🏷️ Priority

**High** - Critical for production operations and debugging

## 🔗 Related Issues

- Helps debug #1 (Horizon API Parser)
- Monitors #7 (Error Handling)
- Tracks #9 (Rate Limiting)
