---
name: Fix Horizon API Parser for Soroban-Compatible Format
about: Payment struct doesn't handle new asset_balance_changes field
title: '[BUG] Horizon API Parser Fails with New Soroban Format'
labels: bug, priority:critical, backend, rpc
assignees: ''
---

## Description

The Horizon API has updated its response format to be Soroban-compatible, using `asset_balance_changes` instead of individual `destination`, `amount`, `asset_code` fields. Our current `Payment` struct in the RPC client cannot parse this new format, causing all real RPC calls to fail.

## Current Behavior

When `RPC_MOCK_MODE=false`, the backend fails to parse payment responses from Horizon API:
```
Error: Failed to parse payments response
```

This forces us to use mock mode, preventing real-time data from the Stellar network.

## Expected Behavior

The RPC client should successfully parse both legacy and new Soroban-compatible payment response formats from Horizon API.

## Technical Details

**Affected File:** `backend/src/rpc/stellar.rs`

**Current Payment Struct (lines 68-79):**
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
      "from": "GXXXXXXX...",
      "to": "GDYYYYYY...",
      "amount": "100.0000000"
    }
  ],
  "created_at": "2026-02-02T10:30:00Z"
}
```

## Proposed Solution

1. Update `Payment` struct to handle both formats:
   - Add `asset_balance_changes` field as `Option<Vec<AssetBalanceChange>>`
   - Keep legacy fields for backward compatibility
   - Add custom deserializer or use `#[serde(flatten)]`

2. Create new struct for asset balance changes:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalanceChange {
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    pub from: String,
    pub to: String,
    pub amount: String,
}
```

3. Update parsing logic in:
   - `fetch_payments()` (line 234)
   - `fetch_account_payments()` (line 280)
   - `fetch_payments_for_ledger()` (line 260)

4. Add helper method to extract destination and amount from either format

## Testing

- [ ] Test with real Horizon API (`RPC_MOCK_MODE=false`)
- [ ] Verify backward compatibility with legacy format
- [ ] Test all payment-related endpoints:
  - `/api/anchors`
  - `/api/corridors`
- [ ] Add unit tests for both response formats

## References

- Horizon API Documentation: https://developers.stellar.org/docs/data/horizon
- Soroban Documentation: https://soroban.stellar.org/
- Related: `docs/RPC_INTEGRATION_SUMMARY.md`

## Environment

- Backend: Rust with Axum
- RPC Client: `backend/src/rpc/stellar.rs`
- Current workaround: `RPC_MOCK_MODE=true` in `backend/.env`

## Priority

**Critical** - Blocks real-time data integration with Stellar network
