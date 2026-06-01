# Script to verify performance indexes are created correctly
# Usage: .\scripts\verify_indexes.ps1

$ErrorActionPreference = "Stop"

$DB_URL = if ($env:DATABASE_URL) { $env:DATABASE_URL } else { "sqlite:./stellar_insights.db" }
$DB_PATH = $DB_URL -replace "^sqlite:", ""

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Performance Index Verification" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Database: $DB_PATH"
Write-Host ""

if (-not (Test-Path $DB_PATH)) {
    Write-Host "❌ Database file not found: $DB_PATH" -ForegroundColor Red
    Write-Host "Please run migrations first: sqlx migrate run"
    exit 1
}

Write-Host "✅ Database file found" -ForegroundColor Green
Write-Host ""

# Function to check if index exists
function Check-Index {
    param(
        [string]$IndexName,
        [string]$TableName
    )
    
    $result = sqlite3 $DB_PATH "SELECT name FROM sqlite_master WHERE type='index' AND name='$IndexName';"
    
    if ($result) {
        Write-Host "✅ $IndexName (on $TableName)" -ForegroundColor Green
        return $true
    } else {
        Write-Host "❌ $IndexName (on $TableName) - MISSING" -ForegroundColor Red
        return $false
    }
}

Write-Host "Checking Payments Table Indexes:" -ForegroundColor Yellow
Write-Host "-----------------------------------"
Check-Index "idx_payments_created_at" "payments"
Check-Index "idx_payments_asset_code" "payments"
Check-Index "idx_payments_source_account_date" "payments"
Check-Index "idx_payments_dest_account_date" "payments"
Check-Index "idx_payments_transaction_hash" "payments"
Check-Index "idx_payments_source_account" "payments"
Check-Index "idx_payments_destination_account" "payments"
Write-Host ""

Write-Host "Checking Assets Table Indexes:" -ForegroundColor Yellow
Write-Host "-----------------------------------"
Check-Index "idx_assets_asset_code" "assets"
Check-Index "idx_assets_asset_issuer" "assets"
Check-Index "idx_assets_code_issuer" "assets"
Check-Index "idx_assets_anchor_id_code" "assets"
Check-Index "idx_assets_anchor" "assets"
Write-Host ""

Write-Host "Checking Corridors Table Indexes:" -ForegroundColor Yellow
Write-Host "-----------------------------------"
Check-Index "idx_corridors_source_dest" "corridors"
Check-Index "idx_corridors_dest_source" "corridors"
Check-Index "idx_corridors_reliability_score" "corridors"
Check-Index "idx_corridors_status" "corridors"
Check-Index "idx_corridors_reliability" "corridors"
Write-Host ""

Write-Host "Checking Corridor Metrics Indexes:" -ForegroundColor Yellow
Write-Host "-----------------------------------"
Check-Index "idx_corridor_metrics_key_date" "corridor_metrics"
Check-Index "idx_corridor_metrics_date" "corridor_metrics"
Write-Host ""

Write-Host "Checking Anchor Indexes:" -ForegroundColor Yellow
Write-Host "-----------------------------------"
Check-Index "idx_anchors_stellar_account" "anchors"
Check-Index "idx_anchors_reliability_score" "anchors"
Check-Index "idx_anchors_status" "anchors"
Write-Host ""

Write-Host "Checking Anchor Metrics History Indexes:" -ForegroundColor Yellow
Write-Host "-----------------------------------"
Check-Index "idx_anchor_metrics_anchor_timestamp" "anchor_metrics_history"
Check-Index "idx_anchor_metrics_anchor_time" "anchor_metrics_history"
Check-Index "idx_anchor_metrics_timestamp" "anchor_metrics_history"
Write-Host ""

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Query Plan Verification" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "Testing: Payment time-range query" -ForegroundColor Yellow
Write-Host "-----------------------------------"
sqlite3 $DB_PATH "EXPLAIN QUERY PLAN SELECT * FROM payments WHERE created_at > datetime('now', '-1 day');"
Write-Host ""

Write-Host "Testing: Payment by source account" -ForegroundColor Yellow
Write-Host "-----------------------------------"
sqlite3 $DB_PATH "EXPLAIN QUERY PLAN SELECT * FROM payments WHERE source_account = 'GXXXXXX';"
Write-Host ""

Write-Host "Testing: Corridor lookup" -ForegroundColor Yellow
Write-Host "-----------------------------------"
sqlite3 $DB_PATH "EXPLAIN QUERY PLAN SELECT * FROM corridors WHERE source_asset_code = 'USDC' AND destination_asset_code = 'XLM';"
Write-Host ""

Write-Host "Testing: Asset by code" -ForegroundColor Yellow
Write-Host "-----------------------------------"
sqlite3 $DB_PATH "EXPLAIN QUERY PLAN SELECT * FROM assets WHERE asset_code = 'USDC';"
Write-Host ""

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Index Statistics" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "Total indexes per table:"
sqlite3 $DB_PATH @"
SELECT 
    tbl_name as table_name,
    COUNT(*) as index_count
FROM sqlite_master 
WHERE type='index' AND tbl_name IN ('payments', 'assets', 'corridors', 'corridor_metrics', 'anchors', 'anchor_metrics_history')
GROUP BY tbl_name
ORDER BY tbl_name;
"@
Write-Host ""

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Table Row Counts" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

$paymentsCount = sqlite3 $DB_PATH "SELECT COUNT(*) FROM payments;"
$assetsCount = sqlite3 $DB_PATH "SELECT COUNT(*) FROM assets;"
$corridorsCount = sqlite3 $DB_PATH "SELECT COUNT(*) FROM corridors;"
$anchorsCount = sqlite3 $DB_PATH "SELECT COUNT(*) FROM anchors;"

Write-Host "Payments: $paymentsCount"
Write-Host "Assets: $assetsCount"
Write-Host "Corridors: $corridorsCount"
Write-Host "Anchors: $anchorsCount"
Write-Host ""

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Verification Complete!" -ForegroundColor Green
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:"
Write-Host "1. If any indexes are missing, run: sqlx migrate run"
Write-Host "2. Monitor query performance in application logs"
Write-Host "3. Run load tests to verify improvements"
Write-Host "4. Check for 'USING INDEX' in query plans above"
Write-Host ""
