# Stellar Insights Testnet SLOs and Alerting Thresholds

This document defines Service Level Objectives (SLOs) for the Stellar Insights testnet environment and the alert thresholds that trigger operational notifications. These thresholds were chosen to balance responsiveness (catching real issues quickly) with noise reduction (avoiding alert fatigue).

## Overview

**Monitoring Stack:**
- Metrics collection: Prometheus (`/metrics` endpoint)
- Dashboarding: Grafana
- Distributed tracing: Jaeger
- Centralized logging: ELK (Elasticsearch, Logstash, Kibana)
- Alerting: Prometheus AlertManager (can be routed to Slack, PagerDuty, email, or webhooks)

**Alert notification channels** should be configured in your Prometheus AlertManager config (`alertmanager.yml`):
- Slack: For dev/debug alerts (non-critical)
- PagerDuty: For SEV-1/SEV-2 production incidents
- Email: For audit logging and acknowledgment

---

## 1. SLOs by Service Tier

### Tier 1: Critical Infrastructure (Health, Metrics, Status endpoints)

**Endpoint:** `/health`  
**SLO Target:** 99.9% availability (4.38 hours downtime per month)  
**SLO Target:** p99 latency ≤ 500ms

**Rationale:**  
The health check is the first signal for monitoring and orchestration systems (Kubernetes, monitoring dashboards, alerting systems). It must be extremely reliable. Testnet environments can tolerate slightly lower SLOs than production, but this remains foundational.

**Alert Thresholds:**
- **Error rate > 1%** (severity: warning, 5-min evaluation)
  - Indicates health endpoint is flaky; may reflect underlying issue
  - Action: Check if database, cache, or RPC is degraded
  - Recovery: Usually resolves when underlying service recovers
  
- **Error rate > 5%** (severity: critical, 2-min evaluation)
  - Health endpoint is consistently failing
  - Action: Page on-call; investigate database/cache connectivity immediately
  - Recovery: Restart backend service or failover cache/database

- **p99 latency > 1000ms** (severity: warning)
  - Health checks are slow; indicates system under stress
  - Action: Check database pool utilization, cache hit rate, RPC call backlog
  - Recovery: Scale horizontally or identify slow queries

---

### Tier 2: User-Facing API Endpoints (Anchors, Corridors)

**Endpoints:** `/anchors`, `/corridors`, `/corridors/{id}`  
**SLO Target:** 99.5% availability (3.6 hours downtime per month)  
**SLO Target:** p95 latency ≤ 2s, p99 latency ≤ 5s

**Rationale:**  
These are the primary data endpoints; downtime directly impacts users. These endpoints are cached, so they should be fast under normal load. Testnet can tolerate brief degradation for deployments/maintenance.

**Alert Thresholds:**
- **Error rate > 0.5%** (severity: warning, 5-min evaluation)
  - Indicates persistent backend issue (database, cache, RPC calls)
  - Action: Check error logs; identify which subsystem is failing
  - Recovery: May require database schema verification, cache restart, or RPC failover

- **Error rate > 2%** (severity: critical, 2-min evaluation)
  - Significant user-facing impact
  - Action: Page on-call; begin incident response (see runbooks)
  - Recovery: Rollback recent deploy or restart affected service

- **p95 latency > 5s** (severity: warning, 10-min evaluation)
  - Endpoint under sustained load or slow queries
  - Action: Check database slow-query log; look for N+1 queries or missing indexes
  - Recovery: Query optimization or cache invalidation

- **p99 latency > 10s** (severity: critical, 5-min evaluation)
  - Users see 10+ second waits; quality-of-life degradation
  - Action: Page on-call; check for runaway queries or RPC timeouts
  - Recovery: Restart backend or identify bottleneck service

- **Cache hit rate < 50%** (severity: warning, 15-min evaluation)
  - Indicates cache is ineffective; endpoints will be slow and expensive
  - Action: Check if cache was restarted; verify cache key generation
  - Recovery: Trigger cache rebuild if needed; investigate invalidation rules

