#!/usr/bin/env bash
set -euo pipefail

# Post-deploy mainnet health probe for CI pipelines and deployment runbooks.
#
# Required: curl, jq
# Optional: kubectl (for in-cluster pod readiness check)
#
# Environment:
#   MAINNET_URL       Base URL of the deployed API (default: https://api.stellar-insights.com)
#   HORIZON_URL       Stellar Horizon endpoint (default: https://horizon.stellar.org)
#   NAMESPACE         Kubernetes namespace for optional pod check (default: stellar-insights-mainnet)
#   TIMEOUT_SECONDS   Max wait for /health to become ready (default: 180)
#   MAX_LEDGER_LAG    Max allowed ledger gap between Horizon tip and indexed ledger (default: 12 ≈ 60s)

MAINNET_URL="${MAINNET_URL:-https://api.stellar-insights.com}"
HORIZON_URL="${HORIZON_URL:-https://horizon.stellar.org}"
NAMESPACE="${NAMESPACE:-stellar-insights-mainnet}"
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-180}"
MAX_LEDGER_LAG="${MAX_LEDGER_LAG:-12}"

MAINNET_URL="${MAINNET_URL%/}"
HORIZON_URL="${HORIZON_URL%/}"

failures=0

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "ERROR: required command not found: $1" >&2
    exit 1
  fi
}

pass() {
  echo "PASS: $1"
}

fail() {
  echo "FAIL: $1" >&2
  failures=$((failures + 1))
}

curl_json() {
  local url=$1
  curl -sf --max-time 30 "$url"
}

require_cmd curl
require_cmd jq

echo "=== Mainnet post-deploy health check ==="
echo "API:     ${MAINNET_URL}"
echo "Horizon: ${HORIZON_URL}"
echo ""

# ── Check 1: GET /health → 200 ──────────────────────────────────────────────
echo -n "Check 1: GET /health returns 200... "
health_body=""
for i in $(seq 1 "$TIMEOUT_SECONDS"); do
  if health_body=$(curl_json "${MAINNET_URL}/health" 2>/dev/null); then
    pass "GET /health"
    break
  fi
  if [ "$i" -eq "$TIMEOUT_SECONDS" ]; then
    fail "GET /health did not respond within ${TIMEOUT_SECONDS}s"
    health_body=""
  fi
  sleep 1
done

if [ -z "$health_body" ]; then
  echo ""
  echo "Aborting remaining checks — API unreachable."
  exit 1
fi

# ── Check 2: network is mainnet ───────────────────────────────────────────────
echo -n "Check 2: network context is mainnet... "
network=""
if network_body=$(curl_json "${MAINNET_URL}/api/v1/network" 2>/dev/null); then
  network=$(echo "$network_body" | jq -r '.network // empty')
elif network_body=$(curl_json "${MAINNET_URL}/api/network/info" 2>/dev/null); then
  network=$(echo "$network_body" | jq -r '.network // empty')
else
  network=$(echo "$health_body" | jq -r '.network.network // empty')
fi

if [ "$network" = "mainnet" ]; then
  pass "network=${network}"
else
  fail "expected network=mainnet, got '${network:-<empty>}'"
fi

# ── Check 3: DB pool stats healthy ────────────────────────────────────────────
echo -n "Check 3: DB pool stats healthy... "
if pool_body=$(curl_json "${MAINNET_URL}/api/v1/db/pool-metrics" 2>/dev/null); then
  pool_size=$(echo "$pool_body" | jq -r '.size // 0')
  pool_active=$(echo "$pool_body" | jq -r '.active // 0')
  pool_idle=$(echo "$pool_body" | jq -r '.idle // 0')

  if [ "$pool_size" -gt 0 ] && [ "$pool_active" -le "$pool_size" ]; then
    pass "size=${pool_size} active=${pool_active} idle=${pool_idle}"
  else
    fail "unhealthy pool metrics (size=${pool_size}, active=${pool_active})"
  fi
else
  fail "GET /api/v1/db/pool-metrics failed"
fi

# ── Check 4: Redis ping (via health cache check) ──────────────────────────────
echo -n "Check 4: Redis ping responds... "
cache_healthy=$(echo "$health_body" | jq -r '.checks.cache.healthy // false')
if [ "$cache_healthy" = "true" ]; then
  pass "checks.cache.healthy=true"
else
  cache_message=$(echo "$health_body" | jq -r '.checks.cache.message // "cache unhealthy"')
  fail "Redis/cache check failed: ${cache_message}"
fi

# ── Check 5: indexed ledger within ~60s of Horizon tip ──────────────────────
echo -n "Check 5: indexed ledger within ${MAX_LEDGER_LAG} ledgers of Horizon tip... "
horizon_body=$(curl -sf --max-time 30 "${HORIZON_URL}/ledgers?order=desc&limit=1")
horizon_seq=$(echo "$horizon_body" | jq -r '._embedded.records[0].sequence // empty')
horizon_closed=$(echo "$horizon_body" | jq -r '._embedded.records[0].closed_at // empty')

indexed_ledger=""
if stats_body=$(curl_json "${MAINNET_URL}/api/analytics/event-stats" 2>/dev/null); then
  indexed_ledger=$(echo "$stats_body" | jq -r '.latestLedger // .latest_ledger // empty')
elif summary_body=$(curl_json "${MAINNET_URL}/api/analytics/verification-summary" 2>/dev/null); then
  indexed_ledger=$(echo "$summary_body" | jq -r '.latestLedger // .latest_ledger // empty')
fi

if [ -z "$horizon_seq" ] || [ "$horizon_seq" = "null" ]; then
  fail "could not read Horizon tip ledger"
elif [ -z "$indexed_ledger" ] || [ "$indexed_ledger" = "null" ]; then
  fail "could not read indexed latest ledger from analytics endpoints"
else
  lag=$((horizon_seq - indexed_ledger))
  if [ "$lag" -lt 0 ]; then
    lag=0
  fi
  if [ "$lag" -le "$MAX_LEDGER_LAG" ]; then
    pass "horizon=${horizon_seq} indexed=${indexed_ledger} lag=${lag} closed_at=${horizon_closed}"
  else
    fail "ledger lag ${lag} exceeds max ${MAX_LEDGER_LAG} (horizon=${horizon_seq}, indexed=${indexed_ledger})"
  fi
fi

# ── Optional: Kubernetes pod readiness ──────────────────────────────────────
if command -v kubectl >/dev/null 2>&1; then
  echo -n "Check 6 (optional): Kubernetes backend pods ready... "
  NOT_READY=$(kubectl get pods -n "$NAMESPACE" -l component=backend \
    -o json 2>/dev/null | jq -r '[.items[] | select(.status.conditions[]? | select(.type=="Ready" and .status!="True"))] | length' || echo "0")
  if [ "${NOT_READY}" != "0" ]; then
    fail "${NOT_READY} backend pod(s) not ready in namespace ${NAMESPACE}"
  else
    pass "all backend pods ready"
  fi
else
  echo "Check 6 (optional): SKIP — kubectl not available"
fi

echo ""
if [ "$failures" -gt 0 ]; then
  echo "FAILED: ${failures} health check(s) did not pass" >&2
  exit 1
fi

echo "SUCCESS: all mainnet health checks passed"
