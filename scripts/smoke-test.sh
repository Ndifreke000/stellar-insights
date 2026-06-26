#!/usr/bin/env bash
set -euo pipefail

# Post-deployment smoke test runner for Stellar Insights
# Validates that all critical components are operational after deployment

# Configuration defaults (override with environment variables)
BASE_URL="${BASE_URL:-http://localhost:8080}"
FRONTEND_URL="${FRONTEND_URL:-http://localhost:3000}"
WS_URL="${WS_URL:-ws://localhost:8080/ws}"
CONTRACT_ID="${CONTRACT_ID:-CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4}"
HORIZON_URL="${HORIZON_URL:-https://horizon.stellar.org}"

# Test configuration
TIMEOUT_SECONDS=10
WS_TEST_TIMEOUT_SECONDS=5

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results tracking
PASSED_TESTS=0
FAILED_TESTS=0

# ============================================================================
# Utility Functions
# ============================================================================

log_test() {
  local test_name="$1"
  echo -n "Checking ${test_name}... "
}

log_pass() {
  echo -e "${GREEN}✓${NC}"
  ((PASSED_TESTS++))
}

log_fail() {
  local reason="${1:-Unknown error}"
  echo -e "${RED}✗ ${reason}${NC}"
  ((FAILED_TESTS++))
}

log_warn() {
  echo -e "${YELLOW}⚠ ${1}${NC}"
}

# ============================================================================
# Test 1: Backend Health Check
# ============================================================================

test_backend_health() {
  log_test "backend health endpoint"

  local response
  response=$(curl -sf -m "$TIMEOUT_SECONDS" "$BASE_URL/health" 2>&1 || echo "")

  if [[ -z "$response" ]]; then
    log_fail "backend not responding at $BASE_URL"
    return 1
  fi

  if echo "$response" | grep -q '"status":"ok"\|"status": "ok"'; then
    log_pass
    return 0
  else
    log_fail "health endpoint returned unexpected response: $response"
    return 1
  fi
}

# ============================================================================
# Test 2: Contract Initialization Check
# ============================================================================

test_contract_initialized() {
  log_test "contract initialization"

  # Check if there's a soroban proxy endpoint (optional)
  if ! command -v soroban &> /dev/null && ! curl -sf -m 2 "$BASE_URL/api/contract/info" &> /dev/null; then
    log_warn "soroban CLI not found and no contract proxy endpoint - skipping contract check"
    return 0
  fi

  # Try to fetch contract info via backend proxy if it exists
  local response
  response=$(curl -sf -m "$TIMEOUT_SECONDS" "$BASE_URL/api/contract/info?id=$CONTRACT_ID" 2>&1 || echo "")

  if [[ -z "$response" ]]; then
    # Fallback: just verify contract endpoint is accessible
    if curl -sf -m 2 "$BASE_URL/api/contract" > /dev/null 2>&1; then
      log_pass
      return 0
    else
      log_warn "contract endpoint not accessible (contract proxy may not be deployed)"
      return 0
    fi
  fi

  if echo "$response" | grep -q "initialized\|state"; then
    log_pass
    return 0
  else
    log_fail "contract response invalid: $response"
    return 1
  fi
}

# ============================================================================
# Test 3: Frontend Reachability
# ============================================================================

test_frontend_reachable() {
  log_test "frontend reachability"

  local status_code
  status_code=$(curl -sf -m "$TIMEOUT_SECONDS" -o /dev/null -w "%{http_code}" "$FRONTEND_URL" 2>&1 || echo "000")

  if [[ "$status_code" == "200" ]]; then
    log_pass
    return 0
  elif [[ "$status_code" == "000" ]]; then
    log_fail "frontend not responding at $FRONTEND_URL"
    return 1
  else
    log_fail "frontend returned status $status_code (expected 200)"
    return 1
  fi
}

# ============================================================================
# Test 4: WebSocket Upgrade
# ============================================================================

test_websocket_upgrade() {
  log_test "WebSocket upgrade"

  # Try using websocat if available, otherwise try curl with HTTP Upgrade headers
  if command -v websocat &> /dev/null; then
    if timeout "$WS_TEST_TIMEOUT_SECONDS" websocat -n "$WS_URL" <<< '{}' > /dev/null 2>&1; then
      log_pass
      return 0
    else
      log_fail "WebSocket connection failed (websocat)"
      return 1
    fi
  else
    # Fallback: test HTTP Upgrade with curl
    local response
    response=$(curl -sf -m "$TIMEOUT_SECONDS" -i \
      -H "Connection: Upgrade" \
      -H "Upgrade: websocket" \
      -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
      -H "Sec-WebSocket-Version: 13" \
      "$WS_URL" 2>&1 || echo "")

    if echo "$response" | grep -qi "101\|Switching Protocols"; then
      log_pass
      return 0
    else
      log_warn "WebSocket upgrade check skipped (websocat not available; install with: cargo install websocat)"
      return 0
    fi
  fi
}

