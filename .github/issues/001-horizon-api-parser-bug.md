---
title: "[BUG] Horizon API Parser Fails with New Soroban-Compatible Format"
labels: bug, critical, rpc, stellar, backend
assignees: ''
---

## 🐛 Description

The current `Payment` struct in the RPC client cannot parse the new Horizon API response format that includes Soroban-compatible fields. The new format uses `asset_balance_changes` instead of individual `destination`, `amount`, `asset_code` fields, causing all real RPC calls to fail.

## 📊 Current Behavior

When `RPC_MOCK_MODE=false`, the backend fails to parse payment responses:
```
Error: Failed to parse payments response
```

This forces us to use mock mode, preventing real-time data from the Stellar network.

## ✅ Expected Behavior

The RPC client should successfully parse both legacy and new Soroban-compatible payment response formats from Horizon API.

## 📁 Affected Files

- `backend/src/rpc/stellar.rs` (lines 68-79, Payment struct)
- `backend/src/api/anchors_cached.rs` (uses fetch_account_payments)
- `backend/src/api/corridors_cached.rs` (uses fetch_payments)

## 🔧 Technical Details

**Current Payment Struct:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub paging_token: String,
    pub transaction_hash: String,
    pub source_account: String,
    pub destination: String,
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    pub amount: String,
    pub created_at: String,
}
```

**New Horizon API Format:**
```json
{
  "id": "...",
  "paging_token": "...",
  "transaction_hash": "...",
  "source_account": "...",
  "asset_balance_changes": [
    {
      "asset_type": "credit_alphanum4",
      "asset_code": "USDC",
      "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      "type": "transfer",
      "from": "GXXXXXXX...",
      "to": "GDYYYYYY...",
      "amount": "100.0000000"
    }
  ],
  "created_at": "2026-02-02T10:30:00Z"
}
```

## 📋 Acceptance Criteria

- [ ] Update `Payment` struct to include `asset_balance_changes` field
- [ ] Create `AssetBalanceChange` struct with required fields
- [ ] Add helper methods to extract destination and amount from either format
- [ ] Update all payment parsing call sites
- [ ] Add unit tests for new format parsing
- [ ] Test with real Horizon API (testnet)
- [ ] Update mock data to match new format
- [ ] Document the new structure in code comments

## 🚀 Implementation Steps

1. Add new struct `AssetBalanceChange` with fields: `asset_type`, `asset_code`, `asset_issuer`, `from`, `to`, `amount`, `type`
2. Add `asset_balance_changes: Option<Vec<AssetBalanceChange>>` to `Payment`
3. Create helper method `Payment::get_destination()` that checks both old and new format
4. Create helper method `Payment::get_amount()` with same logic
5. Update all call sites to use helper methods
6. Add integration test with real Horizon API call

## 📚 References

- Horizon API Documentation: https://developers.stellar.org/docs/data/horizon
- Soroban Documentation: https://soroban.stellar.org/
- Related: `docs/RPC_INTEGRATION_SUMMARY.md`

## 🏷️ Priority

**Critical** - Blocks real-time data integration with Stellar network

## 🌍 Environment

- Backend: Rust with Axum
- RPC Client: `backend/src/rpc/stellar.rs`
- Current workaround: `RPC_MOCK_MODE=true` in `backend/.env`