---

### Tier 3: Integration Endpoints (SEP24 proxy, RPC proxy)

**Endpoints:** `/sep24*`, `/rpc/*`  
**SLO Target:** 99.0% availability (7.2 hours downtime per month)  
**SLO Target:** p95 latency ≤ 5s, p99 latency ≤ 15s (RPC call latency varies)

**Rationale:**  
These endpoints depend on external services (Stellar RPC, Horizon) which may be unavailable. Testnet is lower priority for Stellar Foundation support. Failures here do not block critical flows.

**Alert Thresholds:**
- **RPC error rate > 1%** (severity: warning, 5-min evaluation)
  - RPC calls to Stellar network are failing; may indicate network issue or endpoint rate-limiting
  - Action: Check RPC endpoint connectivity; switch to backup endpoint if available
  - Recovery: Switch endpoints or wait for Stellar Foundation to restore service

- **RPC error rate > 5%** (severity: critical, 2-min evaluation)
  - Severe RPC degradation affecting SEP flows
  - Action: Page on-call; attempt endpoint failover
  - Recovery: Failover to secondary RPC endpoint

- **Circuit breaker trip rate > 0.1/s** (severity: warning, 3-min evaluation)
  - Circuit breaker is actively protecting backend from cascading failures
  - Action: Identify which subsystem has the broken dependency
  - Recovery: Fix dependency issue or reduce load

---

### Tier 4: Background Jobs and Infrastructure

**Metrics:** Database pool utilization, cache operations, background job throughput  
**SLO Target:** 99.0% for job completion (jobs should not be silently dropped)  

**Alert Thresholds:**

#### Database Pool Utilization
- **Active connections > 90% of pool size** (severity: warning, 2-min evaluation)
  - Pool is nearly exhausted; next spike will cause request queuing or failures
  - Action: Investigate which queries are holding connections
  - Recovery: Optimize slow queries or increase pool size

- **Active connections > 100% of pool size** (severity: critical, 1-min evaluation)
  - Connection pool is oversubscribed; requests will fail immediately
  - Action: Page on-call; emergency measures may be needed
  - Recovery: Kill long-running queries or temporarily reject new requests

#### Cache Operations
- **Cache miss rate > 60%** (severity: warning, 15-min evaluation)
  - Cache is not effective; indicates high churn or misconfiguration
  - Action: Check cache invalidation strategy; verify memory pressure isn't causing eviction
  - Recovery: Adjust cache TTLs or investigate invalidation triggers

#### Background Jobs
- **Background job failure rate > 5%** (severity: warning, 10-min evaluation)
  - Some background tasks are silently failing; may cause data staleness or missed alerts
  - Action: Check job logs; investigate which jobs are failing and why
  - Recovery: Fix underlying cause (e.g., RPC rate limit, database quota)

- **Job processing lag > 1 hour** (severity: warning, 5-min evaluation)
  - Jobs are backed up; system is falling behind real-time ingestion
  - Action: Check if job processor is scaled appropriately; identify slow jobs
  - Recovery: Scale job processor or optimize job logic

---

## 2. Prometheus Alert Rules

Add the following alert rules to your Prometheus configuration (`prometheus.yml` or a separate `alerts.yml`):

