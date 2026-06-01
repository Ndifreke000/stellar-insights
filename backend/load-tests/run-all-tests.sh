#!/bin/bash

# =============================================================================
# Load Test Runner Script
# =============================================================================
# This script runs all load tests sequentially and generates a comprehensive
# report of the results.
# =============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BASE_URL="${BASE_URL:-http://localhost:8080}"
RESULTS_DIR="./results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Create results directory
mkdir -p "$RESULTS_DIR"

echo -e "${BLUE}==============================================================================${NC}"
echo -e "${BLUE}STELLAR INSIGHTS - LOAD TEST SUITE${NC}"
echo -e "${BLUE}==============================================================================${NC}"
echo ""
echo -e "${GREEN}Base URL:${NC} $BASE_URL"
echo -e "${GREEN}Results Directory:${NC} $RESULTS_DIR"
echo -e "${GREEN}Timestamp:${NC} $TIMESTAMP"
echo ""

# Check if k6 is installed
if ! command -v k6 &> /dev/null; then
    echo -e "${RED}Error: k6 is not installed${NC}"
    echo "Please install k6: https://k6.io/docs/getting-started/installation/"
    exit 1
fi

# Check if server is running
echo -e "${YELLOW}Checking server health...${NC}"
if ! curl -s -f "$BASE_URL/health" > /dev/null; then
    echo -e "${RED}Error: Server is not responding at $BASE_URL${NC}"
    echo "Please start the server before running load tests."
    exit 1
fi
echo -e "${GREEN}✓ Server is healthy${NC}"
echo ""

# Function to run a test
run_test() {
    local test_name=$1
    local test_file=$2
    local output_file="$RESULTS_DIR/${test_name}_${TIMESTAMP}.json"
    
    echo -e "${BLUE}==============================================================================${NC}"
    echo -e "${BLUE}Running: $test_name${NC}"
    echo -e "${BLUE}==============================================================================${NC}"
    
    if k6 run --out json="$output_file" "$test_file"; then
        echo -e "${GREEN}✓ $test_name completed successfully${NC}"
        echo -e "${GREEN}Results saved to: $output_file${NC}"
        return 0
    else
        echo -e "${RED}✗ $test_name failed${NC}"
        return 1
    fi
    
    echo ""
}

# Track test results
declare -a test_results
declare -a test_names

# Run tests
echo -e "${YELLOW}Starting load tests...${NC}"
echo ""

# 1. Corridors Load Test
if run_test "corridors-load-test" "corridors-load-test.js"; then
    test_results+=("PASS")
else
    test_results+=("FAIL")
fi
test_names+=("Corridors Load Test")
sleep 5

# 2. Anchors Load Test
if run_test "anchors-load-test" "anchors-load-test.js"; then
    test_results+=("PASS")
else
    test_results+=("FAIL")
fi
test_names+=("Anchors Load Test")
sleep 5

# 3. RPC Endpoints Load Test
if run_test "rpc-endpoints-load-test" "rpc-endpoints-load-test.js"; then
    test_results+=("PASS")
else
    test_results+=("FAIL")
fi
test_names+=("RPC Endpoints Load Test")
sleep 5

# 4. Full Suite Load Test
if run_test "full-suite-load-test" "full-suite-load-test.js"; then
    test_results+=("PASS")
else
    test_results+=("FAIL")
fi
test_names+=("Full Suite Load Test")
sleep 5

# Optional: Stress Test (commented out by default - takes ~26 minutes)
# echo -e "${YELLOW}Stress test is optional and takes ~26 minutes.${NC}"
# read -p "Run stress test? (y/N): " -n 1 -r
# echo
# if [[ $REPLY =~ ^[Yy]$ ]]; then
#     if run_test "stress-test" "stress-test.js"; then
#         test_results+=("PASS")
#     else
#         test_results+=("FAIL")
#     fi
#     test_names+=("Stress Test")
#     sleep 5
# fi

# Optional: Spike Test
# echo -e "${YELLOW}Spike test is optional and tests sudden traffic bursts.${NC}"
# read -p "Run spike test? (y/N): " -n 1 -r
# echo
# if [[ $REPLY =~ ^[Yy]$ ]]; then
#     if run_test "spike-test" "spike-test.js"; then
#         test_results+=("PASS")
#     else
#         test_results+=("FAIL")
#     fi
#     test_names+=("Spike Test")
# fi

# Generate summary report
echo ""
echo -e "${BLUE}==============================================================================${NC}"
echo -e "${BLUE}TEST SUMMARY${NC}"
echo -e "${BLUE}==============================================================================${NC}"
echo ""

passed=0
failed=0

for i in "${!test_names[@]}"; do
    test_name="${test_names[$i]}"
    result="${test_results[$i]}"
    
    if [ "$result" = "PASS" ]; then
        echo -e "${GREEN}✓${NC} $test_name: ${GREEN}PASSED${NC}"
        ((passed++))
    else
        echo -e "${RED}✗${NC} $test_name: ${RED}FAILED${NC}"
        ((failed++))
    fi
done

echo ""
echo -e "${BLUE}Total Tests:${NC} $((passed + failed))"
echo -e "${GREEN}Passed:${NC} $passed"
echo -e "${RED}Failed:${NC} $failed"
echo ""

# Generate summary file
summary_file="$RESULTS_DIR/summary_${TIMESTAMP}.txt"
{
    echo "STELLAR INSIGHTS - LOAD TEST SUMMARY"
    echo "===================================="
    echo ""
    echo "Date: $(date)"
    echo "Base URL: $BASE_URL"
    echo ""
    echo "Test Results:"
    echo "-------------"
    for i in "${!test_names[@]}"; do
        echo "${test_names[$i]}: ${test_results[$i]}"
    done
    echo ""
    echo "Total: $((passed + failed))"
    echo "Passed: $passed"
    echo "Failed: $failed"
} > "$summary_file"

echo -e "${GREEN}Summary saved to: $summary_file${NC}"
echo ""

# Exit with appropriate code
if [ $failed -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed. Please review the results.${NC}"
    exit 1
fi
