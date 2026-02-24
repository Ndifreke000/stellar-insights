#!/usr/bin/env bash
# Script to identify undocumented public functions in the Stellar Insights backend

set -euo pipefail

echo "=== Stellar Insights Documentation Coverage Report ==="
echo ""

# Count total public functions
TOTAL_PUB_FN=$(rg "^pub fn" backend/src/ --count-matches | awk -F: '{sum += $2} END {print sum}')
TOTAL_PUB_ASYNC_FN=$(rg "^pub async fn" backend/src/ --count-matches | awk -F: '{sum += $2} END {print sum}')
TOTAL_PUB=$(( TOTAL_PUB_FN + TOTAL_PUB_ASYNC_FN ))

# Count documented functions (lines starting with ///)
DOCUMENTED=$(rg "^/// " backend/src/ --count-matches | awk -F: '{sum += $2} END {print sum}')

# Calculate coverage
COVERAGE=$(awk "BEGIN {printf \"%.1f\", ($DOCUMENTED / $TOTAL_PUB) * 100}")

echo "Total public functions: $TOTAL_PUB"
echo "Documented: $DOCUMENTED"
echo "Coverage: $COVERAGE%"
echo ""

echo "=== Files with Most Undocumented Functions ==="
echo ""

# Find files with undocumented public functions
for file in $(rg "^pub (async )?fn" backend/src/ --files-with-matches); do
    PUB_COUNT=$(rg "^pub (async )?fn" "$file" --count-matches || echo "0")
    DOC_COUNT=$(rg "^/// " "$file" --count-matches || echo "0")
    UNDOC=$(( PUB_COUNT - DOC_COUNT ))
    
    if [ "$UNDOC" -gt 0 ]; then
        echo "$file: $UNDOC undocumented ($PUB_COUNT total)"
    fi
done | sort -t: -k2 -rn | head -20

echo ""
echo "=== Priority Files to Document ==="
echo ""
echo "1. backend/src/database.rs - Core database operations"
echo "2. backend/src/api/corridors_cached.rs - Main API endpoints"
echo "3. backend/src/rpc/stellar.rs - RPC client"
echo "4. backend/src/services/*.rs - Business logic"
echo "5. backend/src/api/*.rs - API handlers"
echo ""

echo "=== Next Steps ==="
echo ""
echo "1. Review backend/docs/DOCUMENTATION_STANDARDS.md"
echo "2. Run: cargo doc --no-deps --open"
echo "3. Document functions in priority order"
echo "4. Add to CI: cargo rustdoc -- -D missing-docs"
echo ""
