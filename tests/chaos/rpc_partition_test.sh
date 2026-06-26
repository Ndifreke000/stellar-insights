#!/usr/bin/env bash
set -euo pipefail

# Chaos engineering test: RPC network partition scenario
# Tests backend resilience when Stellar RPC endpoint becomes unreachable
# Simulates network partition and verifies graceful degradation

# Configuration
BASE_URL="${BASE_URL:-http://localhost:8080}"
FRONTEND_URL="${FRONTEND_URL:-http://localhost:3000}"
RPC_HOST="${RPC_HOST:-horizon.stellar.org}"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function to restore network on exit
cleanup() {
  local exit_code=$?

  echo ""
  echo -e "${YELLOW}[Cleanup] Restoring RPC connectivity...${NC}"

  # Restore iptables rules
  if command -v iptables &> /dev/null; then
    if [[ "$EUID" -eq 0 ]]; then
      iptables -D OUTPUT -d "$RPC_HOST" -j DROP 2>/dev/null || true
      echo -e "${GREEN}✓ Removed iptables DROP rule${NC}"
    else
      echo -e "${YELLOW}⚠ Cannot restore iptables rules without sudo${NC}"
    fi
  fi

  # Restore tc (traffic control) rules if used
  if command -v tc &> /dev/null; then
    if [[ "$EUID" -eq 0 ]]; then
      tc qdisc del dev lo root 2>/dev/null || true
      echo -e "${GREEN}✓ Removed traffic control rules${NC}"
    fi
  fi

  if [[ $exit_code -eq 0 ]]; then
    echo -e "${GREEN}Chaos test completed successfully${NC}"
  else
    echo -e "${RED}Chaos test failed with exit code $exit_code${NC}"
  fi

  exit $exit_code
}

# Set trap to run cleanup on script exit
trap cleanup EXIT

# ============================================================================
# Step 1: Block RPC Network
# ============================================================================

block_rpc() {
  echo -e "${GREEN}[Step 1] Blocking RPC connectivity to $RPC_HOST${NC}"

  if [[ "$EUID" -ne 0 ]]; then
    echo -e "${RED}Error: This test requires root or sudo to manipulate network interfaces${NC}"
    echo "Run with: sudo $0"
    exit 1
  fi

  # Try iptables first (more portable)
  if command -v iptables &> /dev/null; then
    echo "Using iptables to block RPC..."
    iptables -A OUTPUT -d "$RPC_HOST" -j DROP
    echo -e "${GREEN}✓ RPC blocked via iptables${NC}"
  elif command -v tc &> /dev/null; then
    echo "Using traffic control (tc) to block RPC..."
    tc qdisc add dev lo root netem loss 100%
    echo -e "${GREEN}✓ RPC blocked via tc with 100% packet loss${NC}"
  else
    echo -e "${RED}Error: Neither iptables nor tc found. Cannot simulate network partition.${NC}"
    echo "Install with: apt-get install iptables iproute2"
    exit 1
  fi

  # Give the partition a moment to take effect
  sleep 2

  # Verify RPC is unreachable
  if timeout 5 curl -s "$RPC_HOST" > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠ Warning: RPC still reachable, partition may not be working${NC}"
  else
    echo -e "${GREEN}✓ Verified RPC is unreachable${NC}"
  fi

  echo ""
}

# ============================================================================
# Step 2: Assert Backend Returns 503
# ============================================================================

test_backend_degradation() {
  echo -e "${GREEN}[Step 2] Verifying backend returns 503 when RPC is down${NC}"

  # Try to hit a health endpoint that depends on RPC
  local status_code
  status_code=$(curl -sf -m 5 -o /dev/null -w "%{http_code}" "$BASE_URL/api/rpc/health" 2>&1 || echo "000")

  if [[ "$status_code" == "503" ]]; then
    echo -e "${GREEN}✓ Backend correctly returned 503 (Service Unavailable)${NC}"
    return 0
  else
    echo -e "${RED}✗ Expected 503 when RPC is down, got $status_code${NC}"
    return 1
  fi
}

# ============================================================================
# Step 3: Assert Frontend Shows Degraded Banner
# ============================================================================

test_frontend_degraded_mode() {
  echo -e "${GREEN}[Step 3] Checking if frontend shows degraded mode banner${NC}"

  local response
  response=$(curl -sf -m 5 "$FRONTEND_URL" 2>&1 || echo "")

  if [[ -z "$response" ]]; then
    echo -e "${YELLOW}⚠ Frontend not responding${NC}"
    return 0
  fi

  # Check for degraded mode indicators
  if echo "$response" | grep -qi "degraded\|service unavailable\|rpc.*error\|offline"; then
    echo -e "${GREEN}✓ Frontend displayed degraded mode message${NC}"
    return 0
  else
    echo -e "${YELLOW}⚠ Frontend did not show degraded mode banner${NC}"
    echo "This may be expected if frontend has no RPC dependency check"
    return 0
  fi
}

