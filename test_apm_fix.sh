#!/bin/bash
# Test script to validate the APM module fix

set -e

echo "🔍 Checking for merge conflicts in Cargo.toml files..."
if grep -r "^<<<<<<< \|^=======$\|^>>>>>>> " backend/**/*.toml 2>/dev/null; then
    echo "❌ Merge conflicts still present!"
    exit 1
fi
echo "✅ No merge conflicts found"

echo ""
echo "🧹 Cleaning build artifacts..."
cd backend
cargo clean 2>/dev/null || echo "⚠️  Cargo not available in this environment"

echo ""
echo "🔨 Building backend..."
cargo build 2>&1 || {
    echo "❌ Build failed"
    exit 1
}

echo ""
echo "📋 Running clippy..."
cargo clippy --all-targets --all-features 2>&1 || {
    echo "⚠️  Clippy warnings found"
}

echo ""
echo "🧪 Running tests..."
cargo test 2>&1 || {
    echo "❌ Tests failed"
    exit 1
}

echo ""
echo "✅ All checks passed!"
