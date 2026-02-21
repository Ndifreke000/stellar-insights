#!/bin/bash
# Test script for graceful shutdown functionality

set -e

echo "==================================="
echo "Graceful Shutdown Test Script"
echo "==================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if server is already running
if lsof -Pi :8080 -sTCP:LISTEN -t >/dev/null 2>&1 ; then
    echo -e "${RED}Error: Port 8080 is already in use${NC}"
    echo "Please stop the existing server first"
    exit 1
fi

echo -e "${YELLOW}Step 1: Building the backend...${NC}"
cargo build --release
echo -e "${GREEN}✓ Build complete${NC}"
echo ""

echo -e "${YELLOW}Step 2: Starting the server in background...${NC}"
RUST_LOG=info cargo run --release > /tmp/stellar_backend.log 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID"
echo -e "${GREEN}✓ Server started${NC}"
echo ""

# Wait for server to be ready
echo -e "${YELLOW}Step 3: Waiting for server to be ready...${NC}"
MAX_WAIT=30
WAITED=0
while ! curl -s http://localhost:8080/health > /dev/null 2>&1; do
    sleep 1
    WAITED=$((WAITED + 1))
    if [ $WAITED -ge $MAX_WAIT ]; then
        echo -e "${RED}✗ Server failed to start within ${MAX_WAIT} seconds${NC}"
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
done
echo -e "${GREEN}✓ Server is ready (took ${WAITED}s)${NC}"
echo ""

# Make some test requests
echo -e "${YELLOW}Step 4: Making test requests...${NC}"
for i in {1..5}; do
    curl -s http://localhost:8080/health > /dev/null && echo "  Request $i: OK" || echo "  Request $i: FAILED"
    sleep 0.2
done
echo -e "${GREEN}✓ Test requests complete${NC}"
echo ""

# Send shutdown signal
echo -e "${YELLOW}Step 5: Sending SIGTERM to server (PID: $SERVER_PID)...${NC}"
SHUTDOWN_START=$(date +%s)
kill -TERM $SERVER_PID

# Wait for graceful shutdown
echo "Waiting for graceful shutdown..."
wait $SERVER_PID 2>/dev/null || true
SHUTDOWN_END=$(date +%s)
SHUTDOWN_DURATION=$((SHUTDOWN_END - SHUTDOWN_START))

echo -e "${GREEN}✓ Server shutdown complete (took ${SHUTDOWN_DURATION}s)${NC}"
echo ""

# Analyze logs
echo -e "${YELLOW}Step 6: Analyzing shutdown logs...${NC}"
echo ""
echo "=== Shutdown Log Analysis ==="

# Check for key shutdown messages
if grep -q "Shutdown signal received" /tmp/stellar_backend.log; then
    echo -e "${GREEN}✓ Shutdown signal detected${NC}"
else
    echo -e "${RED}✗ Shutdown signal not detected${NC}"
fi

if grep -q "Server stopped accepting new connections" /tmp/stellar_backend.log; then
    echo -e "${GREEN}✓ Server stopped accepting connections${NC}"
else
    echo -e "${RED}✗ Server did not stop accepting connections${NC}"
fi

if grep -q "Shutting down background tasks" /tmp/stellar_backend.log; then
    echo -e "${GREEN}✓ Background tasks shutdown initiated${NC}"
else
    echo -e "${RED}✗ Background tasks shutdown not initiated${NC}"
fi

if grep -q "Closing WebSocket connections" /tmp/stellar_backend.log; then
    echo -e "${GREEN}✓ WebSocket connections closed${NC}"
else
    echo -e "${RED}✗ WebSocket connections not closed${NC}"
fi

if grep -q "Flushing cache" /tmp/stellar_backend.log; then
    echo -e "${GREEN}✓ Cache flushed${NC}"
else
    echo -e "${RED}✗ Cache not flushed${NC}"
fi

if grep -q "Closing database connections" /tmp/stellar_backend.log; then
    echo -e "${GREEN}✓ Database connections closed${NC}"
else
    echo -e "${RED}✗ Database connections not closed${NC}"
fi

if grep -q "Graceful shutdown complete" /tmp/stellar_backend.log; then
    echo -e "${GREEN}✓ Graceful shutdown completed successfully${NC}"
else
    echo -e "${RED}✗ Graceful shutdown did not complete${NC}"
fi

echo ""
echo "=== Recent Shutdown Logs ==="
tail -n 30 /tmp/stellar_backend.log | grep -E "(shutdown|Shutdown|SHUTDOWN|Step [0-9])" || echo "No shutdown logs found"

echo ""
echo "==================================="
echo -e "${GREEN}Test Complete!${NC}"
echo "==================================="
echo ""
echo "Full logs available at: /tmp/stellar_backend.log"
echo "Shutdown duration: ${SHUTDOWN_DURATION} seconds"
echo ""

# Cleanup
rm -f /tmp/stellar_backend.log

exit 0