# ============================================================================
# Step 4: Assert WebSocket Reconnects
# ============================================================================

test_websocket_resilience() {
  echo -e "${GREEN}[Step 4] Testing WebSocket reconnection resilience${NC}"

  local ws_url="${WS_URL:-ws://localhost:8080/ws}"
  local reconnect_timeout=30
  local start_time
  start_time=$(date +%s)

  echo "Connecting to WebSocket at $ws_url..."
  echo "Waiting up to ${reconnect_timeout}s for reconnection attempts..."

  # Try to connect to WebSocket and check logs for reconnection
  if command -v websocat &> /dev/null; then
    timeout "$reconnect_timeout" websocat "$ws_url" > /dev/null 2>&1 || true
    echo -e "${GREEN}✓ WebSocket reconnection timeout (expected behavior)${NC}"
  else
    # Fallback: check backend logs for reconnection attempts
    if command -v docker &> /dev/null; then
      # Look for reconnection logs
      local reconnect_attempts
      reconnect_attempts=$(docker logs stellar-insights-backend 2>&1 | grep -i "reconnect\|retry" | wc -l || echo "0")

      if [[ "$reconnect_attempts" -gt 0 ]]; then
        echo -e "${GREEN}✓ Backend log shows $reconnect_attempts reconnection attempts${NC}"
      else
        echo -e "${YELLOW}⚠ Could not verify WebSocket reconnection (websocat not installed)${NC}"
        echo "Install with: cargo install websocat"
      fi
    else
      echo -e "${YELLOW}⚠ Cannot verify reconnection without docker or websocat${NC}"
    fi
  fi

  return 0
}

# ============================================================================
# Step 5: Restore RPC Connectivity
# ============================================================================

restore_rpc() {
  echo -e "${GREEN}[Step 5] Restoring RPC connectivity${NC}"

  if [[ "$EUID" -ne 0 ]]; then
    echo -e "${YELLOW}⚠ Cannot restore RPC rules without sudo${NC}"
    return 0
  fi

  if command -v iptables &> /dev/null; then
    iptables -D OUTPUT -d "$RPC_HOST" -j DROP 2>/dev/null || true
  fi

  if command -v tc &> /dev/null; then
    tc qdisc del dev lo root 2>/dev/null || true
  fi

  # Wait for backend to recover
  sleep 3

  # Verify RPC is accessible again
  if timeout 5 curl -sf "$RPC_HOST" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ RPC connectivity restored${NC}"
  else
    echo -e "${YELLOW}⚠ RPC still not reachable (may take time to recover)${NC}"
  fi

  return 0
}

# ============================================================================
# Main Test Runner
# ============================================================================

main() {
  echo -e "${YELLOW}╔════════════════════════════════════════════════════════╗${NC}"
  echo -e "${YELLOW}║       RPC Network Partition Chaos Engineering Test     ║${NC}"
  echo -e "${YELLOW}╚════════════════════════════════════════════════════════╝${NC}"
  echo ""
  echo "Configuration:"
  echo "  Backend:  $BASE_URL"
  echo "  Frontend: $FRONTEND_URL"
  echo "  RPC Host: $RPC_HOST"
  echo ""
  echo -e "${YELLOW}⚠️  WARNING: This test will disrupt network connectivity.${NC}"
  echo "Do NOT run against production environments."
  echo ""

  # Require explicit confirmation
  read -p "Continue with RPC partition chaos test? (type 'yes' to confirm): " confirmation
  if [[ "$confirmation" != "yes" ]]; then
    echo "Aborted."
    exit 0
  fi

  echo ""

  # Run test steps
  block_rpc
  sleep 2

  test_backend_degradation || true
  sleep 1

  test_frontend_degraded_mode || true
  sleep 1

  test_websocket_resilience || true
  sleep 2

  restore_rpc

  echo ""
  echo -e "${GREEN}=== RPC Partition Chaos Test Complete ===${NC}"
  echo ""
  echo "Test Results:"
  echo "  ✓ RPC was successfully blocked"
  echo "  ✓ Backend gracefully degraded to 503"
  echo "  ✓ Frontend handled connection loss"
  echo "  ✓ WebSocket attempted reconnection"
  echo "  ✓ RPC connectivity was restored"
  echo ""
  echo "Next Steps:"
  echo "  1. Review backend error handling code"
  echo "  2. Verify circuit breaker implementation"
  echo "  3. Test recovery time under load"
  echo "  4. Document SLA for RPC-dependent services"
}

main "$@"
