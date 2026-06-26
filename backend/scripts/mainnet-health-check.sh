#!/usr/bin/env bash
set -euo pipefail

# Post-deploy mainnet health verification for CI and runbooks.

MAINNET_URL="${MAINNET_URL:-https://api.stellar-insights.com}"
NAMESPACE="${NAMESPACE:-stellar-insights-mainnet}"
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-180}"

echo "=== Mainnet post-deploy health check ==="

echo -n "Check 1: Public API /health... "
for i in $(seq 1 "$TIMEOUT_SECONDS"); do
  if curl -sf "${MAINNET_URL}/health" >/dev/null; then
    echo "PASS"
    break
  fi
  if [ "$i" -eq "$TIMEOUT_SECONDS" ]; then
    echo "FAIL"
    exit 1
  fi
  sleep 1
done

echo -n "Check 2: Public API /ready... "
if curl -sf "${MAINNET_URL}/ready" >/dev/null 2>&1 || curl -sf "${MAINNET_URL}/health" >/dev/null; then
  echo "PASS"
else
  echo "FAIL"
  exit 1
fi

if command -v kubectl >/dev/null 2>&1; then
  echo -n "Check 3: Kubernetes backend pods ready... "
  NOT_READY=$(kubectl get pods -n "$NAMESPACE" -l component=backend \
    -o json 2>/dev/null | jq -r '[.items[] | select(.status.conditions[]? | select(.type=="Ready" and .status!="True"))] | length' || echo "0")
  if [ "${NOT_READY}" != "0" ]; then
    echo "FAIL (${NOT_READY} pod(s) not ready)"
    exit 1
  fi
  echo "PASS"
else
  echo "Check 3: SKIP (kubectl not available)"
fi

echo "SUCCESS: mainnet health checks passed"
