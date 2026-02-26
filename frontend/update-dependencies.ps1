# Security Update Script (PowerShell)
# This script updates frontend dependencies to fix security vulnerabilities

$ErrorActionPreference = "Stop"

Write-Host "🔒 Security Dependency Update Script" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan
Write-Host ""

# Check if we're in the frontend directory
if (-not (Test-Path "package.json")) {
    Write-Host "Error: package.json not found. Please run this script from the frontend directory." -ForegroundColor Red
    exit 1
}

Write-Host "📦 Step 1: Backing up current package-lock.json..." -ForegroundColor Yellow
if (Test-Path "package-lock.json") {
    Copy-Item "package-lock.json" "package-lock.json.backup"
    Write-Host "✓ Backup created: package-lock.json.backup" -ForegroundColor Green
} else {
    Write-Host "⚠ No package-lock.json found (this is okay)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "🗑️  Step 2: Cleaning node_modules and lock file..." -ForegroundColor Yellow
if (Test-Path "node_modules") {
    Remove-Item -Recurse -Force "node_modules"
}
if (Test-Path "package-lock.json") {
    Remove-Item "package-lock.json"
}
Write-Host "✓ Cleaned" -ForegroundColor Green

Write-Host ""
Write-Host "📥 Step 3: Installing updated dependencies..." -ForegroundColor Yellow
npm install

Write-Host ""
Write-Host "🔍 Step 4: Running security audit..." -ForegroundColor Yellow
$auditResult = npm audit --audit-level=moderate
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ No vulnerabilities found!" -ForegroundColor Green
} else {
    Write-Host "⚠ Vulnerabilities detected. Review output above." -ForegroundColor Red
    Write-Host ""
    Write-Host "Generating detailed audit report..." -ForegroundColor Yellow
    npm audit --json | Out-File -Encoding UTF8 "audit-report.json"
    Write-Host "Audit report saved to: audit-report.json" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "🧪 Step 5: Running tests..." -ForegroundColor Yellow
$testResult = npm test -- --run
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ All tests passed!" -ForegroundColor Green
} else {
    Write-Host "⚠ Some tests failed. Review output above." -ForegroundColor Red
}

Write-Host ""
Write-Host "🔨 Step 6: Building project..." -ForegroundColor Yellow
$buildResult = npm run build
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Build successful!" -ForegroundColor Green
} else {
    Write-Host "⚠ Build failed. Review output above." -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "📊 Step 7: Checking package versions..." -ForegroundColor Yellow
Write-Host ""
Write-Host "Updated packages:" -ForegroundColor Cyan
npm list jspdf next eslint prisma "@prisma/client" 2>$null

Write-Host ""
Write-Host "✅ Security update complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "1. Test PDF export functionality"
Write-Host "2. Test chart export functionality"
Write-Host "3. Run full test suite: npm test"
Write-Host "4. Review SECURITY_UPDATE_GUIDE.md for details"
Write-Host ""
Write-Host "To rollback if needed:" -ForegroundColor Yellow
Write-Host "  git checkout HEAD -- package.json"
Write-Host "  Move-Item package-lock.json.backup package-lock.json"
Write-Host "  npm install"
