# PR: Distributed Tracing Implementation

## Overview
This PR implements distributed tracing across the Stellar Insights backend services using OpenTelemetry and Jaeger. It addresses the lack of visibility into request lifecycles and performance bottlenecks.

## Changes
- **Tracing Configuration**: Refactored [tracing.rs](file:///c:/Users/Dell/Documents/stellar-insights/backend/src/observability/tracing.rs) to include the `OpenTelemetryLayer` and an OTLP exporter for Jaeger.
- **Database Instrumentation**: Added `#[tracing::instrument]` to all critical [Database](file:///c:/Users/Dell/Documents/stellar-insights/backend/src/database.rs) methods.
- **API Instrumentation**: Instrumented [Anchors](file:///c:/Users/Dell/Documents/stellar-insights/backend/src/api/anchors.rs) and [Corridors](file:///c:/Users/Dell/Documents/stellar-insights/backend/src/api/corridors.rs) handlers to ensure end-to-end request tracking.
- **Local Testing Setup**: Added [docker-compose.jaeger.yml](file:///c:/Users/Dell/Documents/stellar-insights/docker-compose.jaeger.yml) for local trace collection and visualization.

## Verification
1.  Run Jaeger: `docker-compose -f docker-compose.jaeger.yml up -d`
2.  Start Backend: `OTEL_ENABLED=true cargo run`
3.  Access Jaeger UI: `http://localhost:16686`

## Dependencies
- `opentelemetry`
- `opentelemetry-otlp`
- `tracing-opentelemetry`

#728
