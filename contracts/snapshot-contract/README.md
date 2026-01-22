# Snapshot Contract

On-chain analytics snapshot verification for Stellar Insights using Soroban smart contracts.

## Features

✅ **Snapshot Submission** - Submit 32-byte SHA-256 hashes with epoch identifiers  
✅ **Input Validation** - Enforces hash size and epoch requirements  
✅ **Event Emission** - Publishes `SNAP_SUB` event for each submission  
✅ **Duplicate Prevention** - Immutable snapshots per epoch  
✅ **Snapshot Retrieval** - Query by epoch or get latest  
✅ **Hash Verification** - Verify integrity of stored snapshots  

## Quick Start

```bash
# Build contract
cargo build --target wasm32-unknown-unknown --release

# Run tests
cargo test --lib

# Deploy to testnet
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/snapshot_contract.wasm \
  --source ACCOUNT_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

## Contract Interface

### `submit_snapshot(hash: Bytes, epoch: u64) -> u64`
Submit analytics snapshot hash.

**Requirements:**
- Hash must be exactly 32 bytes (SHA-256)
- Epoch must be greater than 0
- Epoch must not already exist

**Returns:** Ledger timestamp  
**Event:** `SNAP_SUB(hash, epoch, timestamp)`

### `get_snapshot(epoch: u64) -> Bytes`
Retrieve snapshot hash for specific epoch.

### `latest_snapshot() -> (Bytes, u64, u64)`
Get most recent snapshot data.

**Returns:** `(hash, epoch, timestamp)`

### `verify_snapshot(hash: Bytes) -> bool`
Check if hash exists in stored snapshots.

## Testing

All 9 tests pass:
- ✅ Submit and retrieve snapshots
- ✅ Event emission verification
- ✅ Invalid hash size rejection
- ✅ Invalid epoch rejection  
- ✅ Duplicate epoch prevention
- ✅ Multiple snapshots handling
- ✅ Latest snapshot retrieval
- ✅ Hash verification (positive/negative)

```bash
cargo test --lib
```

## Integration Example

```rust
use sha2::{Sha256, Digest};
use soroban_sdk::Bytes;

// Generate snapshot hash
let mut hasher = Sha256::new();
hasher.update(analytics_data.as_bytes());
let hash: [u8; 32] = hasher.finalize().into();

// Submit to contract
let timestamp = client.submit_snapshot(
    &Bytes::from_array(&env, &hash),
    &epoch_number
);
```

## Security

- **Hash Validation**: Only SHA-256 (32 bytes) accepted
- **Epoch Validation**: No zero epochs allowed
- **Immutability**: Snapshots cannot be overwritten
- **Persistent Storage**: Long-term data retention

## Documentation

See [DEPLOYMENT.md](./DEPLOYMENT.md) for comprehensive deployment and integration guide.

## Built With

- Soroban SDK v21.7.7
- Rust (no_std)
- WASM target: 9.1K optimized
