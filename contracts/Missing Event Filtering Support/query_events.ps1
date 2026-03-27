# Stellar Snapshot Contract - Event Filtering Query Helper (Windows)
# This script provides common event query patterns using PowerShell

param(
    [Parameter(Mandatory=$true)]
    [string]$ContractId,
    
    [Parameter(Mandatory=$false)]
    [string]$Network = "testnet"
)

Write-Host "Stellar Event Filtering Query Helper" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Green
Write-Host "Contract ID: $ContractId"
Write-Host "Network: $Network"
Write-Host ""

# Query 1: All snapshot events
Write-Host "[1] Query all snapshot events:" -ForegroundColor Cyan
Write-Host "stellar contract events --id $ContractId ``" -ForegroundColor Gray
Write-Host "  --topic snapshot ``" -ForegroundColor Gray
Write-Host "  --network $Network" -ForegroundColor Gray
Write-Host ""

# Query 2: Snapshots from specific submitter
Write-Host "[2] Query snapshots from specific submitter:" -ForegroundColor Cyan
Write-Host "stellar contract events --id $ContractId ``" -ForegroundColor Gray
Write-Host "  --topic snapshot ``" -ForegroundColor Gray
Write-Host "  --topic <SUBMITTER_ADDRESS> ``" -ForegroundColor Gray
Write-Host "  --network $Network" -ForegroundColor Gray
Write-Host ""

# Query 3: Snapshots for specific epoch
Write-Host "[3] Query snapshots for specific epoch:" -ForegroundColor Cyan
Write-Host "stellar contract events --id $ContractId ``" -ForegroundColor Gray
Write-Host "  --topic snapshot ``" -ForegroundColor Gray
Write-Host "  --topic <SUBMITTER_ADDRESS> ``" -ForegroundColor Gray
Write-Host "  --topic <EPOCH_NUMBER> ``" -ForegroundColor Gray
Write-Host "  --network $Network" -ForegroundColor Gray
Write-Host ""

# Query 4: Admin change events
Write-Host "[4] Query admin change events:" -ForegroundColor Cyan
Write-Host "stellar contract events --id $ContractId ``" -ForegroundColor Gray
Write-Host "  --topic admin_ch ``" -ForegroundColor Gray
Write-Host "  --network $Network" -ForegroundColor Gray
Write-Host ""

# Query 5: Pause events
Write-Host "[5] Query pause events:" -ForegroundColor Cyan
Write-Host "stellar contract events --id $ContractId ``" -ForegroundColor Gray
Write-Host "  --topic pause ``" -ForegroundColor Gray
Write-Host "  --network $Network" -ForegroundColor Gray
Write-Host ""

# Query 6: Governance set events
Write-Host "[6] Query governance set events:" -ForegroundColor Cyan
Write-Host "stellar contract events --id $ContractId ``" -ForegroundColor Gray
Write-Host "  --topic gov_set ``" -ForegroundColor Gray
Write-Host "  --network $Network" -ForegroundColor Gray
Write-Host ""

Write-Host "✓ Common query patterns listed above" -ForegroundColor Green
Write-Host ""

Write-Host "Usage Notes:" -ForegroundColor Yellow
Write-Host "- Replace <SUBMITTER_ADDRESS> with actual submitter address"
Write-Host "- Replace <EPOCH_NUMBER> with actual epoch number"
Write-Host "- Topics are matched in order (first topic must match 'snapshot', etc.)"
Write-Host "- Omit topics for broader queries"
