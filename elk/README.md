# ELK Stack for Stellar Insights

This directory contains the Elasticsearch, Logstash, and Kibana (ELK) stack configuration for centralized log aggregation.

## Quick Start (Local Development)

```bash
# Start the ELK stack
cd elk
docker-compose up -d

# Wait for services to be healthy
docker-compose ps

# Access Kibana
# Open http://localhost:5601 in your browser

# View logs
# 1. Create index pattern: Management → Index Patterns → Create "stellar-insights-*"
# 2. Discover tab → Select index pattern
```

## Architecture

```
┌──────────────────────┐
│   Backend Service    │
│  (Rust + tracing)    │
└──────────┬───────────┘
           │
           │ JSON logs via syslog/TCP
           ▼
    ┌─────────────┐
    │  Logstash   │  Parses, filters, redacts
    │  :5000      │
    └──────┬──────┘
           │
           ▼
    ┌─────────────────┐
    │ Elasticsearch   │  Indexes and stores
    │  :9200          │
    └────────┬────────┘
             │
             ▼
        ┌─────────┐
        │ Kibana  │  Query and visualize
        │ :5601   │
        └─────────┘
```

## Configuration

### Elasticsearch (`docker-compose.yml`)
- Single-node cluster (development only)
- `xpack.security.enabled=false` (no authentication in dev)
- Heap size: 512MB (adjust for production)
- Persistent volume: `elasticsearch_data`

### Logstash (`logstash/pipeline/logstash.conf`)
- **Inputs**: TCP/UDP on port 5000 (JSON lines codec)
- **Filters**: 
  - JSON parsing
  - Sensitive data redaction (auth, password, API keys, tokens)
  - Health check endpoint filtering (reduces noise)
  - Environment metadata enrichment
- **Output**: 
  - Elasticsearch with daily index rollover
  - stdout for debugging

### Filebeat (`filebeat/filebeat.yml`)
- Collects Docker container logs
- Forwards to Logstash on port 5000
- Decodes JSON fields for structured log queries

## Connecting the Backend

### Option 1: Direct Syslog Output (Recommended)

In your backend environment or Dockerfile:

```rust
// Backend sends structured JSON logs to Logstash
env "RUST_LOG=info"
env "LOG_FORMAT=json"
// Configure application to send logs to logstash:5000
```

Then pipe logs to Logstash:

```bash
# Example: nc or similar to send logs to logstash
./backend 2>&1 | nc logstash 5000
```

### Option 2: Docker Compose Integration

Add your backend service to `docker-compose.yml`:

```yaml
backend:
  build: ../backend
  ports:
    - "8080:8080"
  environment:
    - DATABASE_URL=sqlite:./stellar_insights.db
    - LOG_FORMAT=json
    - RUST_LOG=info
  depends_on:
    - elasticsearch
  networks:
    - elk
```

## Sensitive Data Redaction

The Logstash pipeline automatically redacts:

1. **Authorization headers**: `Authorization: Bearer ...`
2. **Passwords**: `password: "..."`
3. **API Keys**: `api_key: "..."`
4. **Secrets**: `secret: "..."`
5. **Tokens**: `token: "..."`

The backend also redacts at source via the request/response logging middleware:

- Excludes sensitive headers: Authorization, Cookie, Set-Cookie
- Truncates request/response bodies to 512 bytes
- Gated by `API_LOG_BODIES` environment variable

## Creating Dashboards and Alerts

### Access Kibana

1. Open http://localhost:5601
2. Create index pattern:
   - Management → Index Patterns
   - New pattern: `stellar-insights-*`
   - Time field: `@timestamp`

### Query Examples

**All 5XX errors in the last hour:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "@timestamp": { "gte": "now-1h" } } },
        { "range": { "status": { "gte": 500 } } }
      ]
    }
  }
}
```

**Slow endpoints (> 2 seconds latency):**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "latency_ms": { "gte": 2000 } } }
      ]
    }
  }
}
```

**Errors by request ID (for distributed tracing):**
```
request_id: "abc-def-123" AND level: "ERROR"
```

### Create Alerts

In Kibana, use the Alerting feature to:

1. Monitor error rate thresholds
2. Alert on unusual latency
3. Detect new error patterns
4. Page on-call team for critical issues

## Production Considerations

### High Availability

For production, use:

1. **Elasticsearch Cluster**:
   - Multi-node setup (3+ nodes recommended)
   - Replica shards for resilience
   - ILM (Index Lifecycle Management) for retention

2. **Logstash**:
   - Multiple instances behind a load balancer
   - Persistent queues for durability

3. **Kibana**:
   - Clustered deployment with failover

### Security

1. Enable Elasticsearch authentication (xpack.security)
2. Use TLS for all communication
3. Restrict network access via security groups
4. Audit log access
5. Rotate credentials regularly

### Performance Tuning

1. Adjust Elasticsearch heap based on log volume
2. Configure shard allocation appropriately
3. Use daily or weekly index rollover
4. Implement ILM for old index cleanup
5. Monitor disk usage and retention

### Backup and Recovery

1. Configure snapshot repositories
2. Automated backups of Elasticsearch indices
3. Point-in-time recovery for critical investigations
4. Retention policy aligned with compliance requirements

## Troubleshooting

### Logstash not receiving logs

```bash
# Test connectivity to Logstash
telnet localhost 5000

# Check Logstash logs
docker logs stellar-insights-logstash

# Verify port is listening
netstat -ln | grep 5000
```

### Elasticsearch not responding

```bash
# Check cluster health
curl http://localhost:9200/_cluster/health

# View indices
curl http://localhost:9200/_cat/indices

# Check disk usage
curl http://localhost:9200/_cat/allocation
```

### Kibana won't load

```bash
# Check Kibana logs
docker logs stellar-insights-kibana

# Verify Elasticsearch connection
curl http://localhost:9200 (from within Kibana container)
```

## References

- [Elasticsearch Documentation](https://www.elastic.co/guide/en/elasticsearch/reference/current/)
- [Logstash Documentation](https://www.elastic.co/guide/en/logstash/current/)
- [Kibana Documentation](https://www.elastic.co/guide/en/kibana/current/)
- [Filebeat Documentation](https://www.elastic.co/guide/en/beats/filebeat/current/)
