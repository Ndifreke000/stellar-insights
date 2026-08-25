# ELK Stack for Log Aggregation

Stellar Insights uses an ELK (Elasticsearch, Logstash, Kibana) stack for centralized, structured log aggregation. This enables operators to search, filter, and analyze application logs across all environments.

## Architecture

```
┌──────────────────────────────┐
│  Backend Service             │
│  (Rust + tracing + JSON logs)│
└──────────────┬───────────────┘
               │
    ┌──────────▼──────────┐
    │ Syslog / TCP        │
    │ (port 5000)         │
    └──────────┬──────────┘
               │
       ┌───────▼────────┐
       │   Logstash     │
       │ • Parse JSON   │
       │ • Filter noise │
       │ • Redact PII   │
       └───────┬────────┘
               │
       ┌───────▼──────────────┐
       │  Elasticsearch       │
       │ • Index documents    │
       │ • Daily rollover     │
       │ • Full-text search   │
       └───────┬──────────────┘
               │
       ┌───────▼────────┐
       │   Kibana       │
       │ • Discover     │
       │ • Visualize    │
       │ • Dashboard    │
       └────────────────┘
```

## Quick Start

### Local Development

```bash
# Start the ELK stack
cd stellar-insights/elk
docker-compose up -d

# Wait for services to be healthy (30-60 seconds)
docker-compose ps

# Access Kibana at http://localhost:5601
```

### Connect Your Backend

**Option 1: Docker Compose (Easiest)**

Add backend to `elk/docker-compose.yml`:

```yaml
backend:
  build: ../backend
  environment:
    - DATABASE_URL=sqlite:./stellar_insights.db
    - LOG_FORMAT=json
    - RUST_LOG=backend=info,tower_http=info
  ports:
    - "8080:8080"
  depends_on:
    - logstash
  networks:
    - elk
```

**Option 2: Pipe Logs to Logstash**

```bash
# From your backend entrypoint or CI script
./stellar-insights-backend 2>&1 | nc logstash 5000
```

## Logging Format

The backend emits **structured JSON logs** via the `tracing` crate:

```json
{
  "timestamp": "2026-08-25T14:30:45.123Z",
  "level": "INFO",
  "message": "→ request",
  "target": "backend",
  "module_path": "backend::observability::logging",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "method": "GET",
  "path": "/api/v1/anchors",
  "query": "status=active",
  "span": {
    "name": "request_middleware"
  }
}
```

Each log entry includes:

| Field | Purpose | Example |
| --- | --- | --- |
| `timestamp` | When the event occurred (ISO8601) | `2026-08-25T14:30:45.123Z` |
| `level` | Severity (TRACE, DEBUG, INFO, WARN, ERROR) | `ERROR` |
| `message` | Human-readable event description | `→ request` or `Database error` |
| `request_id` | Correlation ID across services | `550e8400-e29b-41d4-a716-446655440000` |
| `method` | HTTP method | `GET`, `POST`, `PUT`, `DELETE` |
| `path` | Request URI path | `/api/v1/anchors` |
| `status` | HTTP response code | `200`, `500` |
| `latency_ms` | Request duration in milliseconds | `45` |

### JSON Logging Configuration

Control JSON output via environment variables:

```bash
# Enable JSON format (default: true)
export LOG_FORMAT=json

# Set log level
export RUST_LOG=backend=info,tower_http=info

# Enable request/response body logging (default: false) — use cautiously
export API_LOG_BODIES=true

# Send traces to OTEL collector
export OTEL_ENABLED=true
export OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4318/v1/traces
```

## Logstash Pipeline

The pipeline in `elk/logstash/pipeline/logstash.conf` handles:

### Inputs

- **TCP on port 5000**: JSON-formatted log lines
- **UDP on port 5000**: Fallback for unreliable networks

### Filters

1. **JSON Parsing**: Decodes `message` field as JSON
2. **Timestamp Normalization**: Maps application timestamp to `@timestamp`
3. **Request ID Extraction**: Makes `request_id` a searchable field
4. **Sensitive Data Redaction**:
   - Authorization headers → `REDACTED_AUTH`
   - Passwords → `REDACTED_PASSWORD`
   - API keys → `REDACTED_API_KEY`
   - Secrets → `REDACTED_SECRET`
   - Tokens → `REDACTED_TOKEN`
5. **Noise Filtering**: Drops logs from health check endpoints (`/health`, `/ready`)
6. **Metadata Enrichment**: Adds `application`, `service`, `component` tags

