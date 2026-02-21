# ELK Stack Setup for Stellar Insights

Centralized logging using Elasticsearch, Logstash, and Kibana (ELK Stack).

## Architecture

```
Rust Backend → Logstash (TCP:5000) → Elasticsearch (9200) → Kibana (5601)
```

## Quick Start

### 1. Start ELK Stack

```bash
# Start all ELK services
docker-compose -f docker-compose.elk.yml up -d

# Or use the convenience script
./scripts/start-elk.sh
```

### 2. Configure Backend

Add to `backend/.env`:

```bash
LOGSTASH_HOST=localhost:5000
RUST_LOG=info
```

### 3. Start Backend

```bash
cd backend
cargo run
```

### 4. View Logs in Kibana

Open http://localhost:5601

1. Go to **Management** → **Stack Management** → **Index Patterns**
2. Create index pattern: `stellar-insights-*`
3. Select `@timestamp` as time field
4. Go to **Discover** to view logs

## Services

| Service | Port | URL |
|---------|------|-----|
| Elasticsearch | 9200 | http://localhost:9200 |
| Logstash | 5000 (TCP) | - |
| Kibana | 5601 | http://localhost:5601 |

## Log Format

Logs are sent as JSON with the following fields:

```json
{
  "@timestamp": "2024-01-15T10:30:00Z",
  "log_level": "info",
  "message": "Server starting on 127.0.0.1:8080",
  "target": "stellar_insights_backend",
  "service": "stellar-insights"
}
```

## Index Management

Logs are stored in daily indices: `stellar-insights-YYYY.MM.DD`

### Retention Policy

Set up Index Lifecycle Management (ILM) in Kibana:

1. Go to **Management** → **Index Lifecycle Policies**
2. Create policy:
   - Hot phase: 7 days
   - Delete phase: 30 days

## Queries

### Common Kibana Queries

**Error logs:**
```
log_level: "error"
```

**Specific service:**
```
service: "stellar-insights" AND log_level: "warn"
```

**Time range:**
```
@timestamp: [now-1h TO now]
```

## Production Configuration

### Security

Enable X-Pack security in `docker-compose.elk.yml`:

```yaml
elasticsearch:
  environment:
    - xpack.security.enabled=true
    - ELASTIC_PASSWORD=your_password
```

### Resource Limits

Adjust memory in `docker-compose.elk.yml`:

```yaml
elasticsearch:
  environment:
    - "ES_JAVA_OPTS=-Xms2g -Xmx2g"
```

### Persistent Storage

Volumes are automatically created:
- `es_data`: Elasticsearch data

## Troubleshooting

### Elasticsearch not starting

Check memory limits:
```bash
docker logs stellar-elasticsearch
```

Increase vm.max_map_count:
```bash
# Linux
sudo sysctl -w vm.max_map_count=262144

# Windows (WSL2)
wsl -d docker-desktop
sysctl -w vm.max_map_count=262144
```

### Logs not appearing

1. Check Logstash connection:
```bash
curl -X GET "localhost:9200/_cat/indices?v"
```

2. Verify backend connection:
```bash
# Should see LOGSTASH_HOST in logs
grep LOGSTASH backend/.env
```

3. Check Logstash logs:
```bash
docker logs stellar-logstash
```

### Kibana connection refused

Wait for Elasticsearch to be ready:
```bash
curl http://localhost:9200/_cluster/health
```

## Maintenance

### Stop ELK Stack

```bash
docker-compose -f docker-compose.elk.yml down
```

### Clear all data

```bash
docker-compose -f docker-compose.elk.yml down -v
```

### View logs

```bash
docker logs -f stellar-elasticsearch
docker logs -f stellar-logstash
docker logs -f stellar-kibana
```

## Advanced Configuration

### Custom Logstash Pipeline

Edit `elk/logstash/pipeline/logstash.conf`:

```ruby
filter {
  # Add custom filters
  if [log_level] == "error" {
    mutate {
      add_tag => ["alert"]
    }
  }
}
```

### Elasticsearch Mappings

Create custom mappings via Kibana Dev Tools:

```json
PUT stellar-insights-*/_mapping
{
  "properties": {
    "response_time": { "type": "long" },
    "user_id": { "type": "keyword" }
  }
}
```

## Monitoring

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

View in Kibana:
- **Stack Monitoring** → **Elasticsearch**
- **Stack Monitoring** → **Logstash**

## Integration with Alerting

Configure alerts in Kibana:

1. Go to **Management** → **Rules and Connectors**
2. Create rule for error threshold
3. Set up email/Slack notifications

## Cost Optimization

- Use ILM to delete old indices
- Adjust refresh interval: `PUT stellar-insights-*/_settings {"index.refresh_interval": "30s"}`
- Disable replicas for single-node: `"number_of_replicas": 0`

## References

- [Elasticsearch Documentation](https://www.elastic.co/guide/en/elasticsearch/reference/current/index.html)
- [Logstash Documentation](https://www.elastic.co/guide/en/logstash/current/index.html)
- [Kibana Documentation](https://www.elastic.co/guide/en/kibana/current/index.html)
