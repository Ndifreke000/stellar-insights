## Description
<!-- Provide a brief description of the changes -->
Fixes backend compile breakages and hardens Stellar RPC/Horizon deserialization so testnet response format differences do not fail parsing.

## Type of Change
<!-- Mark the relevant option with an "x" -->

- [x] 🐛 Bug fix (non-breaking change which fixes an issue)
- [ ] ✨ New feature (non-breaking change which adds functionality)
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] 📝 Documentation update
- [ ] 🎨 Style/UI update
- [x] ♻️ Code refactoring
- [ ] ⚡ Performance improvement
- [ ] ✅ Test update

## Related Issue
<!-- Link to the issue this PR addresses -->
https://github.com/Ndifreke000/stellar-insights/issues/214
Closes #214

## Changes Made
<!-- List the specific changes made in this PR -->

- Fixed backend compile blockers across model and service alignment.
- Removed duplicate `get_muxed_analytics` function definition to avoid duplicate symbol/build errors.
- Updated realtime broadcaster code paths to match current model/database signatures.
- Hardened `backend/src/rpc/stellar.rs` deserialization:
  - accepts string-or-number for numeric fields
  - adds defaults for optional/missing fields in Horizon payloads
  - prevents testnet parse failures for ledger/payment/trade/liquidity-pool responses

## Testing
<!-- Describe the tests you ran and how to reproduce them -->

### Backend
```bash
cd backend
cargo check -q
cargo test -q rpc::stellar::tests::test_mock_fetch_payments
```

### Frontend
```bash
Not run (backend-only changes)
```

### Contracts
```bash
Not run (backend-only changes)
```

## Screenshots
<!-- If applicable, add screenshots to help explain your changes -->
N/A

## Checklist
<!-- Mark completed items with an "x" -->

- [x] My code follows the project's style guidelines
- [x] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [x] New and existing unit tests pass locally with my changes
- [ ] Any dependent changes have been merged and published

## Additional Notes
- This PR is backend-only and focused on stability/compatibility for RPC/Horizon payload handling.
