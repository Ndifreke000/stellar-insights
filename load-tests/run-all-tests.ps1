# Stellar Insights Backend - Load Test Runner (PowerShell)
# Runs all k6 test suites sequentially and generates reports

param(
    [string]$BaseUrl = "http://localhost:8080"
)

# Configuration
$ResultsDir = "load-test-results"
$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"

Write-Host "========================================" -ForegroundColor Blue
Write-Host "Stellar Insights Load Test Suite" -ForegroundColor Blue
Write-Host "========================================" -ForegroundColor Blue
Write-Host "Target: " -NoNewline
Write-Host $BaseUrl -ForegroundColor Green
Write-Host "Timestamp: $Timestamp"
Write-Host ""

# Create results directory
New-Item -ItemType Directory -Force -Path $ResultsDir | Out-Null

# Check if k6 is installed
if (-not (Get-Command k6 -ErrorAction SilentlyContinue)) {
    Write-Host "Error: k6 is not installed" -ForegroundColor Red
    Write-Host "Please install k6: https://k6.io/docs/getting-started/installation/"
    exit 1
}

# Check if server is accessible
Write-Host "Checking server health..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "$BaseUrl/health" -UseBasicParsing -TimeoutSec 5
    if ($response.StatusCode -eq 200) {
        Write-Host "✓ Server is accessible" -ForegroundColor Green
        Write-Host ""
    }
} catch {
    Write-Host "Error: Server not accessible at $BaseUrl" -ForegroundColor Red
    Write-Host "Please ensure the backend is running"
    exit 1
}

# Function to run a test
function Run-Test {
    param(
        [string]$TestName,
        [string]$TestFile
    )
    
    $OutputFile = "$ResultsDir\${TestName}_${Timestamp}"
    
    Write-Host "========================================" -ForegroundColor Blue
    Write-Host "Running: $TestName" -ForegroundColor Blue
    Write-Host "========================================" -ForegroundColor Blue
    
    $env:BASE_URL = $BaseUrl
    $result = k6 run --out "json=$OutputFile.json" $TestFile
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ $TestName completed successfully" -ForegroundColor Green
        Write-Host ""
        return $true
    } else {
        Write-Host "✗ $TestName failed" -ForegroundColor Red
        Write-Host ""
        return $false
    }
}

# Track results
$TotalTests = 0
$PassedTests = 0
$FailedTests = 0

# Define tests
$tests = @(
    @{Name="Health Check"; File="load-tests\k6-cache-metrics.js"},
    @{Name="Corridors"; File="load-tests\k6-corridors.js"},
    @{Name="Anchors"; File="load-tests\k6-anchors.js"},
    @{Name="RPC"; File="load-tests\k6-rpc.js"},
    @{Name="Liquidity Pools"; File="load-tests\k6-liquidity-pools.js"},
    @{Name="Fee Bumps"; File="load-tests\k6-fee-bumps.js"},
    @{Name="Full Suite"; File="load-tests\k6-full-suite.js"}
)

# Run all tests
foreach ($test in $tests) {
    $TotalTests++
    
    if (Run-Test -TestName $test.Name -TestFile $test.File) {
        $PassedTests++
    } else {
        $FailedTests++
    }
    
    # Wait between tests
    if ($TotalTests -lt $tests.Count) {
        Write-Host "Waiting 10 seconds before next test..." -ForegroundColor Yellow
        Start-Sleep -Seconds 10
    }
}

# Summary
Write-Host "========================================" -ForegroundColor Blue
Write-Host "Test Summary" -ForegroundColor Blue
Write-Host "========================================" -ForegroundColor Blue
Write-Host "Total Tests: $TotalTests"
Write-Host "Passed: " -NoNewline
Write-Host $PassedTests -ForegroundColor Green
if ($FailedTests -gt 0) {
    Write-Host "Failed: " -NoNewline
    Write-Host $FailedTests -ForegroundColor Red
} else {
    Write-Host "Failed: $FailedTests"
}
Write-Host ""
Write-Host "Results saved to: $ResultsDir\"
Write-Host ""

# Exit with appropriate code
if ($FailedTests -gt 0) {
    Write-Host "Some tests failed" -ForegroundColor Red
    exit 1
} else {
    Write-Host "All tests passed!" -ForegroundColor Green
    exit 0
}
