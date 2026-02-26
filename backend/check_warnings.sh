#!/bin/bash
# Script to check for compiler warnings and clippy issues

set -e

echo "🔍 Checking for compiler warnings..."
echo "=================================="

# Build and capture warnings
echo ""
echo "📦 Running cargo build..."
cargo build 2>&1 | tee build.log

# Count warnings
WARNING_COUNT=$(grep -c "warning:" build.log || true)

if [ "$WARNING_COUNT" -eq 0 ]; then
    echo "✅ No compiler warnings found!"
else
    echo "⚠️  Found $WARNING_COUNT compiler warning(s)"
    echo ""
    echo "Warnings:"
    grep "warning:" build.log
fi

echo ""
echo "🔧 Running cargo clippy..."
echo "=================================="

# Run clippy
cargo clippy --all-targets --all-features 2>&1 | tee clippy.log

# Count clippy warnings
CLIPPY_WARNING_COUNT=$(grep -c "warning:" clippy.log || true)

if [ "$CLIPPY_WARNING_COUNT" -eq 0 ]; then
    echo "✅ No clippy warnings found!"
else
    echo "⚠️  Found $CLIPPY_WARNING_COUNT clippy warning(s)"
fi

# Clean up log files
rm -f build.log clippy.log

echo ""
echo "=================================="
if [ "$WARNING_COUNT" -eq 0 ] && [ "$CLIPPY_WARNING_COUNT" -eq 0 ]; then
    echo "✅ All checks passed! Code quality is excellent."
    exit 0
else
    echo "⚠️  Please fix the warnings above."
    exit 1
fi
