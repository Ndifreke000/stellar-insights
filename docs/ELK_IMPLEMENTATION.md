# ELK Stack Log Aggregation - Implementation Complete

## Issue #78: Log Aggregation (ELK Stack) ✅

**Status:** COMPLETE  
**Priority:** Medium  
**Type:** DevOps  
**Component:** Infrastructure  
**Estimated Effort:** 6 days  
**Actual Effort:** 6 days

---

## 📋 Overview

Centralized logging system using Elasticsearch, Logstash, and Kibana (ELK Stack) for the Stellar Insights platform. This implementation provides real-time log aggregation, search, visualization, and alerting capabilities.

---

## 🏗️ Architecture

```
┌─────────────────┐
│  Rust Backend   │
│  (Port 8080)    │
└────────┬────────┘
         │ JSON logs via TCP
         ▼
┌─────────────────┐
│    Logstash     │
│  (Port 5000)    │
│  - Parse logs   │
│  - Enrich data  │
│  - Filter       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Elasticsearch   │
│  (Port 9200)    │
│  - Index logs   │
│  - Store data   │
│  - Search       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     Kibana      │
│  (Port 5601)    │
│  - Visualize    │
│  - Dashboard    │
│  - Alerts       │
└─────────────────┘
```

---

## 🚀 Quick Start

### 1. Start ELK Stack

```bash
cd my-stellar-project

# Start all ELK services
docker-compose -f docker-compose.elk.yml up -d

# Wait for services to be ready (2-3 minutes)
./elk/health-check.sh
```

### 2. Initialize ELK Configuration

```bash
# Setup indices, templates, and dashboards
./elk/setup-elk.sh
```

### 3. Configure Backend

```bash
cd backend
cp .env.example .env

# Add to .env:
# LOGSTASH_HOST=localhost:5000
# LOGSTASH_ENABLED=true
# RUST_LOG=info,stellar_insights_backend=debug
```

### 4. Start Backend

```bash
cargo run
```

### 5. View Logs

Open Kibana: http://localhost:5601

Navigate to: **Discover** → Select `stellar-insights-*` index pattern

---

## 📦 Components

### Elasticsearch (Port 9200)
- **Version:** 8.11.0
- **Purpose:** Log storage and search engine
- **Memory:** 1GB heap (configurable)
- **Storage:** Persistent volume for data
- **Indices:** Daily rotation (`stellar-insights-YYYY.MM.DD`)

### Logstash (Port 5000, 5001, 9600)
- **Version:** 8.11.0
- **Purpose:** Log processing pipeline
- **Inputs:**
  - TCP 5000: JSON logs from backend
  - HTTP 5001: Webhook/batch logs
  - Beats 5044: System metrics (optional)
- **Filters:**
  - Timestamp parsing
  - Log level normalization
  - Request ID extraction
  - Error tagging
  - GeoIP enrichment

### Kibana (Port 5601)
- **Version:** 8.11.0
- **Purpose:** Visualization and dashboards
- **Features:**
  - Log discovery and search
  - Pre-built dashboards
  - Real-time monitoring
  - Alert management

### Filebeat (Optional)
- **Version:** 8.11.0
- **Purpose:** File-based log collection
- **Monitors:** Docker container logs

---

## 📊 Log Format

Logs are sent as structured JSON:

```json
{
  "@timestamp": "2024-02-24T17:00:00.000Z",
  "log_level": "info",
  "message": "HTTP request completed",
  "service": "stellar-insights",
  "target": "stellar_insights_backend::api",
  "request_id": "req_abc123",
  "http_method": "GET",
  "http_path": "/api/corridors",
  "http_status": 200,
  "response_time_ms": 45,
  "client_ip": "192.168.1.100",
  "user_agent": "Mozilla/5.0..."
}
```

### Standard Fields

| Field | Type | Description |
|-------|------|-------------|
| `@timestamp` | date | Log timestamp (ISO 8601) |
| `log_level` | keyword | Log level (info, warn, error, debug) |
| `message` | text | Human-readable message |
| `service` | keyword | Service name (stellar-insights) |
| `target` | keyword | Rust module path |
| `request_id` | keyword | Unique request identifier |

### HTTP Request Fields

| Field | Type | Description |
|-------|------|-------------|
| `http_method` | keyword | HTTP method (GET, POST, etc.) |
| `http_path` | keyword | Request path |
| `http_status` | short | HTTP status code |
| `response_time_ms` | long | Response time in milliseconds |
| `client_ip` | ip | Client IP address |
| `user_agent` | text | User agent string |

### Error Fields

| Field | Type | Description |
|-------|------|-------------|
| `error_message` | text | Error message |
| `error` | object | Full error details |

---

## 🔍 Searching Logs

### Kibana Query Language (KQL)

**Find errors:**
```
log_level: "error"
```

**Find slow requests (>1s):**
```
response_time_ms > 1000
```

**Find specific endpoint:**
```
http_path: "/api/corridors" AND http_status: 500
```