### Outputs

- **Elasticsearch**: Daily indices (`stellar-insights-2026.08.25`)
- **stdout**: Console output for debugging

## Kibana Queries

### Dashboard Access

1. Navigate to http://localhost:5601 (local) or your deployed Kibana URL
2. Select Discover tab
3. Choose `stellar-insights-*` index pattern
4. Use the query syntax below

### Common Queries

**All errors in the last hour:**
```
level: ERROR OR status: 500 OR status: 502 OR status: 503
@timestamp: [now-1h TO now]
```

**Slow API endpoints (>1 second):**
```
latency_ms: [1000 TO *]
path: /api*
```

**Errors for a specific request ID:**
```
request_id: "550e8400-e29b-41d4-a716-446655440000"
level: ERROR
```

**Database connection errors:**
```
message: *database* OR message: *connection*
level: ERROR
```

**All requests to anchor endpoints:**
```
path: /api/v1/anchors*
```

**Performance metrics by endpoint:**
```
path: /api/v1/* | stats avg(latency_ms) as avg_latency, max(latency_ms) as max_latency, count() as total by path
```

## Correlation with APM

Logs and APM traces are correlated via **request_id** (also called `traceparent` in OpenTelemetry):

1. **Backend emits request_id**: Every request gets a unique UUID
2. **Logstash preserves request_id**: Included in Elasticsearch documents
3. **APM agent propagates request_id**: In distributed traces via W3C `traceparent` header
4. **Kibana ↔ APM integration**: Jump between log entry and full trace

**Example workflow:**
```
1. User reports slow API response
2. Find log in Kibana:
   path: /api/v1/anchors
   latency_ms: 5000
   request_id: abc-123-def
3. Click request_id to see full distributed trace (if APM enabled)
4. View database query spans, cache hits, downstream calls
```

## Sensitive Data Handling

### Application-Level Redaction

The request/response logging middleware in `src/observability/logging.rs` redacts:

- **Sensitive headers**: Authorization, Cookie, Set-Cookie (never logged)
- **Request/response bodies**: Truncated to 512 bytes max (set via `API_LOG_BODIES=true`)
- **Pattern matching**: Passwords, tokens, API keys in payloads

### Logstash-Level Redaction

As a second line of defense, Logstash filters redact any patterns that slip through:

```
gsub => [
  "[message]", "(?i)authorization.*", "REDACTED_AUTH",
  "[message]", "(?i)password.*", "REDACTED_PASSWORD"
]
```

### Best Practices

1. **Never log PII directly**: User emails, phone numbers, payment info
2. **Redact in code first**: Don't rely solely on Logstash filters
3. **Audit logs regularly**: Spot-check Kibana for accidental leaks
4. **Limit access**: Restrict Kibana access to ops/SRE teams
5. **Set retention policy**: Auto-delete old logs after N days (see "Production" section)

## Production Deployment

### Elasticsearch Cluster

For high availability and performance:

```hcl
# terraform/modules/elasticsearch/main.tf
resource "aws_opensearch_domain" "logs" {
  domain_name           = "stellar-insights-logs"
  engine_version        = "2.11"
  
  cluster_config {
    instance_type       = "m5.large.search"
    instance_count      = 3  # Minimum for HA
    dedicated_master_enabled = true
    dedicated_master_type = "m5.large.search"
    dedicated_master_count = 3
  }
  
  ebs_options {
    ebs_enabled = true
    volume_size = 500  # GB
    volume_type = "gp3"
  }
  
  encryption_at_rest {
    enabled = true
  }
  
  node_to_node_encryption {
    enabled = true
  }
  
  access_policies = data.aws_iam_policy_document.elasticsearch_access.json
}
```

### Logstash Deployment

Deploy as an ECS service or EC2 auto-scaling group:

```yaml
# .github/workflows/deploy-logstash.yml
jobs:
  deploy-logstash:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Deploy Logstash to ECS
        run: |
          aws ecs update-service \
            --cluster logging \
            --service logstash \
            --force-new-deployment
```

### Kibana Deployment

Deploy as an ECS service behind an ALB:

```yaml
# Terraform or CloudFormation definition
KibanaService:
  Type: AWS::ECS::Service
  Properties:
    ServiceName: stellar-insights-kibana
    DesiredCount: 2
    LoadBalancers:
      - ContainerName: kibana
        ContainerPort: 5601
        TargetGroupArn: !Ref KibanaTargetGroup
```

### Index Lifecycle Management (ILM)

