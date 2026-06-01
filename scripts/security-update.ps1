#!/usr/bin/env pwsh
# Security Update Script for Stellar Insights Frontend
# This script updates vulnerable dependencies and verifies the fixes

Write-Host "🔒 Stellar Insights - Security Update Script" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host ""

# Change to frontend directory
Set-Location -Path "$PSScriptRoot/frontend"

# Step 1: Backup current package-lock.json
Write-Host "📦 Step 1: Backing up package-lock.json..." -ForegroundColor Yellow
if (Test-Path "package-lock.json") {
    Copy-Item "package-lock.json" "package-lock.json.backup"
    Write-Host "✅ Backup created: package-lock.json.backup" -ForegroundColor Green
} else {
    Write-Host "⚠️  No package-lock.json found" -ForegroundColor Yellow
}
Write-Host ""

# Step 2: Check current vulnerabilities
Write-Host "🔍 Step 2: Checking current vulnerabilities..." -ForegroundColor Yellow
$auditBefore = npm audit --json | ConvertFrom-Json
$vulnCountBefore = $auditBefore.metadata.vulnerabilities.total
Write-Host "Found $vulnCountBefore vulnerabilities before update" -ForegroundColor $(if ($vulnCountBefore -gt 0) { "Red" } else { "Green" })
Write-Host ""

# Step 3: Install/Update dependencies
Write-Host "📥 Step 3: Installing updated dependencies..." -ForegroundColor Yellow
npm install
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Failed to install dependencies" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Dependencies installed successfully" -ForegroundColor Green
Write-Host ""

# Step 4: Run npm audit fix
Write-Host "🔧 Step 4: Running npm audit fix..." -ForegroundColor Yellow
npm audit fix
Write-Host ""

# Step 5: Check vulnerabilities after update
Write-Host "🔍 Step 5: Verifying security fixes..." -ForegroundColor Yellow
$auditAfter = npm audit --json | ConvertFrom-Json
$vulnCountAfter = $auditAfter.metadata.vulnerabilities.total

Write-Host ""
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "📊 Security Audit Results" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "Before: $vulnCountBefore vulnerabilities" -ForegroundColor $(if ($vulnCountBefore -gt 0) { "Red" } else { "Green" })
Write-Host "After:  $vulnCountAfter vulnerabilities" -ForegroundColor $(if ($vulnCountAfter -gt 0) { "Red" } else { "Green" })
Write-Host ""

if ($vulnCountAfter -eq 0) {
    Write-Host "✅ SUCCESS: All vulnerabilities fixed!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Cyan
    Write-Host "1. Test the application: npm run dev" -ForegroundColor White
    Write-Host "2. Run tests: npm test" -ForegroundColor White
    Write-Host "3. Commit changes: git add package.json package-lock.json" -ForegroundColor White
    Write-Host "4. Remove backup: Remove-Item package-lock.json.backup" -ForegroundColor White
} else {
    Write-Host "⚠️  WARNING: $vulnCountAfter vulnerabilities remain" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Detailed audit report:" -ForegroundColor Cyan
    npm audit
    Write-Host ""
    Write-Host "To restore backup:" -ForegroundColor Yellow
    Write-Host "Copy-Item package-lock.json.backup package-lock.json -Force" -ForegroundColor White
}

Write-Host ""
Write-Host "=============================================" -ForegroundColor Cyan
