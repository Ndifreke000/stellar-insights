#!/bin/bash

# Anchor Detail Analytics Page - Implementation Verification Script
# Issue #51

echo "=================================================="
echo "  Anchor Detail Analytics Page Verification"
echo "  Issue #51 Implementation Check"
echo "=================================================="
echo ""

FRONTEND_DIR="frontend"
PASSED=0
FAILED=0

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

check_file() {
    if [ -f "$1" ]; then
        echo -e "${GREEN}✅ PASS${NC} - File exists: $1"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}❌ FAIL${NC} - File missing: $1"
        ((FAILED++))
        return 1
    fi
}

check_content() {
    if grep -q "$2" "$1" 2>/dev/null; then
        echo -e "${GREEN}✅ PASS${NC} - Found in $1: $3"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}❌ FAIL${NC} - Not found in $1: $3"
        ((FAILED++))
        return 1
    fi
}

echo "📁 Checking Required Files..."
echo "-----------------------------------"
check_file "$FRONTEND_DIR/src/app/anchors/[address]/page.tsx"
check_file "$FRONTEND_DIR/src/components/anchors/AnchorHeader.tsx"
check_file "$FRONTEND_DIR/src/components/anchors/IssuedAssetsTable.tsx"
check_file "$FRONTEND_DIR/src/components/charts/ReliabilityTrend.tsx"
check_file "$FRONTEND_DIR/src/lib/api.ts"
echo ""

echo "🔍 Checking Main Page Implementation..."
echo "-----------------------------------"
PAGE_FILE="$FRONTEND_DIR/src/app/anchors/[address]/page.tsx"
check_content "$PAGE_FILE" "getAnchorDetail" "API call function"
check_content "$PAGE_FILE" "AnchorHeader" "AnchorHeader component import"
check_content "$PAGE_FILE" "IssuedAssetsTable" "IssuedAssetsTable component import"
check_content "$PAGE_FILE" "ReliabilityTrend" "ReliabilityTrend component import"
check_content "$PAGE_FILE" "use client" "Client component directive"
check_content "$PAGE_FILE" "Suspense" "Suspense boundary"
check_content "$PAGE_FILE" "params: Promise" "Async params handling"
check_content "$PAGE_FILE" "G\]\[A-Z0-9\]" "Address validation regex"
echo ""

echo "🎨 Checking AnchorHeader Component..."
echo "-----------------------------------"
HEADER_FILE="$FRONTEND_DIR/src/components/anchors/AnchorHeader.tsx"
check_content "$HEADER_FILE" "reliability_score" "Reliability score display"
check_content "$HEADER_FILE" "stellar_account" "Stellar account display"
check_content "$HEADER_FILE" "status" "Health status indicator"
check_content "$HEADER_FILE" "Copy" "Copy address button"
check_content "$HEADER_FILE" "stellar.expert" "Stellar Explorer link"
echo ""

echo "📊 Checking IssuedAssetsTable Component..."
echo "-----------------------------------"
TABLE_FILE="$FRONTEND_DIR/src/components/anchors/IssuedAssetsTable.tsx"
check_content "$TABLE_FILE" "IssuedAsset" "Asset type import"
check_content "$TABLE_FILE" "asset_code" "Asset code display"
check_content "$TABLE_FILE" "volume_24h_usd" "24h volume display"
check_content "$TABLE_FILE" "success_rate" "Success rate display"
check_content "$TABLE_FILE" "failure_rate" "Failure rate display"
check_content "$TABLE_FILE" "usePagination" "Pagination hook"
echo ""

echo "📈 Checking ReliabilityTrend Chart..."
echo "-----------------------------------"
CHART_FILE="$FRONTEND_DIR/src/components/charts/ReliabilityTrend.tsx"
check_content "$CHART_FILE" "AreaChart" "AreaChart component"
check_content "$CHART_FILE" "ReliabilityDataPoint" "Type definition"
check_content "$CHART_FILE" "'7d' | '30d' | '90d'" "Time window options"
check_content "$CHART_FILE" "CustomTooltip" "Custom tooltip"
check_content "$CHART_FILE" "ResponsiveContainer" "Responsive container"
echo ""

echo "🔌 Checking API Integration..."
echo "-----------------------------------"
API_FILE="$FRONTEND_DIR/src/lib/api.ts"
check_content "$API_FILE" "getAnchorDetail" "getAnchorDetail function"
check_content "$API_FILE" "AnchorDetailData" "AnchorDetailData type"
check_content "$API_FILE" "IssuedAsset" "IssuedAsset type"
check_content "$API_FILE" "ReliabilityDataPoint" "ReliabilityDataPoint type"
check_content "$API_FILE" "/anchors/" "Anchors API endpoint"
echo ""

echo "🏗️  Checking TypeScript Configuration..."
echo "-----------------------------------"
if [ -f "$FRONTEND_DIR/tsconfig.json" ]; then
    check_content "$FRONTEND_DIR/tsconfig.json" '"@/*"' "Path alias configuration"
    check_content "$FRONTEND_DIR/tsconfig.json" '"jsx"' "JSX configuration"
fi
echo ""

echo "📦 Checking Dependencies..."
echo "-----------------------------------"
if [ -f "$FRONTEND_DIR/package.json" ]; then
    check_content "$FRONTEND_DIR/package.json" 'recharts' "Recharts library"
    check_content "$FRONTEND_DIR/package.json" 'lucide-react' "Lucide icons"
    check_content "$FRONTEND_DIR/package.json" 'next' "Next.js framework"
fi
echo ""

echo "=================================================="
echo "  Verification Summary"
echo "=================================================="
echo -e "${GREEN}✅ Passed: $PASSED${NC}"
echo -e "${RED}❌ Failed: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All checks passed! Implementation is complete.${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Some checks failed. Review the output above.${NC}"
    exit 1
fi