```yaml
groups:
  - name: stellar_insights_testnet
    interval: 30s
    rules:
      # Tier 1: Critical Infrastructure
      - alert: HealthCheckEndpointDown
        expr: rate(http_errors_total{path="/health"}[5m]) > 0.01
        for: 2m
        labels:
          severity: critical
          slo: tier1
        annotations:
          summary: "Health check endpoint error rate > 1%"
          description: "Health endpoint at {{ $labels.instance }} has error rate {{ $value | humanizePercentage }} over the last 5m."
          runbook_url: "https://docs.example.com/runbooks/health-endpoint-down"

      - alert: HealthCheckLatencySpiked
        expr: histogram_quantile(0.99, http_request_duration_seconds{path="/health"}) > 1.0
        for: 5m
        labels:
          severity: warning
          slo: tier1
        annotations:
          summary: "Health check p99 latency > 1s"
          description: "Health endpoint p99 latency is {{ $value | humanizeDuration }} (target: <500ms)"

      # Tier 2: User-Facing API
      - alert: AnchorsEndpointErrors
        expr: rate(http_errors_total{path="/anchors"}[5m]) > 0.005
        for: 5m
        labels:
          severity: warning
          slo: tier2
        annotations:
          summary: "Anchors endpoint error rate > 0.5%"
          description: "{{ $labels.path }} has error rate {{ $value | humanizePercentage }}"
          runbook_url: "https://docs.example.com/runbooks/anchor-errors"

      - alert: AnchorsEndpointCriticalErrors
        expr: rate(http_errors_total{path="/anchors"}[2m]) > 0.02
        for: 2m
        labels:
          severity: critical
          slo: tier2
        annotations:
          summary: "Anchors endpoint error rate > 2% (CRITICAL)"
          description: "Immediate action required: error rate is {{ $value | humanizePercentage }}"

      - alert: CorridorsLatencyHighP95
        expr: histogram_quantile(0.95, http_request_duration_seconds{path="/corridors"}) > 5
        for: 10m
        labels:
          severity: warning
          slo: tier2
        annotations:
          summary: "Corridors endpoint p95 latency > 5s"
          description: "Latency {{ $value | humanizeDuration }} indicates possible database or cache issue"

      - alert: CorridorsLatencyHighP99
        expr: histogram_quantile(0.99, http_request_duration_seconds{path="/corridors"}) > 10
        for: 5m
        labels:
          severity: critical
          slo: tier2
        annotations:
          summary: "Corridors endpoint p99 latency > 10s (CRITICAL)"
          description: "Users are experiencing {{ $value | humanizeDuration }} waits"

      - alert: CacheHitRateLow
        expr: rate(cache_hits_total[5m]) / (rate(cache_hits_total[5m]) + rate(cache_misses_total[5m])) < 0.5
        for: 15m
        labels:
          severity: warning
          slo: tier2
        annotations:
          summary: "Cache hit rate < 50%"
          description: "Cache is ineffective (hit rate: {{ $value | humanizePercentage }}); cached endpoints will be slow"

      # Tier 3: Integration Endpoints
      - alert: RPCErrorRateHigh
        expr: rate(rpc_errors_total[5m]) / rate(rpc_calls_total[5m]) > 0.01
        for: 5m
        labels:
          severity: warning
          slo: tier3
        annotations:
          summary: "RPC error rate > 1%"
          description: "RPC calls to {{ $labels.method }} are failing (error rate: {{ $value | humanizePercentage }})"

      - alert: RPCErrorRateCritical
        expr: rate(rpc_errors_total[2m]) / rate(rpc_calls_total[2m]) > 0.05
        for: 2m
        labels:
          severity: critical
          slo: tier3
        annotations:
          summary: "RPC error rate > 5% (CRITICAL)"
          description: "SEP flows may be unable to complete; error rate {{ $value | humanizePercentage }}"

      - alert: CircuitBreakerTripping
        expr: rate(circuit_breaker_trips_total[1m]) > 0.001
        for: 3m
        labels:
          severity: warning
          slo: tier3
        annotations:
          summary: "Circuit breaker tripping (> 0.1/s)"
          description: "Circuit breaker is actively protecting from cascade failures (trip rate: {{ $value | humanize }}/s)"

      # Tier 4: Infrastructure
      - alert: DatabasePoolExhausted
        expr: db_pool_active / db_pool_size > 0.9
        for: 2m
        labels:
          severity: warning
          slo: tier4
        annotations:
          summary: "Database pool utilization > 90%"
          description: "{{ $value | humanizePercentage }} of connections are active; pool may exhaust under spike"

      - alert: DatabasePoolOversubscribed
        expr: db_pool_active > db_pool_size
        for: 1m
        labels:
          severity: critical
          slo: tier4
        annotations:
          summary: "Database pool oversubscribed (CRITICAL)"
          description: "{{ $value | humanize }} requests queued waiting for connections; immediate action required"

      - alert: DatabaseQueryLatencyHigh
        expr: histogram_quantile(0.95, db_query_duration_seconds) > 1
        for: 10m
        labels:
          severity: warning
          slo: tier4
        annotations:
          summary: "Database queries slow (p95 > 1s)"
          description: "Slow queries detected; check slow-query log for missing indexes or N+1 problems"

      - alert: CacheMissRateHigh
        expr: rate(cache_misses_total[5m]) / (rate(cache_hits_total[5m]) + rate(cache_misses_total[5m])) > 0.6
        for: 15m
        labels:
          severity: warning
          slo: tier4
        annotations:
          summary: "Cache miss rate > 60%"
          description: "Cache is ineffective; backend load will be high (miss rate: {{ $value | humanizePercentage }})"

      - alert: BackgroundJobFailureRate
        expr: rate(background_jobs_failed_total[5m]) / rate(background_jobs_total[5m]) > 0.05
        for: 10m
        labels:
          severity: warning
          slo: tier4
        annotations:
          summary: "Background jobs failing > 5%"
          description: "Jobs are failing silently; data staleness may result (failure rate: {{ $value | humanizePercentage }})"

      - alert: BackgroundJobLag
        expr: background_job_processing_lag_seconds > 3600
        for: 5m
        labels:
          severity: warning
          slo: tier4
        annotations:
          summary: "Background job processing lag > 1 hour"
          description: "Job queue is backed up; real-time ingestion is falling behind (lag: {{ $value | humanizeDuration }})"

      # Global: Very High Error Rate (catches unexpected failures)
      - alert: HighGlobalErrorRate
        expr: rate(http_errors_total[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
          slo: global
        annotations:
          summary: "Global error rate > 5% (CRITICAL INCIDENT)"
          description: "Multiple endpoints experiencing errors; possible cascading failure (error rate: {{ $value | humanizePercentage }})"
          runbook_url: "https://docs.example.com/runbooks/cascading-failure"
```

