# =============================================================================
# Load Test Runner Script (PowerShell)
# =============================================================================
# This script runs all load tests sequentially and generates a comprehensive
# report of the results.
# =============================================================================

param(
    [string]$BaseUrl = "http://localhost:8080"
)

$ErrorActionPreference = "Stop"

# Configuration
$ResultsDir = "./results"
$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"

# Create results directory
New-Item -ItemType Directory -Force -Path $ResultsDir | Out-Null

Write-Host "==============================================================================" -ForegroundColor Blue
Write-Host "STELLAR INSIGHTS - LOAD TEST SUITE" -ForegroundColor Blue
Write-Host "==============================================================================" -ForegroundColor Blue
Write-Host ""
Write-Host "Base URL: $BaseUrl" -ForegroundColor Green
Write-Host "Results Directory: $ResultsDir" -ForegroundColor Green
Write-Host "Timestamp: $Timestamp" -ForegroundColor Green
Write-Host ""

# Check if k6 is installed
try {
    $null = Get-Command k6 -ErrorAction Stop
} catch {
    Write-Host "Error: k6 is not installed" -ForegroundColor Red
    Write-Host "Please install k6: https://k6.io/docs/getting-started/installation/"
    exit 1
}

# Check if server is running
Write-Host "Checking server health..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "$BaseUrl/health" -UseBasicParsing -TimeoutSec 5
    if ($response.StatusCode -eq 200) {
        Write-Host "✓ Server is healthy" -ForegroundColor Green
    }
} catch {
    Write-Host "Error: Server is not responding at $BaseUrl" -ForegroundColor Red
    Write-Host "Please start the server before running load tests."
    exit 1
}
Write-Host ""

# Function to run a test
function Run-Test {
    param(
        [string]$TestName,
        [string]$TestFile
    )
    
    $OutputFile = "$ResultsDir/${TestName}_${Timestamp}.json"
    
    Write-Host "==============================================================================" -ForegroundColor Blue
    Write-Host "Running: $TestName" -ForegroundColor Blue
    Write-Host "==============================================================================" -ForegroundColor Blue
    
    try {
        k6 run --out "json=$OutputFile" $TestFile
        Write-Host "✓ $TestName completed successfully" -ForegroundColor Green
        Write-Host "Results saved to: $OutputFile" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "✗ $TestName failed" -ForegroundColor Red
        return $false
    }
    
    Write-Host ""
}

# Track test results
$testResults = @()
$testNames = @()

# Run tests
Write-Host "Starting load tests..." -ForegroundColor Yellow
Write-Host ""

# 1. Corridors Load Test
$result = Run-Test -TestName "corridors-load-test" -TestFile "corridors-load-test.js"
$testResults += if ($result) { "PASS" } else { "FAIL" }
$testNames += "Corridors Load Test"
Start-Sleep -Seconds 5

# 2. Anchors Load Test
$result = Run-Test -TestName "anchors-load-test" -TestFile "anchors-load-test.js"
$testResults += if ($result) { "PASS" } else { "FAIL" }
$testNames += "Anchors Load Test"
Start-Sleep -Seconds 5

# 3. RPC Endpoints Load Test
$result = Run-Test -TestName "rpc-endpoints-load-test" -TestFile "rpc-endpoints-load-test.js"
$testResults += if ($result) { "PASS" } else { "FAIL" }
$testNames += "RPC Endpoints Load Test"
Start-Sleep -Seconds 5

# 4. Full Suite Load Test
$result = Run-Test -TestName "full-suite-load-test" -TestFile "full-suite-load-test.js"
$testResults += if ($result) { "PASS" } else { "FAIL" }
$testNames += "Full Suite Load Test"
Start-Sleep -Seconds 5

# Generate summary report
Write-Host ""
Write-Host "==============================================================================" -ForegroundColor Blue
Write-Host "TEST SUMMARY" -ForegroundColor Blue
Write-Host "==============================================================================" -ForegroundColor Blue
Write-Host ""

$passed = 0
$failed = 0

for ($i = 0; $i -lt $testNames.Count; $i++) {
    $testName = $testNames[$i]
    $result = $testResults[$i]
    
    if ($result -eq "PASS") {
        Write-Host "✓ $testName`: " -NoNewline -ForegroundColor Green
        Write-Host "PASSED" -ForegroundColor Green
        $passed++
    } else {
        Write-Host "✗ $testName`: " -NoNewline -ForegroundColor Red
        Write-Host "FAILED" -ForegroundColor Red
        $failed++
    }
}

Write-Host ""
Write-Host "Total Tests: $($passed + $failed)" -ForegroundColor Blue
Write-Host "Passed: $passed" -ForegroundColor Green
Write-Host "Failed: $failed" -ForegroundColor Red
Write-Host ""

# Generate summary file
$summaryFile = "$ResultsDir/summary_${Timestamp}.txt"
@"
STELLAR INSIGHTS - LOAD TEST SUMMARY
====================================

Date: $(Get-Date)
Base URL: $BaseUrl

Test Results:
-------------
"@ | Out-File -FilePath $summaryFile

for ($i = 0; $i -lt $testNames.Count; $i++) {
    "$($testNames[$i]): $($testResults[$i])" | Out-File -FilePath $summaryFile -Append
}

@"

Total: $($passed + $failed)
Passed: $passed
Failed: $failed
"@ | Out-File -FilePath $summaryFile -Append

Write-Host "Summary saved to: $summaryFile" -ForegroundColor Green
Write-Host ""

# Exit with appropriate code
if ($failed -eq 0) {
    Write-Host "All tests passed!" -ForegroundColor Green
    exit 0
} else {
    Write-Host "Some tests failed. Please review the results." -ForegroundColor Red
    exit 1
}
