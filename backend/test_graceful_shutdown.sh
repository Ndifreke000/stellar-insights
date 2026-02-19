#!/bin/bash
# Test script for graceful shutdown behavior

set -e

echo "=== Graceful Shutdown Test Script ==="
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test 1: Basic shutdown with SIGTERM
test_sigterm() {
    echo -e "${YELLOW}Test 1: SIGTERM shutdown${NC}"
    echo "Starting server..."
    
    cargo run &
    SERVER_PID=$!
    
    echo "Server PID: $SERVER_PID"
    echo "Waiting 5 seconds for server to start..."
    sleep 5
    
    echo "Sending SIGTERM..."
    kill -TERM $SERVER_PID
    
    echo "Waiting for graceful shutdown..."
    wait $SERVER_PID
    EXIT_CODE=$?
    
    if [ $EXIT_CODE -eq 0 ]; then
        echo -e "${GREEN}✓ Test 1 PASSED: Server shutdown gracefully${NC}"
    else
        echo -e "${RED}✗ Test 1 FAILED: Server exit code: $EXIT_CODE${NC}"
    fi
    echo ""
}

# Test 2: Shutdown with in-flight request
test_with_request() {
    echo -e "${YELLOW}Test 2: Shutdown with in-flight request${NC}"
    echo "Starting server..."
    
    cargo run &
    SERVER_PID=$!
    
    echo "Server PID: $SERVER_PID"
    echo "Waiting 5 seconds for server to start..."
    sleep 5
    
    echo "Sending request in background..."
    curl -s http://localhost:8080/health &
    CURL_PID=$!
    
    sleep 1
    echo "Sending SIGTERM while request is in flight..."
    kill -TERM $SERVER_PID
    
    echo "Waiting for request to complete..."
    wait $CURL_PID
    CURL_EXIT=$?
    
    echo "Waiting for server shutdown..."
    wait $SERVER_PID
    SERVER_EXIT=$?
    
    if [ $CURL_EXIT -eq 0 ] && [ $SERVER_EXIT -eq 0 ]; then
        echo -e "${GREEN}✓ Test 2 PASSED: Request completed and server shutdown gracefully${NC}"
    else
        echo -e "${RED}✗ Test 2 FAILED: Curl exit: $CURL_EXIT, Server exit: $SERVER_EXIT${NC}"
    fi
    echo ""
}

# Test 3: Ctrl+C simulation (SIGINT)
test_sigint() {
    echo -e "${YELLOW}Test 3: SIGINT (Ctrl+C) shutdown${NC}"
    echo "Starting server..."
    
    cargo run &
    SERVER_PID=$!
    
    echo "Server PID: $SERVER_PID"
    echo "Waiting 5 seconds for server to start..."
    sleep 5
    
    echo "Sending SIGINT (Ctrl+C)..."
    kill -INT $SERVER_PID
    
    echo "Waiting for graceful shutdown..."
    wait $SERVER_PID
    EXIT_CODE=$?
    
    if [ $EXIT_CODE -eq 0 ]; then
        echo -e "${GREEN}✓ Test 3 PASSED: Server shutdown gracefully on SIGINT${NC}"
    else
        echo -e "${RED}✗ Test 3 FAILED: Server exit code: $EXIT_CODE${NC}"
    fi
    echo ""
}

# Test 4: Timeout behavior
test_timeout() {
    echo -e "${YELLOW}Test 4: Shutdown timeout behavior${NC}"
    echo "Setting very short timeout..."
    
    export SHUTDOWN_GRACEFUL_TIMEOUT=1
    export SHUTDOWN_BACKGROUND_TIMEOUT=1
    export SHUTDOWN_DB_TIMEOUT=1
    
    echo "Starting server with short timeouts..."
    cargo run &
    SERVER_PID=$!
    
    echo "Server PID: $SERVER_PID"
    echo "Waiting 5 seconds for server to start..."
    sleep 5
    
    echo "Sending SIGTERM..."
    kill -TERM $SERVER_PID
    
    START_TIME=$(date +%s)
    wait $SERVER_PID
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    
    echo "Shutdown took ${DURATION} seconds"
    
    if [ $DURATION -le 5 ]; then
        echo -e "${GREEN}✓ Test 4 PASSED: Shutdown completed within timeout${NC}"
    else
        echo -e "${RED}✗ Test 4 FAILED: Shutdown took too long: ${DURATION}s${NC}"
    fi
    
    unset SHUTDOWN_GRACEFUL_TIMEOUT
    unset SHUTDOWN_BACKGROUND_TIMEOUT
    unset SHUTDOWN_DB_TIMEOUT
    echo ""
}

# Cleanup function
cleanup() {
    echo "Cleaning up any remaining processes..."
    pkill -f "cargo run" || true
    pkill -f "backend" || true
}

# Trap to ensure cleanup on script exit
trap cleanup EXIT

# Main execution
echo "This script will test graceful shutdown behavior"
echo "Make sure the backend is built: cargo build"
echo ""

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found. Please install Rust.${NC}"
    exit 1
fi

# Run tests
echo "Running tests..."
echo ""

test_sigterm
test_sigint
test_with_request
test_timeout

echo "=== Test Summary ==="
echo "All tests completed. Check output above for results."
echo ""
echo "To manually test:"
echo "1. Run: cargo run"
echo "2. Press Ctrl+C"
echo "3. Observe the shutdown logs"
