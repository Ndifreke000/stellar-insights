#!/bin/bash

# Security Update Script
# This script updates frontend dependencies to fix security vulnerabilities

set -e  # Exit on error

echo "🔒 Security Dependency Update Script"
echo "===================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're in the frontend directory
if [ ! -f "package.json" ]; then
    echo -e "${RED}Error: package.json not found. Please run this script from the frontend directory.${NC}"
    exit 1
fi

echo "📦 Step 1: Backing up current package-lock.json..."
if [ -f "package-lock.json" ]; then
    cp package-lock.json package-lock.json.backup
    echo -e "${GREEN}✓ Backup created: package-lock.json.backup${NC}"
else
    echo -e "${YELLOW}⚠ No package-lock.json found (this is okay)${NC}"
fi

echo ""
echo "🗑️  Step 2: Cleaning node_modules and lock file..."
rm -rf node_modules
rm -f package-lock.json
echo -e "${GREEN}✓ Cleaned${NC}"

echo ""
echo "📥 Step 3: Installing updated dependencies..."
npm install

echo ""
echo "🔍 Step 4: Running security audit..."
if npm audit --audit-level=moderate; then
    echo -e "${GREEN}✓ No vulnerabilities found!${NC}"
else
    echo -e "${RED}⚠ Vulnerabilities detected. Review output above.${NC}"
    echo ""
    echo "Generating detailed audit report..."
    npm audit --json > audit-report.json
    echo -e "${YELLOW}Audit report saved to: audit-report.json${NC}"
fi

echo ""
echo "🧪 Step 5: Running tests..."
if npm test -- --run; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
else
    echo -e "${RED}⚠ Some tests failed. Review output above.${NC}"
fi

echo ""
echo "🔨 Step 6: Building project..."
if npm run build; then
    echo -e "${GREEN}✓ Build successful!${NC}"
else
    echo -e "${RED}⚠ Build failed. Review output above.${NC}"
    exit 1
fi

echo ""
echo "📊 Step 7: Checking package versions..."
echo ""
echo "Updated packages:"
npm list jspdf next eslint prisma @prisma/client 2>/dev/null || true

echo ""
echo "✅ Security update complete!"
echo ""
echo "Next steps:"
echo "1. Test PDF export functionality"
echo "2. Test chart export functionality"
echo "3. Run full test suite: npm test"
echo "4. Review SECURITY_UPDATE_GUIDE.md for details"
echo ""
echo "To rollback if needed:"
echo "  git checkout HEAD -- package.json"
echo "  mv package-lock.json.backup package-lock.json"
echo "  npm install"
