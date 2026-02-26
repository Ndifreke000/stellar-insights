# Script to check for sensitive data patterns in logs
# Usage: .\scripts\check_sensitive_logs.ps1 [log_file_or_directory]

param(
    [string]$LogSource = "."
)

Write-Host "🔍 Scanning for sensitive data patterns in logs..." -ForegroundColor Cyan
Write-Host ""

$IssuesFound = 0

function Check-Pattern {
    param(
        [string]$Pattern,
        [string]$Description,
        [string]$Severity
    )
    
    Write-Host "Checking for $Description..." -ForegroundColor Yellow
    
    if ($LogSource -eq ".") {
        # Check running application output
        $output = & cargo run 2>&1 | Select-String -Pattern $Pattern
        if ($output) {
            Write-Host "[$Severity] Found $Description in logs!" -ForegroundColor Red
            $script:IssuesFound++
        } else {
            Write-Host "✓ No $Description found" -ForegroundColor Green
        }
    } else {
        # Check log files
        $matches = Get-ChildItem -Path $LogSource -Recurse -File | 
                   Select-String -Pattern $Pattern -ErrorAction SilentlyContinue
        
        if ($matches) {
            Write-Host "[$Severity] Found $Description in logs!" -ForegroundColor Red
            $matches | Select-Object -First 5 | ForEach-Object {
                Write-Host "  $($_.Filename):$($_.LineNumber) - $($_.Line)" -ForegroundColor Gray
            }
            $script:IssuesFound++
        } else {
            Write-Host "✓ No $Description found" -ForegroundColor Green
        }
    }
    Write-Host ""
}

# Check for Stellar addresses (56 chars starting with G)
Check-Pattern -Pattern "G[A-Z0-9]{55}" -Description "Stellar addresses" -Severity "HIGH"

# Check for potential user IDs (8+ digit numbers)
Check-Pattern -Pattern "\buser[_-]?[0-9]{8,}\b" -Description "User IDs" -Severity "HIGH"

# Check for email addresses
Check-Pattern -Pattern "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}" -Description "Email addresses" -Severity "MEDIUM"

# Check for IPv4 addresses (not redacted)
Check-Pattern -Pattern "\b([0-9]{1,3}\.){3}[0-9]{1,3}\b" -Description "IP addresses (IPv4)" -Severity "MEDIUM"

# Check for potential API keys (common patterns)
Check-Pattern -Pattern "\b(sk_live_|pk_live_|api_key_)[a-zA-Z0-9]{20,}\b" -Description "API keys" -Severity "CRITICAL"

# Check for potential transaction hashes (64 hex chars)
Check-Pattern -Pattern "\b[a-f0-9]{64}\b" -Description "Transaction hashes" -Severity "MEDIUM"

# Check for potential JWT tokens
Check-Pattern -Pattern "eyJ[a-zA-Z0-9_-]{20,}\.[a-zA-Z0-9_-]{20,}\.[a-zA-Z0-9_-]{20,}" -Description "JWT tokens" -Severity "CRITICAL"

# Check for credit card patterns (just in case)
Check-Pattern -Pattern "\b[0-9]{4}[- ]?[0-9]{4}[- ]?[0-9]{4}[- ]?[0-9]{4}\b" -Description "Credit card numbers" -Severity "CRITICAL"

# Check for password-related logging
Check-Pattern -Pattern "password[`"']?\s*[:=]\s*[`"']?[^`"'\s]{8,}" -Description "Passwords" -Severity "CRITICAL"

# Summary
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
if ($IssuesFound -eq 0) {
    Write-Host "✓ No sensitive data patterns found!" -ForegroundColor Green
    Write-Host "✓ Logs appear to be properly redacted." -ForegroundColor Green
    exit 0
} else {
    Write-Host "✗ Found $IssuesFound potential issues!" -ForegroundColor Red
    Write-Host "⚠ Please review and apply appropriate redaction." -ForegroundColor Yellow
    exit 1
}