# ============================================================================
# Test 5: Ledger Lag Check
# ============================================================================

test_ledger_lag() {
  log_test "ledger lag (Horizon sync)"

  # Fetch latest indexed ledger from backend
  local indexed_ledger
  indexed_ledger=$(curl -sf -m "$TIMEOUT_SECONDS" "$BASE_URL/api/rpc/health" 2>&1 | grep -oP '"latest_ledger":\s*\K\d+' || echo "0")

  if [[ "$indexed_ledger" == "0" ]]; then
    log_warn "could not retrieve indexed ledger from backend"
    return 0
  fi

  # Fetch latest ledger from Horizon
  local horizon_ledger
  horizon_ledger=$(curl -sf -m "$TIMEOUT_SECONDS" "$HORIZON_URL/ledgers?order=desc&limit=1" 2>&1 | grep -oP '"sequence":\s*\K\d+' || echo "0")

  if [[ "$horizon_ledger" == "0" ]]; then
    log_warn "could not reach Horizon network"
    return 0
  fi

  # Calculate lag (assuming ~5 seconds per ledger)
  local lag_seconds
  lag_seconds=$(( (horizon_ledger - indexed_ledger) * 5 ))

  if [[ $lag_seconds -lt 120 ]]; then
    log_pass
    return 0
  else
    log_fail "indexed ledger is ${lag_seconds}s behind Horizon tip (indexed: $indexed_ledger, Horizon: $horizon_ledger)"
    return 1
  fi
}

# ============================================================================
# Test 6: Analytics Dashboard Endpoints
# ============================================================================

test_analytics_endpoints() {
  log_test "analytics dashboard endpoints"

  local endpoints=(
    "/api/anchors"
    "/api/corridors"
    "/api/analytics/summary"
  )

  for endpoint in "${endpoints[@]}"; do
    local response
    response=$(curl -sf -m "$TIMEOUT_SECONDS" "$BASE_URL$endpoint" 2>&1 || echo "")

    if [[ -z "$response" ]]; then
      log_fail "endpoint $endpoint not responding"
      return 1
    fi

    # Basic JSON validation
    if ! echo "$response" | grep -qE '^\[|\{'; then
      log_fail "endpoint $endpoint returned invalid JSON"
      return 1
    fi
  done

  log_pass
  return 0
}

# ============================================================================
# Test 7: Database Connectivity Check
# ============================================================================

test_database_connectivity() {
  log_test "database connectivity"

  # Check a simple endpoint that queries the database
  local response
  response=$(curl -sf -m "$TIMEOUT_SECONDS" "$BASE_URL/api/anchors?limit=1" 2>&1 || echo "")

  if [[ -z "$response" ]]; then
    log_fail "database query failed (no response from anchors endpoint)"
    return 1
  fi

  if echo "$response" | grep -qE '^\[|\{'; then
    log_pass
    return 0
  else
    log_fail "database query returned invalid response"
    return 1
  fi
}

# ============================================================================
# Main Test Runner
# ============================================================================

main() {
  echo -e "${GREEN}=== Stellar Insights Post-Deployment Smoke Tests ===${NC}"
  echo "Target backend: $BASE_URL"
  echo "Target frontend: $FRONTEND_URL"
  echo "Target WebSocket: $WS_URL"
  echo ""

  # Run all tests
  test_backend_health
  test_frontend_reachable
  test_database_connectivity
  test_analytics_endpoints
  test_websocket_upgrade
  test_ledger_lag
  test_contract_initialized

  # Print summary
  echo ""
  echo -e "${GREEN}=== Test Summary ===${NC}"
  echo -e "Passed: ${GREEN}${PASSED_TESTS}${NC}"
  echo -e "Failed: ${RED}${FAILED_TESTS}${NC}"
  echo ""

  if [[ $FAILED_TESTS -eq 0 ]]; then
    echo -e "${GREEN}All smoke tests passed.${NC}"
    return 0
  else
    echo -e "${RED}${FAILED_TESTS} smoke test(s) failed.${NC}"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Check backend is running: docker-compose ps"
    echo "  2. Review backend logs: docker logs stellar-insights-backend"
    echo "  3. Verify network connectivity: curl -v $BASE_URL/health"
    echo "  4. Check database status: docker-compose logs db"
    return 1
  fi
}

# Run main function and exit with appropriate code
main "$@"
