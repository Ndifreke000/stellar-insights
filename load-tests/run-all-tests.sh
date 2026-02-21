#!/bin/bash

# Stellar Insights Backend - Load Test Runner
# Runs all k6 test suites sequentially and generates reports

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BASE_URL="${BASE_URL:-http://localhost:8080}"
RESULTS_DIR="load-test-results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Stellar Insights Load Test Suite${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Target: ${GREEN}${BASE_URL}${NC}"
echo -e "Timestamp: ${TIMESTAMP}"
echo ""

# Create results directory
mkdir -p "${RESULTS_DIR}"

# Check if k6 is installed
if ! command -v k6 &> /dev/null; then
    echo -e "${RED}Error: k6 is not installed${NC}"
    echo "Please install k6: https://k6.io/docs/getting-started/installation/"
    exit 1
fi

# Check if server is accessible
echo -e "${YELLOW}Checking server health...${NC}"
if ! curl -s -f "${BASE_URL}/health" > /dev/null; then
    echo -e "${RED}Error: Server not accessible at ${BASE_URL}${NC}"
    echo "Please ensure the backend is running"
    exit 1
fi
echo -e "${GREEN}✓ Server is accessible${NC}"
echo ""

# Function to run a test
run_test() {
    local test_name=$1
    local test_file=$2
    local output_file="${RESULTS_DIR}/${test_name}_${TIMESTAMP}"
    
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}Running: ${test_name}${NC}"
    echo -e "${BLUE}========================================${NC}"
    
    if k6 run \
        -e BASE_URL="${BASE_URL}" \
        --out json="${output_file}.json" \
        "${test_file}"; then
        echo -e "${GREEN}✓ ${test_name} completed successfully${NC}"
        echo ""
        return 0
    else
        echo -e "${RED}✗ ${test_name} failed${NC}"
        echo ""
        return 1
    fi
}

# Track results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Run all tests
tests=(
    "Health Check:load-tests/k6-cache-metrics.js"
    "Corridors:load-tests/k6-corridors.js"
    "Anchors:load-tests/k6-anchors.js"
    "RPC:load-tests/k6-rpc.js"
    "Liquidity Pools:load-tests/k6-liquidity-pools.js"
    "Fee Bumps:load-tests/k6-fee-bumps.js"
    "Full Suite:load-tests/k6-full-suite.js"
)

for test in "${tests[@]}"; do
    IFS=':' read -r name file <<< "$test"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if run_test "$name" "$file"; then
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    
    # Wait between tests
    if [ $TOTAL_TESTS -lt ${#tests[@]} ]; then
        echo -e "${YELLOW}Waiting 10 seconds before next test...${NC}"
        sleep 10
    fi
done

# Summary
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Test Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Total Tests: ${TOTAL_TESTS}"
echo -e "${GREEN}Passed: ${PASSED_TESTS}${NC}"
if [ $FAILED_TESTS -gt 0 ]; then
    echo -e "${RED}Failed: ${FAILED_TESTS}${NC}"
else
    echo -e "Failed: ${FAILED_TESTS}"
fi
echo ""
echo -e "Results saved to: ${RESULTS_DIR}/"
echo ""

# Exit with appropriate code
if [ $FAILED_TESTS -gt 0 ]; then
    echo -e "${RED}Some tests failed${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
