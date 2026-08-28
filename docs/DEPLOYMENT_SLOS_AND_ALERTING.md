# Deploying SLOs and Alerting for Stellar Insights Testnet

This guide walks through deploying the SLO definitions and alert rules to your Stellar Insights testnet monitoring stack.

## Prerequisites

- **Prometheus** running and scraping metrics from your backend
- **AlertManager** deployed and accessible
- **Grafana** for dashboard visualization (optional but recommended)
- **Stellar Insights Backend** running with `/metrics` endpoint enabled

Deployment status of monitoring infrastructure was audited in [Issue #1881](https://github.com/Ndifreke000/stellar-insights/issues/1881).

---

## 1. Deploy Prometheus Alert Rules

Alert rules define **what** to alert on (thresholds, conditions, etc.).

### Option A: Docker / Docker Compose

If running Prometheus in Docker:

```bash
# Copy alert rules into the container
docker cp monitoring/prometheus-alert-rules.yaml \
  prometheus:/etc/prometheus/alert-rules.yaml

# Reload Prometheus (signal HUP to reload config without downtime)
docker exec prometheus kill -HUP 1
```

### Option B: Kubernetes

If running Prometheus via Helm or Kubernetes:

```bash
# Create a ConfigMap with the alert rules
kubectl create configmap prometheus-alert-rules \
  --from-file=monitoring/prometheus-alert-rules.yaml \
  -n monitoring \
  --dry-run=client -o yaml | kubectl apply -f -

# Update Prometheus Helm values to reference the ConfigMap
# In your prometheus-values.yaml:
# prometheusSpec:
#   ruleSelector:
#     matchLabels:
#       prometheus: "stellar-insights"
#   additionalScrapeConfigs:
#     - job_name: 'stellar-insights'
#       static_configs:
#         - targets: ['localhost:8080']

# Redeploy Prometheus
helm upgrade prometheus prometheus-community/kube-prometheus-stack \
  -f prometheus-values.yaml -n monitoring
```

### Option C: Manual Prometheus Configuration

Edit `prometheus.yml` and add the rules file:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "alert-rules.yaml"  # Add this line

alerting:
  alertmanagers:
    - static_configs:
        - targets:
            - alertmanager:9093

scrape_configs:
  - job_name: 'stellar-insights'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

Then restart Prometheus:

```bash
systemctl restart prometheus
# or
supervisorctl restart prometheus
```

### Verify Alert Rules Loaded

Check Prometheus web UI at `http://prometheus:9090/alerts`:
- You should see all alerts in "Alert" section
- Green status means no active alerts
- Red status means alerts are firing

```bash
# Or query via API
curl 'http://prometheus:9090/api/v1/rules' | jq '.data.groups[] | select(.name == "stellar_insights_testnet_slos")'
```

---

## 2. Configure AlertManager

AlertManager handles alert routing, grouping, and notification delivery (Slack, PagerDuty, email).

### Step 1: Set Environment Variables

```bash
# Slack webhook URL (from your Slack app configuration)
export SLACK_WEBHOOK_URL="https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK"

# PagerDuty integration key (from PagerDuty service configuration)
export PAGERDUTY_SERVICE_KEY="YOUR_PAGERDUTY_KEY"

# Email configuration (optional)
export SMTP_HOST="smtp.gmail.com"
export SMTP_PORT="587"
export SMTP_USER="your-email@gmail.com"
export SMTP_PASSWORD="your-app-password"
```

### Step 2: Substitute Environment Variables

```bash
# Use envsubst to inject variables into the config
envsubst < monitoring/alertmanager-config.yaml > /tmp/alertmanager-config.yaml

# Verify variables were substituted
grep -E 'SLACK|PAGERDUTY|SMTP' /tmp/alertmanager-config.yaml
# Should show actual URLs/keys, not ${VAR} placeholders
```

### Step 3: Deploy AlertManager Configuration

**Docker:**
```bash
docker cp /tmp/alertmanager-config.yaml alertmanager:/etc/alertmanager/config.yml
docker exec alertmanager killall -HUP alertmanager
```

**Kubernetes:**
```bash
kubectl create configmap alertmanager-config \
  --from-file=/tmp/alertmanager-config.yaml \
  -n monitoring \
  --dry-run=client -o yaml | kubectl apply -f -

kubectl rollout restart statefulset/alertmanager -n monitoring
```

**Standalone:**
```bash
cp /tmp/alertmanager-config.yaml /etc/alertmanager/alertmanager.yml
systemctl restart alertmanager
```

### Step 4: Verify AlertManager Configuration

```bash
# Check AlertManager web UI at http://alertmanager:9093
# Navigate to "Config" tab to see loaded configuration

# Or verify via API
curl 'http://alertmanager:9093/api/v1/status' | jq '.data'
```

---

## 3. Create Slack Channels and Configure Permissions

### Create Channels in Slack

**#stellar-insights-alerts** (critical incidents)
- Topic: "Critical alerts for Stellar Insights testnet (SEV-1 incidents)"
- Description: "Real-time notifications for production incidents requiring immediate response"
- Access: Restricted to on-call engineers + management
- Notification settings: @here mentions enabled

**#stellar-insights-dev** (warnings and dev alerts)
- Topic: "Development team alerts for Stellar Insights testnet"
- Description: "Non-critical alerts and warnings for developer investigation"
- Access: All developers
- Notification settings: Normal; no @here

**#stellar-insights-monitoring** (audit log)
- Topic: "Audit log of all Stellar Insights metrics and alerts"
- Description: "Historical record of all alerts fired (no action required)"
- Access: Optional for interested developers
- Notification settings: Off

### Create Incoming Webhooks

For each channel that needs alerts:

1. Go to Slack workspace → Settings & administration → Manage apps
2. Search for "Incoming Webhooks"
3. Click "Add to Slack"
4. Select channel: **#stellar-insights-alerts**
5. Copy the webhook URL
6. Set environment variable: `export SLACK_WEBHOOK_URL="https://hooks.slack.com/..."`
7. Repeat for other channels if different webhooks are desired

---

## 4. Configure PagerDuty (Optional)

PagerDuty automatically escalates critical incidents if they're not acknowledged.

### Create PagerDuty Integration

1. Log into PagerDuty
2. Create a new Service for "Stellar Insights Testnet"
3. In Service → Integrations → Add an integration → "Prometheus"
4. Copy the "Integration Key"
5. Set environment variable: `export PAGERDUTY_SERVICE_KEY="..."`

### Set Up On-Call Schedule

1. PagerDuty → Escalation Policies → Create new policy
2. Configure escalation levels:
   - **Level 1 (0 min):** Primary on-call engineer
   - **Level 2 (15 min):** Engineering manager
   - **Level 3 (30 min):** Team lead
3. Create an on-call schedule that rotates weekly
4. Attach schedule to the "Stellar Insights Testnet" service

### Test PagerDuty Integration

```bash
# Manually trigger a test alert to verify routing
curl -X POST http://prometheus:9090/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d '{
    "alerts": [
      {
        "status": "firing",
        "labels": {
          "severity": "critical",
          "alertname": "TestAlert"
        },
        "annotations": {
          "summary": "Test alert from Prometheus",
          "description": "This is a test alert to verify PagerDuty integration"
        }
      }
    ]
  }'

# Check PagerDuty for incident creation
# You should receive a phone call/SMS if set up correctly
```

---

## 5. Create Grafana Dashboard (Optional)

A visual dashboard makes it easier to spot trends and correlate metrics.

### Option A: Import Pre-Built Dashboard

The Grafana community has Prometheus dashboards. To import one:

1. Grafana → Dashboards → Browse
2. Import → "1860" (Node Exporter Full) or search "Prometheus"
3. Select Prometheus data source
4. Click Import

### Option B: Create Custom Dashboard

```bash
# Copy the dashboard JSON (see below) to a file
cat > /tmp/stellar-insights-dashboard.json << 'EOF'
{
  "dashboard": {
    "title": "Stellar Insights Testnet - SLOs",
    "panels": [
      {
        "title": "Health Endpoint Availability",
        "targets": [
          {
            "expr": "(rate(http_requests_total{path=\"/health\",status=~\"2..\"}[5m]) / rate(http_requests_total{path=\"/health\"}[5m])) * 100",
            "legendFormat": "{{ instance }}"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "percent",
            "min": 0,
            "max": 100,
            "thresholds": {
              "steps": [
                { "value": null, "color": "red" },
                { "value": 99.9, "color": "green" }
              ]
            }
          }
        }
      }
    ]
  }
}
EOF

# Import into Grafana via UI or API
curl -X POST http://grafana:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GRAFANA_API_KEY" \
  -d @/tmp/stellar-insights-dashboard.json
```

### Option C: Write Your Own

Use the dashboard JSON structure in the SLOs documentation. Key panels:

| Panel | PromQL Query | Unit | Thresholds |
|-------|---|---|---|
| Health availability | `rate(http_requests_total{path="/health",status=~"2.."}[5m]) / rate(http_requests_total[5m])` | % | 99.9% |
| Anchors error rate | `rate(http_errors_total{path="/anchors"}[5m]) / rate(http_requests_total{path="/anchors"}[5m])` | % | 0.5% warning |
| Corridors p95 latency | `histogram_quantile(0.95, rate(http_request_duration_seconds_bucket{path="/corridors"}[5m]))` | s | 5s warning |
| DB pool utilization | `(db_pool_active / db_pool_size) * 100` | % | 90% warning |
| Cache hit rate | `rate(cache_hits_total[5m]) / (rate(cache_hits_total[5m]) + rate(cache_misses_total[5m]))` | % | 50% warning |

---

## 6. Testing Alerts

### Generate a Test Alert

To verify alerts are working end-to-end:

```bash
# Simulate a slow health endpoint (should trigger HealthCheckLatency alert)
# This assumes you can slow down the health endpoint temporarily via a toggle or feature flag

# Or manually modify Prometheus alert rules temporarily:
sed -i 's/expr: rate(http_errors_total{path="\/health"}\[5m\]) > 0.01/expr: rate(http_requests_total{path="\/health"}\[5m\]) > 0/' \
  /etc/prometheus/alert-rules.yaml

systemctl restart prometheus

# Wait 2-5 minutes for alert to fire
# Check Prometheus: http://prometheus:9090/alerts
# Verify Slack notification arrived in #stellar-insights-alerts
# Check PagerDuty for incident creation

# Revert the temporary change
git checkout /etc/prometheus/alert-rules.yaml
systemctl restart prometheus
```

### Verify Alert Routing

Check AlertManager logs for any routing errors:

```bash
# Docker
docker logs alertmanager | grep -i "error\|warning\|webhook"

# Kubernetes
kubectl logs -n monitoring deployment/alertmanager | grep -i "webhook"

# Systemd
journalctl -u alertmanager -f
```

---

## 7. On-Call Runbook Setup

Create a runbook for each critical alert so responders know what to do.

### Runbook Template

```bash
mkdir -p docs/runbooks

cat > docs/runbooks/health-endpoint-down.md << 'EOF'
# Health Endpoint Down

## Detection
- **Alert:** HealthCheckEndpointDown
- **Threshold:** Error rate > 1% for 2 minutes
- **SLO:** 99.9% availability

## Impact
- Kubernetes may restart pods (liveness probe failures)
- Monitoring dashboards show backend as unhealthy
- Load balancer may mark backend as down

## Diagnosis
1. Check if backend is running:
   ```bash
   curl http://localhost:8080/health
   ```
2. Check backend logs:
   ```bash
   docker logs stellar-insights-backend | tail -50
   ```
3. Verify database connectivity:
   ```bash
   curl http://localhost:8080/api/v1/db/pool-metrics
   ```
4. Check cache connectivity (Redis):
   ```bash
   redis-cli ping
   ```

## Remediation
- Restart backend: `docker restart stellar-insights-backend`
- Restart database: `docker restart stellar-insights-postgres`
- Restart cache: `docker restart stellar-insights-redis`
- If persistent, check logs for configuration errors

## Escalation
- If unresolved after 5 minutes: page secondary on-call
- If unresolved after 15 minutes: page engineering manager
EOF
```

### Runbook Locations

Place all runbooks in: `docs/runbooks/`

Reference them in alert annotations:

```yaml
annotations:
  runbook_url: "https://github.com/Ndifreke000/stellar-insights/blob/main/docs/runbooks/health-endpoint-down.md"
```

---

## 8. Monitoring the SLOs

### Monthly SLO Report

Calculate uptime for each SLO at month-end:

```bash
#!/bin/bash
# Query Prometheus for downtime (when alerts fired)
curl 'http://prometheus:9090/api/v1/query_range' \
  --data-urlencode 'query=ALERTS{alertname="HealthCheckEndpointDown"}' \
  --data-urlencode 'start=2025-01-01T00:00:00Z' \
  --data-urlencode 'end=2025-02-01T00:00:00Z' \
  --data-urlencode 'step=1m' | jq '.data.result'

# Calculate total downtime and remaining error budget
# (1 - SLO_target) * seconds_in_month - actual_downtime_seconds
```

### Alert Noise Tracking

Monitor alert noise to avoid alert fatigue:

```bash
# Query AlertManager for alert counts
curl 'http://alertmanager:9093/api/v1/alerts' | jq '[.data[] | .labels.alertname] | group_by(.) | map({name: .[0], count: length})'

# Track false positives (alerts that resolve without action)
# If > 10% of alerts resolve within 5 minutes, consider adjusting thresholds
```

---

## 9. Troubleshooting

### Alerts Not Firing

```bash
# Check alert rules loaded
curl http://prometheus:9090/api/v1/rules | jq '.data.groups[] | select(.name == "stellar_insights_testnet_slos")'

# Check if conditions are met
curl 'http://prometheus:9090/api/v1/query?query=rate(http_errors_total{path="/health"}[5m])'

# If query returns empty, metrics may not be exported
curl http://localhost:8080/metrics | grep http_errors
```

### Alerts Not Routing to Slack

```bash
# Check AlertManager configuration
curl http://alertmanager:9093/api/v1/status | jq '.data.config'

# Check AlertManager logs
docker logs alertmanager | grep -i "slack\|webhook"

# Test Slack webhook manually
curl -X POST $SLACK_WEBHOOK_URL \
  -H 'Content-type: application/json' \
  -d '{"text":"Test message from AlertManager"}'
```

### PagerDuty Not Creating Incidents

```bash
# Verify PagerDuty integration key is correct
curl http://alertmanager:9093/api/v1/status | grep pagerduty

# Check AlertManager logs for PagerDuty errors
docker logs alertmanager | grep -i "pagerduty"

# Test PagerDuty API directly (requires service key)
curl -X POST https://events.pagerduty.com/v2/enqueue \
  -H 'Content-type: application/json' \
  -d "{\"routing_key\":\"$PAGERDUTY_SERVICE_KEY\",\"event_action\":\"trigger\",\"dedup_key\":\"test\",\"payload\":{\"summary\":\"Test incident\",\"severity\":\"critical\",\"source\":\"AlertManager\"}}"
```

---

## 10. Next Steps

1. **Deploy alert rules** to Prometheus
2. **Configure AlertManager** with Slack/PagerDuty
3. **Create runbooks** for top 5 alerts
4. **Test alerts** by generating test conditions
5. **Create Grafana dashboard** for visualization
6. **Brief on-call team** on alert SLOs and runbooks
7. **Set up monthly SLO reports** to track performance
8. **Tune alerts** over first 4 weeks based on noise and true positives

---

## Additional Resources

- **Prometheus AlertManager documentation:** https://prometheus.io/docs/alerting/latest/overview/
- **PagerDuty integration guide:** https://www.pagerduty.com/docs/guides/prometheus-integration/
- **Grafana dashboard best practices:** https://grafana.com/docs/grafana/latest/dashboards/
- **Stellar Insights SLOs document:** `docs/SLOs_AND_ALERTING.md`
- **Monitoring infrastructure audit:** Issue #1881
