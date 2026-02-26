#!/bin/bash

# Script to check for sensitive data patterns in logs
# Usage: ./scripts/check_sensitive_logs.sh [log_file_or_directory]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default to checking current output
LOG_SOURCE="${1:-.}"

echo "🔍 Scanning for sensitive data patterns in logs..."
echo ""

ISSUES_FOUND=0

# Function to check pattern
check_pattern() {
    local pattern="$1"
    local description="$2"
    local severity="$3"
    
    if [ "$LOG_SOURCE" = "." ]; then
        # Check running application output
        echo -e "${YELLOW}Checking for $description...${NC}"
        if timeout 5s cargo run 2>&1 | grep -E "$pattern" > /dev/null 2>&1; then
            echo -e "${RED}[$severity] Found $description in logs!${NC}"
            ISSUES_FOUND=$((ISSUES_FOUND + 1))
        else
            echo -e "${GREEN}✓ No $description found${NC}"
        fi
    else
        # Check log files
        echo -e "${YELLOW}Checking for $description...${NC}"
        if grep -rE "$pattern" "$LOG_SOURCE" > /dev/null 2>&1; then
            echo -e "${RED}[$severity] Found $description in logs!${NC}"
            grep -rE "$pattern" "$LOG_SOURCE" | head -5
            ISSUES_FOUND=$((ISSUES_FOUND + 1))
        else
            echo -e "${GREEN}✓ No $description found${NC}"
        fi
    fi
    echo ""
}

# Check for Stellar addresses (56 chars starting with G)
check_pattern "G[A-Z0-9]{55}" "Stellar addresses" "HIGH"

# Check for potential user IDs (8+ digit numbers)
check_pattern "\buser[_-]?[0-9]{8,}\b" "User IDs" "HIGH"

# Check for email addresses
check_pattern "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}" "Email addresses" "MEDIUM"

# Check for IPv4 addresses (not redacted)
check_pattern "\b([0-9]{1,3}\.){3}[0-9]{1,3}\b" "IP addresses (IPv4)" "MEDIUM"

# Check for potential API keys (common patterns)
check_pattern "\b(sk_live_|pk_live_|api_key_)[a-zA-Z0-9]{20,}\b" "API keys" "CRITICAL"

# Check for potential transaction hashes (64 hex chars)
check_pattern "\b[a-f0-9]{64}\b" "Transaction hashes" "MEDIUM"

# Check for potential JWT tokens
check_pattern "eyJ[a-zA-Z0-9_-]{20,}\.[a-zA-Z0-9_-]{20,}\.[a-zA-Z0-9_-]{20,}" "JWT tokens" "CRITICAL"

# Check for credit card patterns (just in case)
check_pattern "\b[0-9]{4}[- ]?[0-9]{4}[- ]?[0-9]{4}[- ]?[0-9]{4}\b" "Credit card numbers" "CRITICAL"

# Check for password-related logging
check_pattern "password[\"']?\s*[:=]\s*[\"']?[^\"'\s]{8,}" "Passwords" "CRITICAL"

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $ISSUES_FOUND -eq 0 ]; then
    echo -e "${GREEN}✓ No sensitive data patterns found!${NC}"
    echo -e "${GREEN}✓ Logs appear to be properly redacted.${NC}"
    exit 0
else
    echo -e "${RED}✗ Found $ISSUES_FOUND potential issues!${NC}"
    echo -e "${YELLOW}⚠ Please review and apply appropriate redaction.${NC}"
    exit 1
fi