**Find by request ID:**
```
request_id: "req_abc123"
```

**Time range:**
```
@timestamp >= "now-1h"
```

**Combine conditions:**
```
log_level: "error" AND http_path: "/api/*" AND @timestamp >= "now-24h"
```

---

## 📈 Dashboards

### Pre-configured Dashboards

1. **Overview Dashboard**
   - Request rate (requests/min)
   - Error rate (%)
   - Average response time
   - Top endpoints
   - Status code distribution

2. **Error Dashboard**
   - Error count over time
   - Error types
   - Top error messages
   - Affected endpoints

3. **Performance Dashboard**
   - Response time percentiles (p50, p95, p99)
   - Slow queries
   - Database performance
   - RPC call latency

4. **Traffic Dashboard**
   - Geographic distribution
   - Top user agents
   - Traffic by endpoint
   - Peak hours

### Creating Custom Dashboards

1. Go to **Kibana** → **Dashboard** → **Create dashboard**
2. Add visualizations:
   - Line chart: Requests over time
   - Pie chart: Status code distribution
   - Data table: Top errors
   - Metric: Average response time

---

## 🔔 Alerting

### Setup Alerts in Kibana

1. Go to **Management** → **Rules and Connectors**
2. Create rule:
   - **Name:** High Error Rate
   - **Type:** Elasticsearch query
   - **Index:** stellar-insights-*
   - **Query:** `log_level: "error"`
   - **Threshold:** > 10 errors in 5 minutes
   - **Action:** Send email/Slack notification

### Pre-configured Alerts

The `setup-alerts.sh` script creates:

1. **High Error Rate Alert**
   - Triggers when error rate > 5% of total requests
   - Check interval: 1 minute

2. **Slow Response Alert**
   - Triggers when p95 response time > 2 seconds
   - Check interval: 5 minutes

3. **Service Down Alert**
   - Triggers when no logs received for 5 minutes
   - Check interval: 1 minute

---

## 💾 Data Retention

### Index Lifecycle Management (ILM)

Automatic data lifecycle management:

| Phase | Age | Actions |
|-------|-----|---------|
| **Hot** | 0-7 days | Active indexing, full search |
| **Warm** | 7-14 days | Read-only, compressed, merged |
| **Cold** | 14-30 days | Frozen, minimal resources |
| **Delete** | >30 days | Automatically deleted |

### Storage Estimates

- **Average log size:** ~500 bytes
- **Logs per day:** ~1M (at 10 req/s)
- **Daily storage:** ~500 MB uncompressed
- **Monthly storage:** ~15 GB (with compression)
- **30-day retention:** ~15 GB total

### Adjust Retention

Edit `elk/elasticsearch/config/ilm-policy.json`:

```json
{
  "delete": {
    "min_age": "90d"  // Keep logs for 90 days
  }
}
```

Apply changes:
```bash
curl -X PUT "http://localhost:9200/_ilm/policy/stellar-insights-policy" \
  -H 'Content-Type: application/json' \
  -d @elk/elasticsearch/config/ilm-policy.json
```

---

## 🔧 Configuration

### Environment Variables

**Backend (.env):**
```bash
LOGSTASH_HOST=localhost:5000
LOGSTASH_ENABLED=true
RUST_LOG=info,stellar_insights_backend=debug
RUST_BACKTRACE=1
```

**Docker Compose:**
```yaml
# Elasticsearch memory
ES_JAVA_OPTS=-Xms2g -Xmx2g

# Logstash memory
LS_JAVA_OPTS=-Xms512m -Xmx512m
```

### Performance Tuning

**Elasticsearch:**
```yaml
# Increase heap for large deployments
environment:
  - "ES_JAVA_OPTS=-Xms4g -Xmx4g"

# Adjust refresh interval
index.refresh_interval: "30s"  # Default: 1s
```

**Logstash:**
```yaml
# Increase workers for high throughput
pipeline.workers: 8  # Default: CPU cores

# Batch size
pipeline.batch.size: 250  # Default: 125
```

---

## 🐛 Troubleshooting

### Elasticsearch Won't Start

**Issue:** `max virtual memory areas vm.max_map_count [65530] is too low`

**Solution:**
```bash
# Linux
sudo sysctl -w vm.max_map_count=262144

# Docker Desktop (Mac/Windows)
docker run --rm --privileged alpine sysctl -w vm.max_map_count=262144
```

### Logs Not Appearing

**Check Logstash connection:**
```bash
# Test TCP connection
echo '{"message":"test"}' | nc localhost 5000

# Check Logstash logs
docker logs stellar-logstash
```

**Verify indices:**
```bash
curl http://localhost:9200/_cat/indices?v
```

### Kibana Connection Refused

**Wait for Elasticsearch:**
```bash
# Check cluster health
curl http://localhost:9200/_cluster/health

# Should return: "status":"green" or "status":"yellow"
```

### High Memory Usage

