#!/bin/bash
# Phase 2: Remove Unused/Incomplete Features
# WARNING: This script deletes code. Review carefully before running.

set -e

cd "$(dirname "$0")/../.."

echo "🗑️  Phase 2: Removing Unused Features"
echo "====================================="
echo ""
echo "⚠️  WARNING: This will delete code!"
echo "    Make sure you have a backup branch."
echo ""
read -p "Continue? (yes/no): " confirm

if [ "$confirm" != "yes" ]; then
    echo "Aborted."
    exit 1
fi

# 2.1 Remove ML/AI Stubs
echo ""
echo "Removing ML/AI stub files..."
rm -f backend/src/ml.rs
rm -f backend/src/ml_tests.rs
rm -f backend/src/ml_handlers.rs
rm -f backend/examples/ml_test.rs
echo "  ✓ Removed ML stub files"

# 2.2 Remove Game Components
echo ""
echo "Removing game components..."
rm -rf frontend/src/app/game
rm -f frontend/src/components/DiceAnimation.tsx
echo "  ✓ Removed game components"

# 2.3 Remove Orphaned Contract Folders
echo ""
echo "Removing orphaned contract folders..."
rm -rf "contracts/Add Health Check and Readiness Endpoints"
rm -rf "contracts/Add Request Validation Middleware"
rm -rf "contracts/Implement Proper Cache Invalidation Strategy"
rm -rf "contracts/Implement Stellar DEX Liquidity Aggregator"
rm -rf "contracts/Proper Cache Invalidation Strategy"
rm -rf "contracts/example-contract"
echo "  ✓ Removed orphaned contract folders"

# 2.4 Remove Temporary/Output Files
echo ""
echo "Removing temporary files..."
rm -f backend/check_out.txt
rm -f backend/check_output.txt
rm -f backend/errors.txt
rm -f backend/check_warnings.ps1
rm -f backend/check_warnings.sh
echo "  ✓ Removed temporary files"

# 2.5 Create removal summary
echo ""
echo "Creating removal summary..."
cat > REMOVED_FEATURES.md << 'EOF'
# Removed Features Summary

This document tracks features removed during the refactoring process.

## ML/AI Features (Removed)

**Reason:** Incomplete stubs with no real implementation

**Files removed:**
- `backend/src/ml.rs`
- `backend/src/ml_tests.rs`
- `backend/src/ml_handlers.rs`
- `backend/examples/ml_test.rs`

**To re-add:** Design proper ML architecture first, then implement.

## Game Components (Removed)

**Reason:** Not part of core analytics platform

**Files removed:**
- `frontend/src/app/game/` (empty directory)
- `frontend/src/components/DiceAnimation.tsx`

**To re-add:** Define game feature requirements and user stories.

## Orphaned Contract Folders (Removed)

**Reason:** Leftover from development, not actual contracts

**Folders removed:**
- `contracts/Add Health Check and Readiness Endpoints/`
- `contracts/Add Request Validation Middleware/`
- `contracts/Implement Proper Cache Invalidation Strategy/`
- `contracts/Implement Stellar DEX Liquidity Aggregator/`
- `contracts/Proper Cache Invalidation Strategy/`
- `contracts/example-contract/`

## Features Requiring Decision

### SEP-24/31 Proxy Implementations

**Status:** NOT REMOVED (requires team decision)

**Files:**
- `backend/src/services/sep24_proxy.rs` (12,957 chars)
- `backend/src/services/sep31_proxy.rs` (13,050 chars)
- `frontend/src/components/Sep24Flow.tsx` (17,432 chars)
- `frontend/src/components/Sep31PaymentFlow.tsx` (17,754 chars)
- `frontend/src/components/Sep6DepositForm.tsx` (8,185 chars)
- `frontend/src/components/Sep6WithdrawForm.tsx` (9,824 chars)

**Question:** Are these used in production?
- If NO: Remove them
- If YES: Move to dedicated `backend/src/sep/` module

### Telegram Bot Integration

**Status:** NOT REMOVED (requires team decision)

**Files:**
- `backend/src/telegram/` directory
- `backend/migrations/017_create_telegram_subscriptions.sql`

**Question:** Is Telegram bot actually integrated?
- If NO: Remove it
- If YES: Document integration and complete implementation

### Governance/Analytics Contracts

**Status:** NOT REMOVED (requires team decision)

**Contracts:**
- `contracts/governance/` - Governance contract
- `contracts/analytics/` - Analytics contract
- `contracts/access-control/` - ACL contract
- `contracts/secure-contract/` - Security contract

**Question:** Which contracts are deployed vs. experimental?
- Deploy or remove each contract
- Document deployment status

## Restoration Instructions

If you need to restore any removed feature:

```bash
# Find the commit where feature was removed
git log --all --full-history -- path/to/file

# Restore from that commit
git checkout <commit-hash>^ -- path/to/file
```
EOF

echo "  ✓ Created REMOVED_FEATURES.md"

echo ""
echo "✅ Phase 2 Complete!"
echo ""
echo "Summary:"
echo "  - Removed ML stub files (4 files)"
echo "  - Removed game components (2 items)"
echo "  - Removed orphaned contract folders (6 folders)"
echo "  - Removed temporary files (5 files)"
echo ""
echo "⚠️  Manual decisions needed:"
echo "  1. SEP-24/31 proxy implementations - keep or remove?"
echo "  2. Telegram bot integration - keep or remove?"
echo "  3. Governance/analytics contracts - deploy or remove?"
echo ""
echo "See REMOVED_FEATURES.md for details."
echo ""
echo "Next: Update Cargo.toml and package.json to remove unused dependencies"
