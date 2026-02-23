# APM Integration Architecture - Stellar Insights

## Overview

This document outlines the comprehensive Application Performance Monitoring (APM) integration strategy for Stellar Insights, supporting both New Relic and Datadog platforms.

## Current State Analysis

### Existing Monitoring
- **Logging**: Tracing subscriber with structured logging
- **Basic Metrics**: Limited custom metrics
- **Error Tracking**: Basic error logging
- **Performance**: No dedicated APM solution

### Gaps Identified
- No distributed tracing
- Limited performance insights
- No real-time alerting
- Missing application metrics
- No database performance monitoring
- No frontend performance tracking

## Architecture Design

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Frontend      │    │   Backend        │    │   Database      │
│   (Next.js)     │───▶│   (Rust/Axum)    │───▶│   (PostgreSQL)  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Browser APM   │    │   Server APM     │    │   DB APM        │
│   (RUM)         │    │   (APM Agent)    │    │   (Query Stats) │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 ▼
                    ┌──────────────────┐
                    │   APM Platform   │
                    │ (New Relic/      │
                    │  Datadog)        │
                    └──────────────────┘
                                 │
                                 ▼
                    ┌──────────────────┐
                    │   Dashboards &   │
                    │   Alerts         │
                    └──────────────────┘
```

## Implementation Strategy

### Phase 1: Backend APM Integration
1. **OpenTelemetry Setup**
   - Distributed tracing
   - Custom metrics
   - Error tracking
   - Performance monitoring

2. **Database Monitoring**
   - Query performance
   - Connection pooling
   - Transaction tracking

3. **API Monitoring**
   - Request/response metrics
   - Error rates
   - Latency tracking

### Phase 2: Frontend APM Integration
1. **Real User Monitoring (RUM)**
   - Page load performance
   - User interaction tracking
   - JavaScript errors

2. **API Client Monitoring**
   - Request timing
   - Error handling
   - Network performance

### Phase 3: Infrastructure Integration
1. **Container Monitoring**
   - Resource usage
   - Health checks
   - Auto-scaling metrics

2. **CI/CD Integration**
   - Deployment tracking
   - Performance tests
   - Rollback monitoring

## Technology Stack

### Backend (Rust)
- **OpenTelemetry SDK**: Core observability
- **Tracing**: Distributed tracing
- **Metrics**: Custom application metrics
- **Logs**: Structured logging with correlation

### Frontend (Next.js)
- **@opentelemetry/api**: OpenTelemetry API
- **@opentelemetry/sdk-web**: Web SDK
- **@opentelemetry/instrumentation**: Auto-instrumentation

### APM Platforms
- **New Relic**: Full-stack observability
- **Datadog**: Infrastructure + APM

## Key Metrics to Track

### Application Metrics
- Request rate and latency
- Error rate and types
- Database query performance
- Cache hit rates
- WebSocket connection metrics
- Background job performance

### Business Metrics
- Active users
- API usage patterns
- Stellar network requests
- Data ingestion rates
- Alert generation rates

### Infrastructure Metrics
- CPU and memory usage
- Disk I/O and network
- Container health
- Database connections

## Instrumentation Points

### API Endpoints
```rust
#[instrument(skip_all)]
async fn get_corridors(
    State(app_state): State<AppState>,
    Query(params): Query<CorridorParams>,
) -> Result<Json<CorridorResponse>, AppError> {
    let span = tracing::Span::current();
    span.record("corridor_count", &params.limit);
    span.record("filter_type", &params.filter_type.as_deref().unwrap_or("none"));
    
    // Implementation...
}
```

### Database Operations
```rust
#[instrument(skip(db))]
async fn get_corridor_data(
    db: &PgPool,
    corridor_id: &str,
) -> Result<Option<Corridor>, DbError> {
    let span = tracing::Span::current();
    span.record("corridor_id", corridor_id);
    
    // Query with timing...
}
```

### Background Jobs
```rust
#[instrument(skip_all)]
async fn process_stellar_data(
    stellar_client: &StellarRpcClient,
    db: &PgPool,
) -> Result<(), ProcessingError> {
    let span = tracing::Span::current();
    let start = Instant::now();
    
    // Processing logic...
    
    let duration = start.elapsed();
    span.record("processing_duration_ms", duration.as_millis());
}
```

## Configuration Management

### Environment Variables
```bash
# APM Configuration
OTEL_SERVICE_NAME=stellar-insights
OTEL_SERVICE_VERSION=1.0.0
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_RESOURCE_ATTRIBUTES=service.name=stellar-insights,service.version=1.0.0

# New Relic
NEW_RELIC_LICENSE_KEY=your_license_key
NEW_RELIC_APP_NAME=stellar-insights

# Datadog
DD_API_KEY=your_api_key
DD_ENV=production
DD_SERVICE=stellar-insights
DD_VERSION=1.0.0
```

### Feature Flags
```rust
pub struct ApmConfig {
    pub enabled: bool,
    pub platform: ApmPlatform,
    pub sample_rate: f64,
    pub trace_level: TraceLevel,
}

pub enum ApmPlatform {
    NewRelic,
    Datadog,
    OpenTelemetry,
}
```

## Alerting Strategy

### Critical Alerts
- Error rate > 5%
- Response time > 2s
- Database connection failures
- Memory usage > 80%
- Disk space < 10%

### Warning Alerts
- Response time > 1s
- Error rate > 2%
- CPU usage > 70%
- Cache miss rate > 20%

### Info Alerts
- New deployments
- Configuration changes
- Performance improvements

## Dashboard Templates

### Application Overview
- Request rate and latency
- Error rate breakdown
- Active user count
- Database performance

### Infrastructure Health
- Resource utilization
- Container status
- Network latency
- Storage metrics

### Business Metrics
- API usage patterns
- Stellar network activity
- Data ingestion rates
- Alert trends

## Implementation Timeline

### Week 1: Foundation
- OpenTelemetry setup
- Basic instrumentation
- Environment configuration

### Week 2: Backend Integration
- API endpoint tracing
- Database monitoring
- Custom metrics

### Week 3: Frontend Integration
- RUM implementation
- Client-side metrics
- Error tracking

### Week 4: Platform Integration
- New Relic setup
- Datadog configuration
- Dashboard creation

### Week 5: Optimization
- Alert configuration
- Performance tuning
- Documentation

## Security Considerations

### Data Protection
- PII filtering
- Sensitive data masking
- GDPR compliance

### Access Control
- API key management
- Role-based access
- Audit logging

### Network Security
- TLS encryption
- Firewall rules
- VPC isolation

## Performance Impact

### Overhead Analysis
- OpenTelemetry: <5% CPU overhead
- Metrics collection: <2% memory overhead
- Network bandwidth: ~1MB/hour

### Optimization Strategies
- Sampling strategies
- Batch processing
- Async reporting
- Local buffering

## Migration Strategy

### Phase 1: Parallel Operation
- Run APM alongside existing logging
- Validate data accuracy
- Performance impact assessment

### Phase 2: Gradual Migration
- Route specific endpoints to APM
- Monitor data consistency
- Adjust configuration

### Phase 3: Full Integration
- Complete migration
- Retire legacy monitoring
- Optimize performance

## Success Metrics

### Technical KPIs
- <100ms APM overhead
- 99.9% data accuracy
- <5min alert latency
- 99% uptime

### Business KPIs
- Faster issue detection
- Improved user experience
- Reduced MTTR (Mean Time To Resolution)
- Better resource utilization

---

**Version**: 1.0  
**Last Updated**: February 2026  
**Next Review**: March 2026
