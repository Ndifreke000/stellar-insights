#!/usr/bin/env bash
set -euo pipefail

# Backend smoke tests run after testnet deployment.
# Uses BACKEND_URL when set; otherwise validates the in-cluster health endpoint via kubectl.

BACKEND_URL="${BACKEND_URL:-}"
NAMESPACE="${NAMESPACE:-stellar-insights-testnet}"
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-120}"

echo "=== Backend testnet smoke tests ==="

if [ -n "$BACKEND_URL" ]; then
  echo "Checking external health endpoint: ${BACKEND_URL}/health"
  for i in $(seq 1 "$TIMEOUT_SECONDS"); do
    if curl -sf "${BACKEND_URL}/health" >/dev/null; then
      echo "PASS: ${BACKEND_URL}/health responded OK"
      exit 0
    fi
    sleep 1
  done
  echo "FAIL: ${BACKEND_URL}/health did not respond within ${TIMEOUT_SECONDS}s"
  exit 1
fi

if ! command -v kubectl >/dev/null 2>&1; then
  echo "SKIP: kubectl not available and BACKEND_URL unset"
  exit 0
fi

POD=$(kubectl get pods -n "$NAMESPACE" -l component=backend -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)
if [ -z "$POD" ]; then
  echo "FAIL: no backend pod found in namespace ${NAMESPACE}"
  exit 1
fi

echo "Checking in-cluster health on pod ${POD}"
RESPONSE=$(kubectl exec -n "$NAMESPACE" "$POD" -- curl -sf http://localhost:8080/health)
echo "PASS: backend health check succeeded"
echo "Response: ${RESPONSE}"
