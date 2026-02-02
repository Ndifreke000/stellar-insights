---
title: "[BUG] Incomplete Corridor Asset Pair Detection Logic"
labels: bug, high, corridors, rpc, backend
assignees: ''
---

## 🐛 Description

The current corridor detection logic in `list_corridors()` assumes the destination asset is always XLM (native). This is a known limitation documented in the code with a TODO comment. This causes incorrect corridor identification and missing data for non-XLM destination corridors.

## 📊 Current Behavior

**Current Code (lines 76-79 in `backend/src/api/corridors_cached.rs`):**
```rust
// For now, assume destination is XLM (we'd need more data to determine actual destination asset)
let asset_to = "XLM:native".to_string();
```

**Problems:**
- All corridors show XLM as destination
- Missing actual cross-asset corridors (USDC->EUR, etc.)
- Inaccurate corridor metrics
- Cannot track real payment paths

## ✅ Expected Behavior

- Detect actual destination asset from payment data
- Support all asset pair combinations
- Accurately identify corridor paths
- Handle path payments with intermediate assets

## 📁 Affected Files

- `backend/src/api/corridors_cached.rs` (lines 60-110)
- `backend/src/rpc/stellar.rs` (Payment struct)

## 🔍 Root Cause

The Horizon API payment response doesn't always include explicit destination asset information. Need to:
1. Parse `asset_balance_changes` to find receiving account's asset
2. Handle path payments that go through multiple assets
3. Track both direct and indirect corridors

## 📋 Acceptance Criteria

- [ ] Parse destination asset from `asset_balance_changes`
- [ ] Detect all unique asset pairs from payment data
- [ ] Handle path payments correctly
- [ ] Support native XLM and issued assets
- [ ] Create corridors for all detected pairs
- [ ] Add tests with various asset combinations
- [ ] Update documentation

## 🚀 Implementation Steps

1. Update `Payment` struct to include destination asset info
2. Create helper function `extract_asset_pairs()` from payment
3. Handle multiple balance changes in single payment
4. Identify source and destination from balance change types
5. Update corridor grouping logic
6. Add validation for asset pair format
7. Test with real multi-asset payments

## 💻 Technical Approach

```rust
fn extract_corridor_from_payment(payment: &Payment) -> Option<(Asset, Asset)> {
    // Parse asset_balance_changes
    let balance_changes = payment.asset_balance_changes.as_ref()?;
    
    // Find 'transfer' type changes
    let mut source_asset = None;
    let mut dest_asset = None;
    
    for change in balance_changes {
        let amount: f64 = change.amount.parse().ok()?;
        
        if amount < 0.0 {
            // Negative amount = source
            source_asset = Some(Asset {
                code: change.asset_code.clone(),
                issuer: change.asset_issuer.clone(),
            });
        } else if amount > 0.0 {
            // Positive amount = destination
            dest_asset = Some(Asset {
                code: change.asset_code.clone(),
                issuer: change.asset_issuer.clone(),
            });
        }
    }
    
    Some((source_asset?, dest_asset?))
}
```

## 🧪 Test Cases Needed

- [ ] Direct XLM payment
- [ ] Direct issued asset payment (USDC->USDC)
- [ ] Cross-asset payment (USDC->EUR)
- [ ] Path payment through multiple assets
- [ ] Failed payment handling
- [ ] Multiple balance changes in single payment

## 🏷️ Priority

**High** - Causes incorrect corridor data and missing corridors

## 🔗 Related Issues

- Depends on #1 (Horizon API Parser) being fixed first
- Blocks accurate corridor metrics
