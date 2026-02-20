## Description
<!-- Provide a brief description of the changes -->
Adds account-merge ingestion and API support in the backend, includes tests and docs, and fixes frontend dashboard/backend connectivity issues (including `8081` backend fallback and sidebar runtime error fix).

## Type of Change
<!-- Mark the relevant option with an "x" -->

- [x] 🐛 Bug fix (non-breaking change which fixes an issue)
- [x] ✨ New feature (non-breaking change which adds functionality)
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [x] 📝 Documentation update
- [x] 🎨 Style/UI update
- [ ] ♻️ Code refactoring
- [ ] ⚡ Performance improvement
- [x] ✅ Test update

## Related Issue
<!-- Link to the issue this PR addresses -->
https://github.com/Ndifreke000/stellar-insights/issues/227
Closes #227

## Changes Made
<!-- List the specific changes made in this PR -->

- Added account merge detector service, ingestion wiring, persistence migration, and `/api/account-merges` endpoints.
- Added backend tests for account merge persistence/query behavior and updated backend docs/README coverage.
- Fixed frontend runtime issues (`Database` icon import) and made dashboard API route automatically resolve backend URL across env + `8080/8081` fallbacks.

## Testing
<!-- Describe the tests you ran and how to reproduce them -->

### Backend
```bash
cd backend
cargo test -q --test account_merge_test
cargo test -q
```

### Frontend
```bash
cd frontend
npm run -s lint
```

### Contracts
```bash
cd contracts
cargo test
```

## Screenshots
<!-- If applicable, add screenshots to help explain your changes -->

## Checklist
<!-- Mark completed items with an "x" -->

- [x] My code follows the project's style guidelines
- [x] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [x] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [x] I have added tests that prove my fix is effective or that my feature works
- [x] New and existing unit tests pass locally with my changes
- [ ] Any dependent changes have been merged and published

## Additional Notes