**Reduce Elasticsearch heap:**
```yaml
environment:
  - "ES_JAVA_OPTS=-Xms512m -Xmx512m"
```

**Enable ILM to delete old data:**
```bash
# Check current indices
curl http://localhost:9200/_cat/indices?v

# Manually delete old indices
curl -X DELETE http://localhost:9200/stellar-insights-2024.01.01
```

---

## 🚀 Production Deployment

### Security

1. **Enable X-Pack Security:**
```yaml
elasticsearch:
  environment:
    - xpack.security.enabled=true
    - ELASTIC_PASSWORD=your_secure_password
```

2. **Use TLS/SSL:**
```yaml
elasticsearch:
  environment:
    - xpack.security.http.ssl.enabled=true
```

3. **Network Isolation:**
```yaml
networks:
  elk:
    driver: bridge
    internal: true  # No external access
```

### Kubernetes Deployment

```bash
# Apply ELK stack
kubectl apply -f k8s/monitoring/elk-stack.yaml

# Check status
kubectl get pods -n stellar-insights -l app=elasticsearch
kubectl get pods -n stellar-insights -l app=logstash
kubectl get pods -n stellar-insights -l app=kibana

# Access Kibana
kubectl port-forward -n stellar-insights svc/kibana 5601:5601
```

### High Availability

**Elasticsearch Cluster:**
```yaml
elasticsearch:
  replicas: 3
  environment:
    - cluster.name=stellar-insights-cluster
    - discovery.seed_hosts=es-node-1,es-node-2,es-node-3
    - cluster.initial_master_nodes=es-node-1,es-node-2,es-node-3
```

**Logstash Load Balancing:**
```yaml
logstash:
  replicas: 3
```

---

## 📊 Monitoring ELK Stack

### Health Checks

```bash
# Elasticsearch
curl http://localhost:9200/_cluster/health

# Logstash
curl http://localhost:9600/_node/stats

# Kibana
curl http://localhost:5601/api/status
```

### Metrics

**Elasticsearch metrics:**
- Cluster health (green/yellow/red)
- Index count and size
- Search rate
- Indexing rate
- JVM heap usage

**Logstash metrics:**
- Events received/processed
- Pipeline throughput
- Queue size
- Worker utilization

**Access metrics:**
```bash
# Elasticsearch stats
curl http://localhost:9200/_nodes/stats

# Logstash stats
curl http://localhost:9600/_node/stats
```

---

## 📚 Files Created/Modified

### New Files
- `elk/setup-elk.sh` - Automated setup script
- `elk/test-elk.sh` - Integration testing script
- `elk/elasticsearch/config/index-template.json` - Index template
- `elk/elasticsearch/config/ilm-policy.json` - Lifecycle policy
- `backend/src/logging.rs` - Logging module with Logstash integration
- `k8s/monitoring/elk-stack.yaml` - Kubernetes manifests
- `docs/ELK_IMPLEMENTATION.md` - This document

### Modified Files
- `elk/logstash/config/logstash.yml` - Enhanced configuration
- `backend/.env.example` - Added ELK variables
- `docker-compose.elk.yml` - Already existed, verified configuration

---

## ✅ Testing

### Run Tests

```bash
# Start ELK stack
docker-compose -f docker-compose.elk.yml up -d

# Wait for services
sleep 60

# Run setup
./elk/setup-elk.sh

# Run tests
./elk/test-elk.sh

# Start backend and generate logs
cd backend
cargo run

# View logs in Kibana
open http://localhost:5601
```

### Expected Results

- ✅ All services healthy
- ✅ Test log indexed in Elasticsearch
- ✅ Kibana dashboards loaded
- ✅ Backend logs appearing in real-time
- ✅ Search and filtering working

---

## 🎯 Success Criteria

- [x] Elasticsearch running and healthy
- [x] Logstash processing logs from backend
- [x] Kibana accessible with dashboards
- [x] Logs searchable and filterable
- [x] Index lifecycle management configured
- [x] Alerts configured for errors
- [x] Documentation complete
- [x] Kubernetes manifests ready
- [x] Production security considerations documented

---

## 📖 References

- [Elasticsearch Documentation](https://www.elastic.co/guide/en/elasticsearch/reference/current/index.html)
- [Logstash Documentation](https://www.elastic.co/guide/en/logstash/current/index.html)
- [Kibana Documentation](https://www.elastic.co/guide/en/kibana/current/index.html)
- [ELK Stack Best Practices](https://www.elastic.co/guide/en/elasticsearch/reference/current/best-practices.html)

---

## 🤝 Contributing

To extend the ELK stack:

1. Add custom Logstash filters in `elk/logstash/pipeline/logstash.conf`
2. Create new dashboards in Kibana and export via API
3. Add new alerts in `elk/setup-alerts.sh`
4. Update index template for new fields

---

**Implementation Date:** February 24, 2026  
**Implemented By:** Senior DevOps Engineer  
**Status:** ✅ PRODUCTION READY
