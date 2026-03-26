# Integration Tests for Soroban Contracts

This directory contains comprehensive integration tests for the Soroban smart contracts, including Analytics and Access Control contracts.

## Project Structure

```
contracts/
├── Cargo.toml                 # Workspace root
├── analytics/                 # Analytics contract
│   ├── Cargo.toml
│   └── src/lib.rs
├── access_control/            # Access Control contract
│   ├── Cargo.toml
│   └── src/lib.rs
└── tests/
    └── integration_test.rs     # Integration tests
```

## Running Tests

### Run all integration tests:

```bash
cd contracts
cargo test --test integration_test
```

### Run specific test:

```bash
cargo test --test integration_test test_analytics_with_access_control
```

### Run tests with output:

```bash
cargo test --test integration_test -- --nocapture
```

### Run with detailed output:

```bash
cargo test --test integration_test -- --nocapture --test-threads=1
```

## Test Coverage

### Test 1: Analytics with Access Control Integration

- Deploys both contracts
- Initializes with admin
- Grants Submitter role to user
- Verifies user can submit snapshots
- Validates snapshot retrieval

### Test 2: Multiple Snapshots and Role Management

- Tests multiple concurrent submitters
- Verifies multiple snapshot submissions
- Validates role-based access for multiple users
- Ensures all snapshots are retrievable

### Test 3: Role Revocation Prevents Operations

- Tests role granting and revocation lifecycle
- Verifies successful operations with role
- Confirms role revocation takes effect
- Validates access denial after revocation

### Test 4: Admin Operations

- Tests admin role delegation
- Verifies secondary admin can grant roles
- Validates hierarchical role management

### Test 5: Concurrent Snapshot Submissions

- Tests high concurrency scenarios
- 5 submitters × 10 snapshots each = 50 total submissions
- Validates all snapshots are correctly stored
- Verifies retrieval of first and last snapshots

### Test 6: Edge Cases and Error Handling

- Tests retrieval of non-existent snapshots
- Verifies behavior for users without roles
- Tests handling of large snapshot IDs (near u64::MAX)

### Test 7: Contract State Persistence

- Verifies snapshots persist after submission
- Tests state persistence across client instances
- Confirms storage layer functionality

## Quality Assurance Coverage

✅ **Contract Interactions**: Full end-to-end contract communication
✅ **End-to-End Validation**: Complete workflow testing
✅ **Real-World Scenarios**: Multi-user, role-based access control
✅ **Error Handling**: Edge cases and boundary conditions
✅ **State Persistence**: Storage and retrieval verification
✅ **Concurrency**: Multiple simultaneous operations

## Dependencies

- `soroban-sdk` = "21.6.0" with testutils feature
- Rust edition 2021

## Building

```bash
# Build all contracts
cargo build --release

# Build specific contract
cargo build --release -p analytics
cargo build --release -p access_control
```

## Test Results

All tests validate:

- ✓ Snapshot submission and retrieval
- ✓ Role-based access control
- ✓ Admin operations
- ✓ Error handling
- ✓ State persistence
- ✓ Concurrent operations

## Troubleshooting

If tests fail to compile:

1. Ensure Rust is up to date: `rustup update`
2. Clear build artifacts: `cargo clean`
3. Rebuild: `cargo build`

If tests fail to run:

1. Check test output with `--nocapture` flag
2. Enable verbose output with `--test-threads=1`
3. Review test logic and contract implementations
