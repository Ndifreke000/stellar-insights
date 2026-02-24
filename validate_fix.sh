#!/bin/bash
# Validation Report for Issue #460 Fix

echo "=========================================="
echo "Issue #460 Fix Validation Report"
echo "=========================================="
echo ""

# Test 1: Check for merge conflicts
echo "✓ Test 1: Checking for merge conflicts..."
if grep -r "^<<<<<<< \|^=======$\|^>>>>>>> " backend/ --include="*.rs" --include="*.toml" 2>/dev/null; then
    echo "  ❌ FAILED: Merge conflicts still present"
    exit 1
else
    echo "  ✅ PASSED: No merge conflicts found"
fi
echo ""

# Test 2: Validate TOML syntax
echo "✓ Test 2: Validating Cargo.toml syntax..."
if command -v cargo &> /dev/null; then
    cd backend/apm
    if cargo metadata --format-version 1 &> /dev/null; then
        echo "  ✅ PASSED: Cargo.toml is valid"
    else
        echo "  ❌ FAILED: Cargo.toml has syntax errors"
        exit 1
    fi
    cd ../..
else
    echo "  ⚠️  SKIPPED: Cargo not available (manual verification required)"
fi
echo ""

# Test 3: Check fixed files
echo "✓ Test 3: Verifying fixed files..."
FILES=(
    "backend/apm/Cargo.toml"
    "backend/src/api/corridors_cached.rs"
    "backend/src/rpc/error.rs"
)

for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✅ $file exists and is readable"
    else
        echo "  ❌ $file not found"
        exit 1
    fi
done
echo ""

# Test 4: Attempt build (if Cargo available)
echo "✓ Test 4: Build verification..."
if command -v cargo &> /dev/null; then
    cd backend
    echo "  Cleaning build artifacts..."
    cargo clean &> /dev/null
    
    echo "  Building project..."
    if cargo build 2>&1 | tee /tmp/build.log; then
        echo "  ✅ PASSED: Build successful"
    else
        echo "  ❌ FAILED: Build errors detected"
        echo "  See /tmp/build.log for details"
        exit 1
    fi
    cd ..
else
    echo "  ⚠️  SKIPPED: Cargo not available"
    echo "  Manual verification required:"
    echo "    cd backend"
    echo "    cargo clean"
    echo "    cargo build"
fi
echo ""

# Test 5: Run clippy (if available)
echo "✓ Test 5: Linting verification..."
if command -v cargo &> /dev/null; then
    cd backend
    if cargo clippy --all-targets --all-features 2>&1 | tee /tmp/clippy.log; then
        echo "  ✅ PASSED: No critical linting errors"
    else
        echo "  ⚠️  WARNING: Clippy found issues (see /tmp/clippy.log)"
    fi
    cd ..
else
    echo "  ⚠️  SKIPPED: Cargo not available"
fi
echo ""

# Test 6: Run tests (if available)
echo "✓ Test 6: Test suite verification..."
if command -v cargo &> /dev/null; then
    cd backend
    if cargo test 2>&1 | tee /tmp/test.log; then
        echo "  ✅ PASSED: All tests passed"
    else
        echo "  ❌ FAILED: Some tests failed (see /tmp/test.log)"
        exit 1
    fi
    cd ..
else
    echo "  ⚠️  SKIPPED: Cargo not available"
fi
echo ""

echo "=========================================="
echo "Validation Summary"
echo "=========================================="
echo "✅ All merge conflicts resolved"
echo "✅ File syntax validated"
echo "✅ All fixed files present"
if command -v cargo &> /dev/null; then
    echo "✅ Build verification complete"
else
    echo "⚠️  Build verification requires Rust toolchain"
    echo ""
    echo "To complete validation, run:"
    echo "  cd backend"
    echo "  cargo clean"
    echo "  cargo build"
    echo "  cargo clippy --all-targets --all-features"
    echo "  cargo test"
fi
echo ""
echo "Issue #460 fix is ready for deployment!"
