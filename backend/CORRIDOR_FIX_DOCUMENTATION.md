# Corridor Detection Bug Fix - Documentation

## Overview
Fixed the corridor detection logic to properly identify actual asset pairs from payment operations instead of assuming all destinations are XLM.

## Problem Statement
The previous implementation had a hardcoded assumption that all corridor destinations were XLM (native), which caused:
- Incorrect corridor identification
- Missing cross-asset corridors (USDC->EUR, etc.)
- Inaccurate corridor metrics
- Inability to track real payment paths

## Solution

### 1. Enhanced Payment Struct
**File**: `backend/src/rpc/stellar.rs`

Added support for path payment operations by extending the `Payment` struct with:
- `operation_type`: Identifies the type of operation (payment, path_payment_strict_send, path_payment_strict_receive)
- `source_asset_type`, `source_asset_code`, `source_asset_issuer`: Source asset information for path payments
- `source_amount`: Amount sent in source asset for path payments
- `from`, `to`: Explicit sender and receiver fields

```rust
pub struct Payment {
    // ... existing fields ...
    
    // Path payment fields
    #[serde(rename = "type")]
    pub operation_type: Option<String>,
    pub source_asset_type: Option<String>,
    pub source_asset_code: Option<String>,
    pub source_asset_issuer: Option<String>,
    pub source_amount: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}
```

### 2. Asset Pair Extraction Logic
**File**: `backend/src/api/corridors_cached.rs`

Created a new `AssetPair` struct and `extract_asset_pair_from_payment()` function that:
- Detects operation type (regular payment vs path payment)
- Extracts correct source and destination assets based on operation type
- Handles native XLM and issued assets
- Returns None for malformed payments

#### Operation Type Handling:

**Regular Payments** (`payment`):
- Source and destination use the same asset
- Asset info from `asset_type`, `asset_code`, `asset_issuer` fields

**Path Payments** (`path_payment_strict_send`, `path_payment_strict_receive`):
- Source asset from `source_asset_type`, `source_asset_code`, `source_asset_issuer`
- Destination asset from `asset_type`, `asset_code`, `asset_issuer`
- Supports cross-asset corridors (USD->EUR, XLM->USDC, etc.)

### 3. Updated Corridor Detection
**File**: `backend/src/api/corridors_cached.rs`

Modified `list_corridors()` function to:
- Use `extract_asset_pair_from_payment()` for each payment
- Group payments by actual asset pairs
- Log warnings for payments that can't be parsed
- Support all asset pair combinations

### 4. Enhanced Mock Data
**File**: `backend/src/rpc/stellar.rs`

Updated `mock_payments()` to generate diverse test data:
- Mix of regular payments and path payments (20% path payments)
- Various asset combinations (USDC, EURT, BRL, NGNT, etc.)
- Native XLM and issued assets
- Cross-asset corridors for testing

## Test Coverage

Added comprehensive unit tests in `backend/src/api/corridors_cached.rs`:

1. **test_extract_asset_pair_regular_payment_native**: XLM->XLM payments
2. **test_extract_asset_pair_regular_payment_issued_asset**: USDC->USDC payments
3. **test_extract_asset_pair_path_payment_cross_asset**: USD->EUR path payments
4. **test_extract_asset_pair_path_payment_native_to_issued**: XLM->USDC path payments
5. **test_extract_asset_pair_path_payment_issued_to_native**: BRL->XLM path payments
6. **test_extract_asset_pair_missing_operation_type**: Fallback to regular payment behavior

## API Response Changes

### Before:
```json
{
  "id": "USDC:GISSUER->XLM:native",
  "source_asset": "USDC",
  "destination_asset": "XLM",
  ...
}
```

### After:
```json
[
  {
    "id": "USDC:GISSUER->USDC:GISSUER",
    "source_asset": "USDC",
    "destination_asset": "USDC",
    ...
  },
  {
    "id": "USD:GUSDISSUER->EUR:GEURISSUER",
    "source_asset": "USD",
    "destination_asset": "EUR",
    ...
  },
  {
    "id": "XLM:native->USDC:GISSUER",
    "source_asset": "XLM",
    "destination_asset": "USDC",
    ...
  }
]
```

## Supported Corridor Types

1. **Same-asset corridors**: USDC->USDC, XLM->XLM
2. **Cross-asset corridors**: USD->EUR, USDC->BRL
3. **Native to issued**: XLM->USDC, XLM->EUR
4. **Issued to native**: USDC->XLM, BRL->XLM
5. **Issued to issued**: USDC->EUR, USD->BRL

## Backward Compatibility

- Existing API endpoints remain unchanged
- Response structure is identical
- Only the corridor identification logic improved
- Old clients will see more accurate corridor data

## Error Handling

- Malformed payments are logged with warnings
- Missing operation_type defaults to regular payment
- Invalid asset pairs are skipped gracefully
- No breaking changes to API contract

## Performance Considerations

- Asset pair extraction is O(1) per payment
- No additional API calls required
- Caching strategy remains unchanged
- Memory footprint minimal (AssetPair is lightweight)

## Future Enhancements

1. Track intermediate assets in path payments
2. Calculate conversion rates for cross-asset corridors
3. Add path payment specific metrics (slippage, path length)
4. Support for liquidity pool operations
5. Historical corridor evolution tracking

## Testing Instructions

### Run Unit Tests
```bash
cd backend
cargo test api::corridors_cached::tests
```

### Manual Testing
1. Start the backend server
2. Call `/api/corridors` endpoint
3. Verify diverse asset pairs in response
4. Check for cross-asset corridors (not just XLM destinations)

### Expected Results
- Multiple corridor types visible
- Accurate source and destination assets
- Path payments correctly identified
- No "all destinations are XLM" pattern

## Files Modified

1. `backend/src/rpc/stellar.rs`
   - Enhanced Payment struct
   - Updated mock_payments() function

2. `backend/src/api/corridors_cached.rs`
   - Added AssetPair struct
   - Added extract_asset_pair_from_payment() function
   - Updated list_corridors() function
   - Added comprehensive unit tests

## Acceptance Criteria - Status

✅ Parse destination asset from payment data
✅ Detect all unique asset pairs from payment data
✅ Handle path payments correctly
✅ Support native XLM and issued assets
✅ Create corridors for all detected pairs
✅ Add tests with various asset combinations
✅ Update documentation

## References

- [Stellar Payment Object Documentation](https://developers.stellar.org/docs/data/horizon/api-reference/resources/payments/object)
- [Path Payment Strict Send](https://developers.stellar.org/api/resources/operations/object/path-payment-strict-send/)
- [Path Payment Strict Receive](https://developers.stellar.org/api/resources/operations/object/path-payment-strict-receive/)
