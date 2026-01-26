# Stellar Insights Analytics Snapshot Contract

A Soroban smart contract for submitting and verifying cryptographic hashes of analytics snapshots on the Stellar blockchain.

## Overview

This contract enables authorized administrators to submit SHA-256 hashes of analytics data snapshots on-chain, creating an immutable audit trail. Each snapshot is associated with an epoch identifier and emits an event for off-chain verification.

## Features

- **Admin Authorization**: Only designated admin can submit snapshots
- **Epoch-based Storage**: Snapshots organized by epoch number
- **Duplicate Prevention**: Each epoch can only have one snapshot
- **Event Emission**: All submissions emit verifiable events
- **Efficient Retrieval**: Get snapshots by epoch or retrieve latest
- **Type-safe Errors**: Comprehensive error handling with Soroban's error system

## Contract Functions

### Initialize
```rust
fn initialize(env: Env, admin: Address)
```
Sets up the contract with an authorized admin address. Can only be called once.

### Submit Snapshot
```rust
fn submit_snapshot(
    env: Env,
    epoch: u64,
    hash: BytesN<32>,
    caller: Address
) -> Result<u64, Error>
```
Submits a snapshot hash for a specific epoch. Only callable by admin.

**Parameters:**
- `epoch`: Unique epoch identifier (must be > 0)
- `hash`: 32-byte SHA-256 hash of analytics data
- `caller`: Address attempting submission (must be admin)

**Returns:** Ledger timestamp of submission

**Errors:**
- `UnauthorizedCaller`: Caller is not the admin
- `InvalidEpoch`: Epoch is zero
- `DuplicateEpoch`: Snapshot already exists for this epoch
- `AdminNotSet`: Contract not initialized

### Get Snapshot
```rust
fn get_snapshot(env: Env, epoch: u64) -> Result<BytesN<32>, Error>
```
Retrieves the snapshot hash for a specific epoch.

### Latest Snapshot
```rust
fn latest_snapshot(env: Env) -> Result<(BytesN<32>, u64, u64), Error>
```
Returns the most recent snapshot as `(hash, epoch, timestamp)`.

### Get Admin
```rust
fn get_admin(env: Env) -> Result<Address, Error>
```
Returns the current admin address.

### Get Latest Epoch
```rust
fn get_latest_epoch(env: Env) -> u64
```
Returns the highest epoch number recorded (0 if none).

## Events

### AnalyticsSnapshotSubmitted
Emitted when a snapshot is successfully submitted.

**Fields:**
- `epoch`: u64 - Epoch identifier
- `hash`: BytesN<32> - Snapshot hash
- `timestamp`: u64 - Ledger timestamp

**Topic:** `SNAP_SUB`

## Error Codes

| Error | Code | Description |
|-------|------|-------------|
| UnauthorizedCaller | 1 | Caller is not authorized |
| DuplicateEpoch | 2 | Epoch already has a snapshot |
| InvalidHashSize | 3 | Hash is not 32 bytes |
| InvalidEpoch | 4 | Epoch must be > 0 |
| AdminNotSet | 5 | Contract not initialized |
| SnapshotNotFound | 6 | No snapshot for epoch |

## Building

```bash
cd contracts/stellar_insights
cargo build --target wasm32-unknown-unknown --release
```

## Testing

Run all tests:
```bash
cargo test
```

Run specific test:
```bash
cargo test test_successful_snapshot_submission
```

Run with output:
```bash
cargo test -- --nocapture
```

## Deployment

1. Build the contract:
```bash
cargo build --target wasm32-unknown-unknown --release
```

2. Optimize the WASM:
```bash
soroban contract optimize \
    --wasm target/wasm32-unknown-unknown/release/stellar_insights.wasm
```

3. Deploy to testnet:
```bash
soroban contract deploy \
    --wasm target/wasm32-unknown-unknown/release/stellar_insights.wasm \
    --source <YOUR_SECRET_KEY> \
    --rpc-url https://soroban-testnet.stellar.org:443 \
    --network-passphrase "Test SDF Network ; September 2015"
```

4. Initialize with admin:
```bash
soroban contract invoke \
    --id <CONTRACT_ID> \
    --source <ADMIN_SECRET_KEY> \
    --rpc-url https://soroban-testnet.stellar.org:443 \
    --network-passphrase "Test SDF Network ; September 2015" \
    -- initialize \
    --admin <ADMIN_PUBLIC_KEY>
```

## Usage Example

```rust
use soroban_sdk::{Address, BytesN, Env};
use stellar_insights::StellarInsightsContractClient;

// Initialize client
let client = StellarInsightsContractClient::new(&env, &contract_id);

// Initialize contract
client.initialize(&admin_address);

// Submit snapshot
let epoch = 1u64;
let hash = compute_sha256_hash(analytics_data);
let timestamp = client.submit_snapshot(&epoch, &hash, &admin_address);

// Retrieve snapshot
let retrieved_hash = client.get_snapshot(&epoch);

// Get latest
let (hash, epoch, timestamp) = client.latest_snapshot();
```

## Security Considerations

1. **Admin Key Security**: The admin private key must be securely stored. Loss or compromise of this key affects the entire system.

2. **Epoch Management**: Ensure epochs are generated sequentially or with a clear ordering strategy to maintain data integrity.

3. **Hash Verification**: While the contract stores hashes on-chain, off-chain systems must verify the hash computation is correct.

4. **Immutability**: Once submitted, snapshots cannot be modified or deleted. Ensure data accuracy before submission.

5. **Gas Costs**: Each submission consumes network resources. Plan submission frequency accordingly.

## Architecture

```
┌─────────────────────────────────────┐
│   Backend Analytics System          │
│                                     │
│  1. Aggregate analytics data        │
│  2. Compute SHA-256 hash            │
│  3. Sign transaction as admin       │
│  4. Submit to contract              │
└─────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│   Stellar Insights Contract         │
│                                     │
│  - Verify admin authorization       │
│  - Check epoch uniqueness           │
│  - Store hash on-chain              │
│  - Emit verification event          │
└─────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│   Stellar Blockchain                │
│                                     │
│  - Immutable storage                │
│  - Event log for verification       │
│  - Public auditability              │
└─────────────────────────────────────┘
```

## License

This contract is part of the Stellar Insights project.

## Development

### Prerequisites
- Rust 1.70+
- Soroban CLI
- `wasm32-unknown-unknown` target

### Setup
```bash
rustup target add wasm32-unknown-unknown
cargo install soroban-cli
```

### Code Structure
```
src/
├── lib.rs        # Main contract implementation
├── errors.rs     # Error definitions
├── events.rs     # Event structures
└── test.rs       # Unit tests

tests/
└── snapshot_test.rs  # Integration tests
```

## Acceptance Criteria

✅ Only admin can submit snapshots  
✅ Snapshots are stored and retrievable by epoch  
✅ Latest snapshot can be retrieved  
✅ Events emitted on successful submission  
✅ Unauthorized callers are rejected  
✅ Duplicate epochs are prevented  
✅ All tests pass  
✅ Comprehensive error handling  
✅ Production-ready code quality  

## Contributing

When contributing, ensure:
- All tests pass
- Code follows Rust best practices
- Documentation is updated
- Error handling is comprehensive
- Events are properly emitted
