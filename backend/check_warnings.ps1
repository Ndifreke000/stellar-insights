# PowerShell script to check for compiler warnings and clippy issues

Write-Host "🔍 Checking for compiler warnings..." -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan

# Build and capture warnings
Write-Host ""
Write-Host "📦 Running cargo build..." -ForegroundColor Yellow
$buildOutput = cargo build 2>&1 | Out-String
$buildOutput | Out-File -FilePath "build.log"
Write-Host $buildOutput

# Count warnings
$warningCount = ($buildOutput | Select-String -Pattern "warning:" -AllMatches).Matches.Count

if ($warningCount -eq 0) {
    Write-Host "✅ No compiler warnings found!" -ForegroundColor Green
} else {
    Write-Host "⚠️  Found $warningCount compiler warning(s)" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Warnings:" -ForegroundColor Yellow
    $buildOutput | Select-String -Pattern "warning:"
}

Write-Host ""
Write-Host "🔧 Running cargo clippy..." -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan

# Run clippy
$clippyOutput = cargo clippy --all-targets --all-features 2>&1 | Out-String
$clippyOutput | Out-File -FilePath "clippy.log"
Write-Host $clippyOutput

# Count clippy warnings
$clippyWarningCount = ($clippyOutput | Select-String -Pattern "warning:" -AllMatches).Matches.Count

if ($clippyWarningCount -eq 0) {
    Write-Host "✅ No clippy warnings found!" -ForegroundColor Green
} else {
    Write-Host "⚠️  Found $clippyWarningCount clippy warning(s)" -ForegroundColor Yellow
}

# Clean up log files
Remove-Item -Path "build.log" -ErrorAction SilentlyContinue
Remove-Item -Path "clippy.log" -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "==================================" -ForegroundColor Cyan
if ($warningCount -eq 0 -and $clippyWarningCount -eq 0) {
    Write-Host "✅ All checks passed! Code quality is excellent." -ForegroundColor Green
    exit 0
} else {
    Write-Host "⚠️  Please fix the warnings above." -ForegroundColor Yellow
    exit 1
}
