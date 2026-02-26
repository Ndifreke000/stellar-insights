#!/bin/bash
# Analyze current codebase state before refactoring

cd "$(dirname "$0")/../.."

echo "📊 Stellar Insights - Codebase Analysis"
echo "========================================"
echo ""

# File counts
echo "1. FILE COUNTS"
echo "--------------"
TOTAL_RS=$(find backend/src -name "*.rs" | wc -l)
TOTAL_TS=$(find frontend/src -name "*.ts" -o -name "*.tsx" | wc -l)
BACKEND_MD=$(find backend -maxdepth 1 -name "*.md" ! -name "README.md" | wc -l)
FRONTEND_MD=$(find frontend -maxdepth 1 -name "*.md" ! -name "README.md" | wc -l)
TOTAL_MD=$(find . -name "*.md" | wc -l)

echo "Backend Rust files: $TOTAL_RS"
echo "Frontend TS/TSX files: $TOTAL_TS"
echo "Backend .md in root: $BACKEND_MD"
echo "Frontend .md in root: $FRONTEND_MD"
echo "Total .md files: $TOTAL_MD"

# Large files
echo ""
echo "2. LARGEST FILES (Top 10)"
echo "-------------------------"
find backend/src -name "*.rs" -exec wc -l {} + | sort -rn | head -11 | tail -10 | awk '{printf "%-50s %6d lines\n", $2, $1}'

# Service files
echo ""
echo "3. SERVICE FILES"
echo "----------------"
SERVICE_COUNT=$(find backend/src/services -name "*.rs" 2>/dev/null | wc -l)
echo "Total service files: $SERVICE_COUNT"
echo ""
echo "Services:"
find backend/src/services -name "*.rs" 2>/dev/null | xargs wc -l | sort -rn | head -11 | tail -10 | awk '{printf "  %-45s %5d lines\n", $2, $1}'

# TODO comments
echo ""
echo "4. TECHNICAL DEBT"
echo "-----------------"
TODO_RUST=$(rg -i "TODO|FIXME|XXX|HACK" backend/src --type rust 2>/dev/null | wc -l || echo "0")
TODO_TS=$(rg -i "TODO|FIXME|XXX|HACK" frontend/src --type typescript 2>/dev/null | wc -l || echo "0")
TODO_TOTAL=$((TODO_RUST + TODO_TS))

echo "TODO comments in Rust: $TODO_RUST"
echo "TODO comments in TypeScript: $TODO_TS"
echo "Total TODO comments: $TODO_TOTAL"

# Unused features
echo ""
echo "5. POTENTIALLY UNUSED FEATURES"
echo "-------------------------------"
[ -f "backend/src/ml.rs" ] && echo "✓ ML stub files exist" || echo "✗ ML stub files removed"
[ -d "frontend/src/app/game" ] && echo "✓ Game folder exists" || echo "✗ Game folder removed"
[ -f "backend/src/services/sep24_proxy.rs" ] && echo "✓ SEP-24 proxy exists" || echo "✗ SEP-24 proxy removed"
[ -f "backend/src/services/sep31_proxy.rs" ] && echo "✓ SEP-31 proxy exists" || echo "✗ SEP-31 proxy removed"
[ -d "backend/src/telegram" ] && echo "✓ Telegram bot exists" || echo "✗ Telegram bot removed"

# Contract directories
echo ""
echo "6. CONTRACT DIRECTORIES"
echo "-----------------------"
find contracts -maxdepth 1 -type d ! -name "contracts" | while read dir; do
    echo "  - $(basename "$dir")"
done

# Dependencies
echo ""
echo "7. DEPENDENCIES"
echo "---------------"
RUST_DEPS=$(grep -c "^[a-z]" backend/Cargo.toml 2>/dev/null || echo "0")
NPM_DEPS=$(grep -c '"' frontend/package.json 2>/dev/null | awk '{print int($1/2)}')

echo "Rust dependencies: $RUST_DEPS"
echo "NPM dependencies: $NPM_DEPS"

# Code duplication
echo ""
echo "8. POTENTIAL DUPLICATES"
echo "-----------------------"
[ -f "backend/src/api/corridors.rs" ] && [ -f "backend/src/api/corridors_cached.rs" ] && \
    echo "⚠️  Both corridors.rs and corridors_cached.rs exist"

[ -f "backend/src/analytics.rs" ] && [ -f "backend/src/services/analytics.rs" ] && \
    echo "⚠️  Both analytics.rs and services/analytics.rs exist"

# Summary
echo ""
echo "================================"
echo "SUMMARY"
echo "================================"
echo ""
echo "Codebase Size:"
echo "  - $TOTAL_RS Rust files"
echo "  - $TOTAL_TS TypeScript files"
echo "  - $TOTAL_MD Markdown files"
echo ""
echo "Issues Found:"
echo "  - $BACKEND_MD .md files in backend root (should be 0)"
echo "  - $FRONTEND_MD .md files in frontend root (should be 0)"
echo "  - $TODO_TOTAL TODO comments (should be 0)"
echo "  - $SERVICE_COUNT service files (target: 8-10 consolidated modules)"
echo ""
echo "Largest Files:"
find backend/src -name "*.rs" -exec wc -l {} + | sort -rn | head -4 | tail -3 | awk '{printf "  - %s (%d lines)\n", $2, $1}'
echo ""
echo "Estimated Refactoring Impact:"
echo "  - Documentation cleanup: -103 files from roots"
echo "  - Feature removal: -15 to -20 files"
echo "  - Modularization: Better organization, no file >500 lines"
echo "  - Expected size reduction: ~40%"
echo ""
echo "Next Steps:"
echo "  1. Review REFACTORING_PLAN.md"
echo "  2. Create backup branch"
echo "  3. Run Phase 1: ./scripts/refactoring/1-reorganize-backend-docs.sh"
echo ""
