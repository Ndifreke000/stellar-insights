# ELK Stack Quick Reference

## 🚀 Quick Commands

### Start/Stop
```bash
# Start ELK
docker-compose -f docker-compose.elk.yml up -d

# Stop ELK
docker-compose -f docker-compose.elk.yml down

# Stop and remove data
docker-compose -f docker-compose.elk.yml down -v
```

### Setup
```bash
# Initial setup
./elk/setup-elk.sh

# Test integration
./elk/test-elk.sh

# Health check
./elk/health-check.sh
```

### View Logs
```bash
docker logs -f stellar-elasticsearch
docker logs -f stellar-logstash
docker logs -f stellar-kibana
```

## 🔗 Access URLs

| Service | URL |
|---------|-----|
| Kibana | http://localhost:5601 |
| Elasticsearch | http://localhost:9200 |
| Logstash Monitoring | http://localhost:9600 |

## 🔍 Common Queries

### Kibana (KQL)

```
# Errors only
log_level: "error"

# Slow requests
response_time_ms > 1000

# Specific endpoint
http_path: "/api/corridors"

# Last hour
@timestamp >= "now-1h"

# Combine
log_level: "error" AND http_path: "/api/*"
```

### Elasticsearch (API)

```bash
# Search all logs
curl "http://localhost:9200/stellar-insights-*/_search?pretty"

# Search errors
curl "http://localhost:9200/stellar-insights-*/_search?q=log_level:error&pretty"

# Count logs
curl "http://localhost:9200/stellar-insights-*/_count?pretty"
```

## 📊 Useful Endpoints

```bash
# Cluster health
curl http://localhost:9200/_cluster/health?pretty

# List indices
curl http://localhost:9200/_cat/indices?v

# Index stats
curl http://localhost:9200/stellar-insights-*/_stats?pretty

# Node stats
curl http://localhost:9200/_nodes/stats?pretty

# Logstash stats
curl http://localhost:9600/_node/stats?pretty
```

## 🐛 Troubleshooting

### Elasticsearch won't start
```bash
# Increase vm.max_map_count
sudo sysctl -w vm.max_map_count=262144
```

### No logs appearing
```bash
# Test Logstash
echo '{"message":"test"}' | nc localhost 5000

# Check indices
curl http://localhost:9200/_cat/indices?v
```

### High memory usage
```bash
# Reduce heap in docker-compose.elk.yml
ES_JAVA_OPTS=-Xms512m -Xmx512m
```

## 📝 Log Format

```json
{
  "@timestamp": "2024-02-24T17:00:00Z",
  "log_level": "info",
  "message": "Request completed",
  "request_id": "req_123",
  "http_method": "GET",
  "http_path": "/api/corridors",
  "http_status": 200,
  "response_time_ms": 45
}
```

## 🔔 Alert Thresholds

- **High Error Rate:** > 5% of requests
- **Slow Response:** p95 > 2 seconds
- **Service Down:** No logs for 5 minutes

## 💾 Data Retention

- **Hot:** 0-7 days (active)
- **Warm:** 7-14 days (compressed)
- **Cold:** 14-30 days (frozen)
- **Delete:** > 30 days

## 🔧 Configuration Files

- `docker-compose.elk.yml` - Docker services
- `elk/logstash/config/logstash.yml` - Logstash config
- `elk/logstash/pipeline/logstash.conf` - Log pipeline
- `elk/elasticsearch/config/index-template.json` - Index template
- `elk/elasticsearch/config/ilm-policy.json` - Retention policy

## 📚 Documentation

- Full docs: `docs/ELK_IMPLEMENTATION.md`
- Setup guide: `docs/ELK_SETUP.md`
- README: `elk/README.md`
