#!/bin/bash

echo "=========================================="
echo "CLOSING ALL LAZY ISSUES"
echo "=========================================="
echo ""

# Close all existing issues
echo "Fetching all open issues..."
ISSUES=$(gh issue list --state open --limit 100 --json number,title | jq -r '.[] | select(.title | startswith("[Backend]") or startswith("[SDK]") or startswith("[Mobile]")) | .number')

COUNT=$(echo "$ISSUES" | wc -l)
echo "Found $COUNT issues to close"
echo ""

for issue in $ISSUES; do
    echo "Closing issue #$issue..."
    gh issue close $issue --comment "Closing lazy issue - will be replaced with properly detailed version" 2>/dev/null
    sleep 0.5
done

echo ""
echo "✅ Closed all lazy issues"
echo ""
echo "=========================================="
echo "NOW CREATE PROPER DETAILED ISSUES"
echo "=========================================="
echo ""
echo "Run: python3 scripts/create_all_70_detailed.py"
echo ""