---

## 3. Alert Routing and Notification Channels

### AlertManager Configuration (`alertmanager.yml`)

```yaml
global:
  resolve_timeout: 5m
  slack_api_url: "https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK"

route:
  receiver: default
  group_by: ['alertname', 'cluster']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 4h

  # Critical incidents → PagerDuty immediately
  routes:
    - match:
        severity: critical
      receiver: pagerduty
      group_wait: 0s
      group_interval: 1m
      repeat_interval: 15m

    # Warnings → Slack dev channel
    - match:
        severity: warning
      receiver: slack_dev
      group_wait: 30s
      group_interval: 5m
      repeat_interval: 8h

receivers:
  # Default: dev channel
  - name: default
    slack_configs:
      - channel: '#stellar-insights-alerts'
        title: '{{ .GroupLabels.alertname }}'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'

  # Critical: PagerDuty (requires PagerDuty integration key)
  - name: pagerduty
    pagerduty_configs:
      - service_key: "YOUR_PAGERDUTY_SERVICE_KEY"
        description: '{{ .GroupLabels.alertname }}: {{ range .Alerts }}{{ .Annotations.summary }}{{ end }}'

  # Warnings: Slack dev channel
  - name: slack_dev
    slack_configs:
      - channel: '#stellar-insights-dev'
        title: '[WARNING] {{ .GroupLabels.alertname }}'
        text: '{{ range .Alerts }}{{ .Annotations.summary }}{{ end }}'
```

### Slack Channel Setup

**#stellar-insights-alerts** (critical incidents only)
- Subscribers: On-call engineer, engineering manager, platform team lead
- Notification: @here for SEV-1 alerts
- SLA for response: 5 minutes

**#stellar-insights-dev** (warnings and informational)
- Subscribers: Dev team
- Notification: No escalation; developers check channel during work hours
- SLA for response: Best effort (not a paging alert)

