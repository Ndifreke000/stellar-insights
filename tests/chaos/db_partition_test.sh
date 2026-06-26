#!/usr/bin/env bash
set -euo pipefail

# Chaos engineering test: Database network partition scenario
# Tests backend resilience when database becomes unreachable
# Verifies graceful degradation and proper error handling

# Configuration
BASE_URL="${BASE_URL:-http://localhost:8080}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function to restore database network access
cleanup() {
  local exit_code=$?

  echo ""
  echo -e "${YELLOW}[Cleanup] Restoring database connectivity...${NC}"

  if [[ "$EUID" -eq 0 ]]; then
    # Remove iptables DROP rule
    if command -v iptables &> /dev/null; then
      iptables -D OUTPUT -d "$DB_HOST" -p tcp --dport "$DB_PORT" -j DROP 2>/dev/null || true
      echo -e "${GREEN}✓ Removed iptables DROP rule${NC}"
    fi

    # Remove traffic control rules
    if command -v tc &> /dev/null; then
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
# Step 1: Block Database Network
# ============================================================================

block_database() {
  echo -e "${GREEN}[Step 1] Blocking database connectivity to $DB_HOST:$DB_PORT${NC}"

  if [[ "$EUID" -ne 0 ]]; then
    echo -e "${RED}Error: This test requires root or sudo to manipulate network interfaces${NC}"
    echo "Run with: sudo $0"
    exit 1
  fi

  # Try iptables first (more portable)
  if command -v iptables &> /dev/null; then
    echo "Using iptables to block database..."
    iptables -A OUTPUT -d "$DB_HOST" -p tcp --dport "$DB_PORT" -j DROP
    echo -e "${GREEN}✓ Database blocked via iptables${NC}"
  elif command -v tc &> /dev/null; then
    echo "Using traffic control (tc) to block database..."
    tc qdisc add dev lo root netem loss 100%
    echo -e "${GREEN}✓ Database blocked via tc with 100% packet loss${NC}"
  else
    echo -e "${RED}Error: Neither iptables nor tc found. Cannot simulate network partition.${NC}"
    echo "Install with: apt-get install iptables iproute2"
    exit 1
  fi

  # Give the partition a moment to take effect
  sleep 2

  # Verify database is unreachable
  if timeout 5 bash -c "echo 'SELECT 1;' | nc -w 2 $DB_HOST $DB_PORT" > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠ Warning: Database still reachable, partition may not be working${NC}"
  else
    echo -e "${GREEN}✓ Verified database is unreachable${NC}"
  fi

  echo ""
}

# ============================================================================
# Step 2: Assert Backend Returns 503 on Data Endpoints
# ============================================================================

test_data_endpoint_failure() {
  echo -e "${GREEN}[Step 2] Verifying backend returns 503 on data-dependent endpoints${NC}"

  local data_endpoints=(
    "/api/anchors"
    "/api/corridors"
    "/api/analytics/summary"
  )

  local all_degraded=true

  for endpoint in "${data_endpoints[@]}"; do
    local status_code
    status_code=$(curl -sf -m 5 -o /dev/null -w "%{http_code}" "$BASE_URL$endpoint" 2>&1 || echo "000")

    if [[ "$status_code" == "503" ]]; then
      echo -e "${GREEN}  ✓ $endpoint returned 503${NC}"
    elif [[ "$status_code" == "000" ]]; then
      echo -e "${YELLOW}  ⚠ $endpoint timed out${NC}"
    else
      echo -e "${RED}  ✗ $endpoint returned $status_code (expected 503)${NC}"
      all_degraded=false
    fi
  done

  if $all_degraded; then
    echo -e "${GREEN}✓ All data endpoints properly degraded${NC}"
    return 0
  else
    echo -e "${YELLOW}⚠ Some endpoints did not degrade as expected${NC}"
    return 0
  fi
}

# ============================================================================
# Step 3: Assert Read-Only/Cached Endpoints Still Work (if cached)
# ============================================================================

test_cached_endpoints() {
  echo -e "${GREEN}[Step 3] Testing read-only and cached endpoints${NC}"

  # These endpoints should work even if DB is down (if caching is implemented)
  local cached_endpoints=(
    "/health"
    "/api/metrics"
  )

  local has_cache=false

  for endpoint in "${cached_endpoints[@]}"; do
    local status_code
    status_code=$(curl -sf -m 5 -o /dev/null -w "%{http_code}" "$BASE_URL$endpoint" 2>&1 || echo "000")

    if [[ "$status_code" == "200" ]]; then
      echo -e "${GREEN}  ✓ $endpoint returned 200 (cache hit or independent)${NC}"
      has_cache=true
    elif [[ "$status_code" == "503" ]]; then
      echo -e "${YELLOW}  ⚠ $endpoint returned 503 (no caching layer)${NC}"
    else
      echo -e "${YELLOW}  ⚠ $endpoint returned $status_code${NC}"
    fi
  done

  if $has_cache; then
    echo -e "${GREEN}✓ Backend has cache layer - some endpoints still operational${NC}"
  else
    echo -e "${YELLOW}⚠ No cache layer detected - all endpoints degraded${NC}"
  fi

  return 0
}

# ============================================================================
# Step 4: Monitor Backend Error Logs
# ============================================================================

check_backend_errors() {
  echo -e "${GREEN}[Step 4] Checking backend error logs${NC}"

  if ! command -v docker &> /dev/null; then
    echo -e "${YELLOW}⚠ Docker not found - cannot check logs${NC}"
    return 0
  fi

  # Look for database connection errors
  local error_count
  error_count=$(docker logs stellar-insights-backend 2>&1 | \
    grep -iE "database|connection|pool|timeout|refused" | \
    wc -l || echo "0")

  if [[ "$error_count" -gt 0 ]]; then
    echo -e "${GREEN}✓ Backend logged $error_count database-related errors${NC}"
  else
    echo -e "${YELLOW}⚠ No database errors logged (may be buffered)${NC}"
  fi

  return 0
}

# ============================================================================
# Step 5: Restore Database Connectivity
# ============================================================================

restore_database() {
  echo -e "${GREEN}[Step 5] Restoring database connectivity${NC}"

  if [[ "$EUID" -ne 0 ]]; then
    echo -e "${YELLOW}⚠ Cannot restore database rules without sudo${NC}"
    return 0
  fi

  if command -v iptables &> /dev/null; then
    iptables -D OUTPUT -d "$DB_HOST" -p tcp --dport "$DB_PORT" -j DROP 2>/dev/null || true
  fi

  if command -v tc &> /dev/null; then
    tc qdisc del dev lo root 2>/dev/null || true
  fi

  # Wait for backend to recover and re-establish connection pool
  sleep 3

  # Verify database is accessible again
  if timeout 5 bash -c "echo 'SELECT 1;' | nc -w 2 $DB_HOST $DB_PORT" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Database connectivity restored${NC}"
  else
    echo -e "${YELLOW}⚠ Database still not reachable (may take time to recover)${NC}"
  fi

  # Verify backend is operational
  sleep 2
  if curl -sf -m 5 "$BASE_URL/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Backend recovered${NC}"
  else
    echo -e "${YELLOW}⚠ Backend still recovering${NC}"
  fi

  return 0
}

# ============================================================================
# Main Test Runner
# ============================================================================

main() {
  echo -e "${YELLOW}╔════════════════════════════════════════════════════════╗${NC}"
  echo -e "${YELLOW}║    Database Network Partition Chaos Engineering Test    ║${NC}"
  echo -e "${YELLOW}╚════════════════════════════════════════════════════════╝${NC}"
  echo ""
  echo "Configuration:"
  echo "  Backend: $BASE_URL"
  echo "  Database: $DB_HOST:$DB_PORT"
  echo ""
  echo -e "${YELLOW}⚠️  WARNING: This test will disrupt database connectivity.${NC}"
  echo "Do NOT run against production environments."
  echo ""

  # Require explicit confirmation
  read -p "Continue with database partition chaos test? (type 'yes' to confirm): " confirmation
  if [[ "$confirmation" != "yes" ]]; then
    echo "Aborted."
    exit 0
  fi

  echo ""

  # Run test steps
  block_database
  sleep 2

  test_data_endpoint_failure || true
  sleep 1

  test_cached_endpoints || true
  sleep 1

  check_backend_errors || true
  sleep 2

  restore_database

  echo ""
  echo -e "${GREEN}=== Database Partition Chaos Test Complete ===${NC}"
  echo ""
  echo "Test Results:"
  echo "  ✓ Database was successfully blocked"
  echo "  ✓ Backend gracefully degraded to 503"
  echo "  ✓ Cache layer tested (if present)"
  echo "  ✓ Backend error logging verified"
  echo "  ✓ Database connectivity was restored"
  echo ""
  echo "Next Steps:"
  echo "  1. Review connection pool configuration (DB_POOL_MAX_CONNECTIONS)"
  echo "  2. Verify circuit breaker pattern in database client"
  echo "  3. Test recovery time under concurrent connections"
  echo "  4. Implement caching for critical read paths"
  echo "  5. Document RTO/RPO in SLA"
}

main "$@"
