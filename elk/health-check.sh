#!/bin/bash

# ELK Stack Health Check Script
# Verifies all ELK components are running and healthy

set -e

ELASTICSEARCH_URL="${ELASTICSEARCH_URL:-http://localhost:9200}"
KIBANA_URL="${KIBANA_URL:-http://localhost:5601}"
LOGSTASH_URL="${LOGSTASH_URL:-http://localhost:9600}"

echo "🔍 Checking ELK Stack Health..."
echo "================================"

# Check Elasticsearch
echo -n "Elasticsearch: "
if curl -s -f "${ELASTICSEARCH_URL}/_cluster/health" > /dev/null 2>&1; then
    HEALTH=$(curl -s "${ELASTICSEARCH_URL}/_cluster/health" | jq -r '.status')
    echo "✅ Running (Status: $HEALTH)"
    
    # Show cluster stats
    DOCS=$(curl -s "${ELASTICSEARCH_URL}/_cat/count?format=json" | jq -r '.[0].count')
    INDICES=$(curl -s "${ELASTICSEARCH_URL}/_cat/indices?format=json" | jq '. | length')
    echo "   📊 Documents: $DOCS | Indices: $INDICES"
else
    echo "❌ Not reachable"
    exit 1
fi

# Check Logstash
echo -n "Logstash: "
if curl -s -f "${LOGSTASH_URL}/_node/stats" > /dev/null 2>&1; then
    EVENTS=$(curl -s "${LOGSTASH_URL}/_node/stats/events" | jq -r '.events.in')
    echo "✅ Running (Events processed: $EVENTS)"
else
    echo "❌ Not reachable"
    exit 1
fi

# Check Kibana
echo -n "Kibana: "
if curl -s -f "${KIBANA_URL}/api/status" > /dev/null 2>&1; then
    STATUS=$(curl -s "${KIBANA_URL}/api/status" | jq -r '.status.overall.state')
    echo "✅ Running (Status: $STATUS)"
else
    echo "❌ Not reachable"
    exit 1
fi

echo ""
echo "================================"
echo "✅ All ELK components are healthy!"
echo ""
echo "Access URLs:"
echo "  Elasticsearch: ${ELASTICSEARCH_URL}"
echo "  Kibana: ${KIBANA_URL}"
echo "  Logstash API: ${LOGSTASH_URL}"