Automatically manage index retention:

```json
{
  "policy": "stellar-insights-ilm",
  "phases": {
    "hot": {
      "min_age": "0d",
      "actions": {
        "rollover": {
          "max_primary_shard_size": "50gb",
          "max_age": "1d"
        }
      }
    },
    "warm": {
      "min_age": "7d",
      "actions": {
        "set_replicas": {
          "number_of_replicas": 1
        }
      }
    },
    "cold": {
      "min_age": "30d",
      "actions": {
        "searchable_snapshot": {
          "snapshot_repository": "backup"
        }
      }
    },
    "delete": {
      "min_age": "90d",
      "actions": {
        "delete": {}
      }
    }
  }
}
```

### Security

1. **Enable TLS**: All communication encrypted end-to-end
2. **Authentication**: IAM roles or basic auth for Elasticsearch
3. **Audit logging**: Track who accesses logs in Kibana
4. **Network isolation**: Private subnets, security groups
5. **Backup**: Automated snapshots to S3

## Monitoring the ELK Stack

### Key Metrics

Monitor these to ensure reliable log aggregation:

| Metric | Healthy | Warning | Critical |
| --- | --- | --- | --- |
| Elasticsearch cluster health | green | yellow | red |
| Logstash pipeline lag | < 1s | 1-10s | > 10s |
| Disk usage | < 70% | 70-80% | > 80% |
| Index ingest rate | > 1000 docs/s | 100-1000 docs/s | < 100 docs/s |
| Kibana response time | < 500ms | 500-2000ms | > 2000ms |

### CloudWatch Alarms

```python
# Alert when Elasticsearch cluster is unhealthy
aws cloudwatch put-metric-alarm \
  --alarm-name elasticsearch-cluster-health \
  --metric-name ClusterHealthStatus \
  --namespace AWS/ES \
  --statistic Minimum \
  --period 300 \
  --threshold 0 \
  --comparison-operator LessThanOrEqualToThreshold \
  --alarm-actions arn:aws:sns:us-east-1:123456789:on-call
```

### Debugging

**Logstash not processing logs:**

```bash
# Check pipeline stats
curl http://logstash:9600/_node/stats/pipelines

# Verify Elasticsearch connectivity
curl http://logstash:9600/_node/pipelines
```

**Elasticsearch disk full:**

```bash
# Delete old indices
curl -X DELETE "localhost:9200/stellar-insights-2026.08.*"

# Increase disk allocation
# (See production Terraform module above)
```

**Kibana slowness:**

```bash
# Check index stats
curl "localhost:9200/_cat/indices?s=docs.count:desc"

# Optimize hot indices
curl -X POST "localhost:9200/stellar-insights-latest/_forcemerge?max_num_segments=1"
```

## References

- [Elasticsearch Documentation](https://www.elastic.co/guide/en/elasticsearch/reference/current/)
- [Logstash Configuration Guide](https://www.elastic.co/guide/en/logstash/current/configuration.html)
- [Kibana Query Language (KQL)](https://www.elastic.co/guide/en/kibana/current/kuery-query.html)
- [OpenTelemetry Logs](https://opentelemetry.io/docs/concepts/signals/logs/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)

## Troubleshooting

### Q: Logs not appearing in Kibana?

**A:** Check these in order:
1. `docker-compose logs logstash` — Are there parsing errors?
2. `curl http://localhost:9200/_cat/indices` — Is index created?
3. Create index pattern in Kibana if not auto-created
4. Verify backend is sending logs to port 5000

### Q: "Index pattern does not exist"?

**A:** Create it manually:
1. Kibana → Management → Index Patterns
2. Click "Create" button
3. Enter pattern: `stellar-insights-*`
4. Select `@timestamp` as time field

### Q: Logs are truncated or missing fields?

**A:** Check Logstash filter:
1. Look for parsing errors in `docker-compose logs logstash`
2. Ensure JSON is valid on the wire (pipe through `jq` to verify)
3. Check redaction rules aren't removing too much

### Q: Retention is eating disk?

**A:** Enable ILM (Index Lifecycle Management):
1. Set up hot-warm-cold-delete tiers
2. Use smaller shard sizes
3. Auto-delete after 90 days (configurable)

## See Also

- [Database Migrations](./DATABASE_MIGRATIONS.md)
- [Blue-Green Deployment](./BLUE_GREEN_DEPLOYMENT.md)
- [APM Integration](./APM_INTEGRATION.md) (coming: #2151)