**#stellar-insights-monitoring** (all metrics and observability)
- Subscribers: SRE/devops team, optional for interested developers
- Purpose: Audit log of all metric changes; not actionable
- Retention: 90 days

---

## 4. Grafana Dashboard Setup

To monitor these SLOs in real time, create a Grafana dashboard with the following panels:

### Dashboard: Stellar Insights Testnet SLOs

**Row 1: Tier 1 - Critical Infrastructure**
- Panel: Health endpoint availability (% of successful requests, target: 99.9%)
- Panel: Health endpoint latency (p50, p95, p99; target: <500ms for p99)
- Panel: Database/cache/RPC health sub-checks

**Row 2: Tier 2 - User-Facing API**
- Panel: Anchors endpoint error rate (%, target: <0.5%)
- Panel: Corridors endpoint error rate (%, target: <0.5%)
- Panel: Anchors latency distribution (p50, p95, p99)
- Panel: Corridors latency distribution (p50, p95, p99)
- Panel: Cache hit rate (%, target: >80%)

**Row 3: Tier 3 - Integration Endpoints**
- Panel: SEP24 proxy error rate (%)
- Panel: RPC proxy error rate (%, target: <1%)
- Panel: RPC latency distribution by method
- Panel: Circuit breaker trip rate (count/s)

**Row 4: Tier 4 - Infrastructure**
- Panel: Database pool utilization (%, target: <70%)
- Panel: Database query latency (p50, p95, p99)
- Panel: Cache operations and hit/miss breakdown
- Panel: Background job failure rate (%)
- Panel: Background job queue depth (jobs waiting)

**Row 5: Overall Health**
- Panel: Global error rate (%, target: <0.1%)
- Panel: Active connections (websockets, database)
- Panel: RPC call rate (calls/s)
- Panel: Request rate by endpoint (top 5)

### Dashboard Query Examples

```promql
# Health endpoint SLO
(rate(http_requests_total{path="/health",status=~"2.."}[5m]) / rate(http_requests_total{path="/health"}[5m])) * 100

# Anchors endpoint availability (SLO target: >99.5%)
(rate(http_requests_total{path="/anchors",status=~"2.."}[5m]) / rate(http_requests_total{path="/anchors"}[5m])) * 100

# Corridors latency p95
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket{path="/corridors"}[5m]))

# Cache hit rate
rate(cache_hits_total[5m]) / (rate(cache_hits_total[5m]) + rate(cache_misses_total[5m]))

# Database pool utilization
(db_pool_active / db_pool_size) * 100

# RPC error rate
rate(rpc_errors_total[5m]) / rate(rpc_calls_total[5m])
```

---

## 5. Incident Response Integration

### Escalation Path

**SEV-1 (Critical, P1):** Page on-call immediately
- Examples: Health endpoint down, >2% error rate on primary endpoints, database pool exhausted
- SLA: 5-minute response time
- Action: Engage primary on-call engineer; escalate to manager if unresolved in 15m

**SEV-2 (High, P2):** Page on-call within 15 minutes
- Examples: >0.5% error rate, p99 latency >10s, cache hit rate <50%
- SLA: 30-minute response time
- Action: Engage on-call engineer; may involve secondary responder

**SEV-3 (Medium, P3):** Slack notification, no page
- Examples: Warnings, minor latency increases, resource utilization trending high
- SLA: Best effort during business hours
- Action: Developers investigate; no escalation required

### On-Call Runbook Template

For each alert, create a runbook in `docs/runbooks/`:

```markdown
# [Alert Name]

## Detection
- **Alert:** [Alert name from Prometheus]
- **Metric:** [e.g., `http_errors_total{path="/anchors"}`]
- **Threshold:** [e.g., error rate >0.5%]

## Impact
- [What users/systems are affected]
- [How to verify the impact]

## Diagnosis
1. Check Grafana dashboard: [link]
2. Look for correlations: [common causes]
3. Check error logs: [e.g., ELK query]
4. Validate RPC/Horizon connectivity: [commands]

## Remediation
- **Quick fix (< 5 min):** [e.g., restart cache]
- **Medium fix (5-30 min):** [e.g., optimize query]
- **Long-term fix:** [e.g., add index, scale horizontally]

## Escalation
- If unresolved after 10 minutes: escalate to [person/team]
- Contact info: [Slack channel, phone, email]
```

