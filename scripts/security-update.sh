#!/bin/bash
# Security Update Script for Stellar Insights Frontend
# This script updates vulnerable dependencies and verifies the fixes

set -e

echo "🔒 Stellar Insights - Security Update Script"
echo "============================================="
echo ""

# Change to frontend directory
cd "$(dirname "$0")/../frontend"

# Step 1: Backup current package-lock.json
echo "📦 Step 1: Backing up package-lock.json..."
if [ -f "package-lock.json" ]; then
    cp package-lock.json package-lock.json.backup
    echo "✅ Backup created: package-lock.json.backup"
else
    echo "⚠️  No package-lock.json found"
fi
echo ""

# Step 2: Check current vulnerabilities
echo "🔍 Step 2: Checking current vulnerabilities..."
VULN_COUNT_BEFORE=$(npm audit --json 2>/dev/null | jq '.metadata.vulnerabilities.total' || echo "0")
echo "Found $VULN_COUNT_BEFORE vulnerabilities before update"
echo ""

# Step 3: Install/Update dependencies
echo "📥 Step 3: Installing updated dependencies..."
npm install
echo "✅ Dependencies installed successfully"
echo ""

# Step 4: Run npm audit fix
echo "🔧 Step 4: Running npm audit fix..."
npm audit fix || true
echo ""

# Step 5: Check vulnerabilities after update
echo "🔍 Step 5: Verifying security fixes..."
VULN_COUNT_AFTER=$(npm audit --json 2>/dev/null | jq '.metadata.vulnerabilities.total' || echo "0")

echo ""
echo "============================================="
echo "📊 Security Audit Results"
echo "============================================="
echo "Before: $VULN_COUNT_BEFORE vulnerabilities"
echo "After:  $VULN_COUNT_AFTER vulnerabilities"
echo ""

if [ "$VULN_COUNT_AFTER" -eq 0 ]; then
    echo "✅ SUCCESS: All vulnerabilities fixed!"
    echo ""
    echo "Next steps:"
    echo "1. Test the application: npm run dev"
    echo "2. Run tests: npm test"
    echo "3. Commit changes: git add package.json package-lock.json"
    echo "4. Remove backup: rm package-lock.json.backup"
else
    echo "⚠️  WARNING: $VULN_COUNT_AFTER vulnerabilities remain"
    echo ""
    echo "Detailed audit report:"
    npm audit
    echo ""
    echo "To restore backup:"
    echo "cp package-lock.json.backup package-lock.json"
fi

echo ""
echo "============================================="
