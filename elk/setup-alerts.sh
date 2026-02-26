#!/bin/bash
# Kibana Alerting Setup Script
# Creates alert rules for monitoring critical application events

set -e

KIBANA_URL="${KIBANA_URL:-http://localhost:5601}"

echo "=== Setting up Kibana Alerts ==="

# Create alert for high error rate
echo "Creating alert: High Error Rate"
curl -X POST "$KIBANA_URL/api/alerting/rule" \
  -H 'kbn-xsrf: true' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "High Error Rate - Stellar Insights",
    "tags": ["stellar-insights", "errors", "critical"],
    "rule_type_id": ".es-query",
    "consumer": "alerts",
    "schedule": {
      "interval": "5m"
    },
    "actions": [],
    "params": {
      "index": ["stellar-insights-*"],
      "timeField": "@timestamp",
      "esQuery": "{\"query\":{\"bool\":{\"must\":[{\"term\":{\"log_level\":\"error\"}}],\"filter\":[{\"range\":{\"@timestamp\":{\"gte\":\"now-5m\"}}}]}}}",
      "threshold": [10],
      "thresholdComparator": ">",
      "size": 100,
      "timeWindowSize": 5,
      "timeWindowUnit": "m"
    },
    "notify_when": "onActionGroupChange"
  }'

# Create alert for slow API responses
echo "Creating alert: Slow API Responses"
curl -X POST "$KIBANA_URL/api/alerting/rule" \
  -H 'kbn-xsrf: true' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "Slow API Responses - Stellar Insights",
    "tags": ["stellar-insights", "performance", "warning"],
    "rule_type_id": ".es-query",
    "consumer": "alerts",
    "schedule": {
      "interval": "5m"
    },
    "actions": [],
    "params": {
      "index": ["stellar-insights-*"],
      "timeField": "@timestamp",
      "esQuery": "{\"query\":{\"bool\":{\"must\":[{\"range\":{\"response_time_ms\":{\"gte\":2000}}}],\"filter\":[{\"range\":{\"@timestamp\":{\"gte\":\"now-5m\"}}}]}}}",
      "threshold": [5],
      "thresholdComparator": ">",
      "size": 100,
      "timeWindowSize": 5,
      "timeWindowUnit": "m"
    },
    "notify_when": "onActionGroupChange"
  }'

# Create alert for RPC failures
echo "Creating alert: RPC Call Failures"
curl -X POST "$KIBANA_URL/api/alerting/rule" \
  -H 'kbn-xsrf: true' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "RPC Call Failures - Stellar Insights",
    "tags": ["stellar-insights", "rpc", "critical"],
    "rule_type_id": ".es-query",
    "consumer": "alerts",
    "schedule": {
      "interval": "5m"
    },
    "actions": [],
    "params": {
      "index": ["stellar-insights-*"],
      "timeField": "@timestamp",
      "esQuery": "{\"query\":{\"bool\":{\"must\":[{\"term\":{\"log_level\":\"error\"}},{\"exists\":{\"field\":\"rpc_method\"}}],\"filter\":[{\"range\":{\"@timestamp\":{\"gte\":\"now-5m\"}}}]}}}",
      "threshold": [3],
      "thresholdComparator": ">",
      "size": 100,
      "timeWindowSize": 5,
      "timeWindowUnit": "m"
    },
    "notify_when": "onActionGroupChange"
  }'

# Create alert for database errors
echo "Creating alert: Database Errors"
curl -X POST "$KIBANA_URL/api/alerting/rule" \
  -H 'kbn-xsrf: true' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "Database Errors - Stellar Insights",
    "tags": ["stellar-insights", "database", "critical"],
    "rule_type_id": ".es-query",
    "consumer": "alerts",
    "schedule": {
      "interval": "5m"
    },
    "actions": [],
    "params": {
      "index": ["stellar-insights-*"],
      "timeField": "@timestamp",
      "esQuery": "{\"query\":{\"bool\":{\"must\":[{\"term\":{\"log_level\":\"error\"}},{\"match\":{\"message\":\"database\"}}],\"filter\":[{\"range\":{\"@timestamp\":{\"gte\":\"now-5m\"}}}]}}}",
      "threshold": [1],
      "thresholdComparator": ">",
      "size": 100,
      "timeWindowSize": 5,
      "timeWindowUnit": "m"
    },
    "notify_when": "onActionGroupChange"
  }'

echo ""
echo "=== Alert Setup Complete ==="
echo "View and manage alerts at: $KIBANA_URL/app/management/insightsAndAlerting/triggersActions/rules"
echo ""
echo "Note: Configure notification actions (email, Slack, etc.) in Kibana UI"