---

## 6. SLO Tracking and Reporting

### Monthly SLO Report

At the end of each month, calculate:
- **Uptime %** for each tier: `(total_time - downtime) / total_time * 100`
- **Error budget remaining** for each SLO: `(1 - target_slo) * total_seconds - actual_downtime_seconds`
- **Top incident causes** and prevention measures
- **Alert noise:** false alert ratio (alerts that resolve without action)
- **MTTR:** mean time to recovery for critical incidents

### Error Budget Policy

- **Tier 1 (99.9% SLO):** 4.38 hours / month downtime budget
  - If budget is exhausted, pause non-critical deployments and focus on stability
  - Use remaining budget for planned maintenance only

- **Tier 2 (99.5% SLO):** 3.6 hours / month downtime budget
  - If budget is <1 hour, increase test coverage for deployments
  - Plan major changes for low-traffic windows

- **Tier 3 & 4:** Best effort; no hard SLA enforcement on testnet

### Alert Tuning

- Track false alert rate in AlertManager or PagerDuty
- If false alert rate > 5%, investigate and adjust threshold or evaluation period
- If alert fires but resolves without action > 3 times/month, revisit threshold
- Document rationale for all threshold changes in git commit messages

---

## 7. Deployment Checklist

Before deploying to testnet, verify:

- [ ] All new metrics are exported in `/metrics` endpoint
- [ ] Prometheus scrape config includes testnet target
- [ ] Alert rules are deployed to Prometheus
- [ ] AlertManager routes are configured
- [ ] Slack channels exist and have correct subscribers
- [ ] On-call rotation is current and notified
- [ ] Grafana dashboard is updated with any new panels
- [ ] Runbooks exist for all critical alerts
- [ ] Team has reviewed alert thresholds and agrees with SLOs

---

## 8. Related Documentation

- **Monitoring Stack Audit:** [Issue #1881](https://github.com/Ndifreke000/stellar-insights/issues/1881) — documents Prometheus/Jaeger/ELK setup
- **Incident Response Runbook:** `docs/runbooks/mainnet-incident-response.md`
- **Prometheus Metrics Reference:** `docs/prometheus-metrics.md`
- **Backend Observability:** `backend/src/observability/`

---

## Appendix: Threshold Justification

### Why 99.9% for Health Endpoint?
Health checks are used by monitoring systems, Kubernetes, and load balancers. A single failure can trigger cascading failures (e.g., Kubernetes restart loop). We chose aggressive 99.9% to ensure this critical signal is always available. Testnet can tolerate 4.38 hours/month of brief health check unavailability.

### Why 0.5% error threshold for user endpoints?
Testnet serves developers and testers. At this scale, 0.5% error rate (1 in 200 requests) is noticeable to users but not catastrophic. It's low enough to catch real problems (database errors, misconfiguration) but high enough to avoid noise from brief RPC hiccups or cache misses.

### Why 5-second p95 latency for corridors?
Corridors data is frequently accessed in user workflows. At 5-second p95, 95% of requests return in < 5s (fast), and 5% wait a bit longer. This keeps the UX responsive while acknowledging that testnet infrastructure is shared and less predictable than production.

### Why 60% cache miss threshold?
A 60% miss rate means the cache is only saving 40% of requests. This is effectively useless for performance but does add latency. If cache miss rate exceeds 60%, it's a sign that the cache configuration needs tuning (TTLs, invalidation strategy, or memory pressure).

### Why 1-hour job queue depth?
Background jobs should process fast enough that the queue doesn't back up by more than 1 hour. If jobs are queued for an hour, users will see stale data and missed alerts. This threshold catches process bottlenecks early.

