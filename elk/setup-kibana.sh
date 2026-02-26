#!/bin/bash
# Kibana Dashboard Setup Script
# This script creates index patterns, visualizations, and dashboards in Kibana

set -e

KIBANA_URL="${KIBANA_URL:-http://localhost:5601}"
ELASTICSEARCH_URL="${ELASTICSEARCH_URL:-http://localhost:9200}"

echo "=== Stellar Insights ELK Stack Setup ==="
echo "Kibana URL: $KIBANA_URL"
echo "Elasticsearch URL: $ELASTICSEARCH_URL"

# Wait for Kibana to be ready
echo "Waiting for Kibana to be ready..."
until curl -s "$KIBANA_URL/api/status" | grep -q '"level":"available"'; do
  echo "Kibana not ready yet, waiting..."
  sleep 5
done
echo "✓ Kibana is ready"

# Wait for Elasticsearch to be ready
echo "Waiting for Elasticsearch to be ready..."
until curl -s "$ELASTICSEARCH_URL/_cluster/health" | grep -q '"status":"green\|yellow"'; do
  echo "Elasticsearch not ready yet, waiting..."
  sleep 5
done
echo "✓ Elasticsearch is ready"

# Create index pattern for application logs
echo "Creating index pattern for application logs..."
curl -X POST "$KIBANA_URL/api/saved_objects/index-pattern/stellar-insights-logs" \
  -H 'kbn-xsrf: true' \
  -H 'Content-Type: application/json' \
  -d '{
    "attributes": {
      "title": "stellar-insights-*",
      "timeFieldName": "@timestamp"
    }
  }' || echo "Index pattern may already exist"

# Create index pattern for error logs
echo "Creating index pattern for error logs..."
curl -X POST "$KIBANA_URL/api/saved_objects/index-pattern/stellar-insights-errors" \
  -H 'kbn-xsrf: true' \
  -H 'Content-Type: application/json' \
  -d '{
    "attributes": {
      "title": "stellar-insights-errors-*",
      "timeFieldName": "@timestamp"
    }
  }' || echo "Error index pattern may already exist"

# Set default index pattern
echo "Setting default index pattern..."
curl -X POST "$KIBANA_URL/api/kibana/settings/defaultIndex" \
  -H 'kbn-xsrf: true' \
  -H 'Content-Type: application/json' \
  -d '{
    "value": "stellar-insights-logs"
  }' || echo "Default index pattern may already be set"

# Create Index Lifecycle Management (ILM) policy
echo "Creating ILM policy for log retention..."
curl -X PUT "$ELASTICSEARCH_URL/_ilm/policy/stellar-insights-ilm-policy" \
  -H 'Content-Type: application/json' \
  -d '{
    "policy": {
      "phases": {
        "hot": {
          "min_age": "0ms",
          "actions": {
            "rollover": {
              "max_age": "1d",
              "max_size": "50gb"
            },
            "set_priority": {
              "priority": 100
            }
          }
        },
        "warm": {
          "min_age": "7d",
          "actions": {
            "set_priority": {
              "priority": 50
            },
            "forcemerge": {
              "max_num_segments": 1
            },
            "shrink": {
              "number_of_shards": 1
            }
          }
        },
        "delete": {
          "min_age": "30d",
          "actions": {
            "delete": {}
          }
        }
      }
    }
  }'

# Create index template with ILM policy
echo "Creating index template..."
curl -X PUT "$ELASTICSEARCH_URL/_index_template/stellar-insights-template" \
  -H 'Content-Type: application/json' \
  -d '{
    "index_patterns": ["stellar-insights-*"],
    "template": {
      "settings": {
        "number_of_shards": 1,
        "number_of_replicas": 0,
        "index.lifecycle.name": "stellar-insights-ilm-policy",
        "index.lifecycle.rollover_alias": "stellar-insights",
        "refresh_interval": "5s"
      },
      "mappings": {
        "properties": {
          "@timestamp": { "type": "date" },
          "log_level": { "type": "keyword" },
          "message": { "type": "text" },
          "service": { "type": "keyword" },
          "target": { "type": "keyword" },
          "request_id": { "type": "keyword" },
          "trace_id": { "type": "keyword" },
          "http_method": { "type": "keyword" },
          "http_path": { "type": "keyword" },
          "http_status": { "type": "integer" },
          "response_time_ms": { "type": "long" },
          "response_time_seconds": { "type": "float" },
          "client_ip": { "type": "ip" },
          "user_id": { "type": "keyword" },
          "error_message": { "type": "text" },
          "error_type": { "type": "keyword" },
          "rpc_method": { "type": "keyword" },
          "query": { "type": "text" },
          "environment": { "type": "keyword" }
        }
      }
    }
  }'

echo ""
echo "=== Setup Complete ==="
echo "Access Kibana at: $KIBANA_URL"
echo ""
echo "Next steps:"
echo "1. Go to Kibana > Discover to view logs"
echo "2. Create visualizations in Kibana > Visualize"
echo "3. Build dashboards in Kibana > Dashboard"
echo ""
echo "Common queries:"
echo "  - Error logs: log_level:\"error\""
echo "  - Slow requests: response_time_ms:>1000"
echo "  - Specific service: service:\"stellar-insights\""
