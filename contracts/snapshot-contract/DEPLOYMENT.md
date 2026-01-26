# Snapshot Contract Deployment Guide

## Overview
The Snapshot Contract provides on-chain verification for analytics snapshots on the Stellar blockchain using Soroban smart contracts.

## Contract Features

### Core Functionality
- **Snapshot Submission**: Submit analytics snapshot hashes with epoch identifiers
- **Input Validation**: Enforces 32-byte hash size and non-zero epochs
- **Duplicate Prevention**: Prevents overwriting existing snapshots
- **Event Emission**: Emits `SNAP_SUB` event for each submission
- **Snapshot Retrieval**: Query snapshots by epoch
- **Latest Snapshot**: Get most recent snapshot data
- **Hash Verification**: Verify snapshot integrity

### Security Features
1. **Hash Size Validation**: Only accepts exactly 32 bytes (SHA-256)
2. **Epoch Validation**: Rejects epoch 0
3. **Immutability**: Snapshots cannot be overwritten once submitted
4. **Persistent Storage**: Uses Soroban's persistent storage tier

## Build Instructions

### Prerequisites
```bash
rustup target add wasm32-unknown-unknown
cargo install soroban-cli
```

### Build Optimized WASM
```bash
cd contracts/snapshot-contract
cargo build --target wasm32-unknown-unknown --release
```

Output: `target/wasm32-unknown-unknown/release/snapshot_contract.wasm` (9.1K)

### Run Tests
```bash
cargo test --lib
```

## Deployment

### 1. Deploy to Testnet
```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/snapshot_contract.wasm \
  --source ACCOUNT_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

### 2. Deploy to Mainnet
```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/snapshot_contract.wasm \
  --source ACCOUNT_SECRET_KEY \
  --rpc-url https://soroban-mainnet.stellar.org:443 \
  --network-passphrase "Public Global Stellar Network ; September 2015"
```

## Contract Interface

### submit_snapshot
Submit analytics snapshot hash for verification.

**Parameters:**
- `hash: Bytes` - 32-byte SHA-256 hash of analytics data
- `epoch: u64` - Epoch identifier (must be > 0)

**Returns:** `u64` - Ledger timestamp

**Panics:**
- Invalid hash size (not 32 bytes)
- Invalid epoch (zero)
- Duplicate epoch submission

**Event:** `SNAP_SUB(hash, epoch, timestamp)`

**Example:**
```rust
let hash = hex::decode("a1b2c3d4...").unwrap();
let timestamp = client.submit_snapshot(&hash, &1234);
```

### get_snapshot
Retrieve snapshot hash for a specific epoch.

**Parameters:**
- `epoch: u64` - Epoch identifier

**Returns:** `Bytes` - 32-byte hash

**Panics:** Epoch not found

### latest_snapshot
Get the most recent snapshot.

**Returns:** `(Bytes, u64, u64)` - (hash, epoch, timestamp)

**Panics:** No snapshots exist

### verify_snapshot
Check if a hash exists in stored snapshots.

**Parameters:**
- `hash: Bytes` - Hash to verify

**Returns:** `bool` - True if found

## Integration with Backend

### Generate Snapshot Hash
```rust
use sha2::{Sha256, Digest};

fn generate_snapshot_hash(data: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hasher.finalize().into()
}
```

### Submit from Backend
```rust
use soroban_sdk::Bytes;

async fn submit_analytics_snapshot(
    client: &SnapshotContractClient,
    metrics: &AnchorMetrics,
    epoch: u64,
) -> Result<u64> {
    let data = serde_json::to_string(metrics)?;
    let hash = generate_snapshot_hash(&data);
    
    let timestamp = client.submit_snapshot(
        &Bytes::from_array(&env, &hash),
        &epoch
    );
    
    Ok(timestamp)
}
```

## Testing

### Unit Tests
All 9 tests pass:
- ✅ Submit and retrieve
- ✅ Event emission
- ✅ Invalid hash size rejection
- ✅ Invalid epoch rejection
- ✅ Duplicate epoch prevention
- ✅ Multiple snapshots
- ✅ Latest snapshot retrieval
- ✅ Hash verification (found)
- ✅ Hash verification (not found)

### Manual Testing
```bash
# Deploy to testnet
CONTRACT_ID=$(soroban contract deploy ...)

# Submit snapshot
soroban contract invoke \
  --id $CONTRACT_ID \
  --source-account YOUR_ACCOUNT \
  -- \
  submit_snapshot \
  --hash $(echo "test" | sha256sum | cut -d' ' -f1) \
  --epoch 1

# Retrieve snapshot
soroban contract invoke \
  --id $CONTRACT_ID \
  -- \
  get_snapshot \
  --epoch 1
```

## Gas Optimization

### Current Size: 9.1K
The contract is highly optimized:
- Minimal dependencies
- No heap allocations
- Efficient storage patterns
- Single map structure

### Storage Costs
- Per snapshot: ~100 bytes
- Persistent tier: Long-term retention
- No TTL management required

## Monitoring

### Event Tracking
Listen for `SNAP_SUB` events:
```javascript
const events = await server
  .events()
  .forContract(contractId)
  .cursor('now')
  .stream({
    onmessage: (event) => {
      if (event.topic[0] === 'SNAP_SUB') {
        console.log('New snapshot:', event.value);
      }
    }
  });
```

### Health Checks
```rust
// Verify latest snapshot is recent
let (_, epoch, timestamp) = client.latest_snapshot();
let now = env.ledger().timestamp();
assert!(now - timestamp < MAX_AGE);
```

## Troubleshooting

### Common Issues

**"Invalid hash size"**
- Ensure hash is exactly 32 bytes
- Use SHA-256 for hashing

**"Invalid epoch"**
- Epoch must be > 0
- Use timestamps or sequential counters

**"Snapshot already exists"**
- Each epoch can only have one snapshot
- Increment epoch for new submissions

**"No snapshot found"**
- Verify epoch exists before retrieval
- Check contract initialization

## Security Considerations

1. **Access Control**: Currently open - consider adding authorization
2. **Rate Limiting**: Implement off-chain to prevent spam
3. **Epoch Management**: Backend must ensure monotonic epochs
4. **Hash Collisions**: SHA-256 makes this cryptographically infeasible

## Upgrade Path

For contract upgrades:
1. Test new version thoroughly
2. Deploy new contract
3. Update backend configuration
4. Maintain both contracts temporarily
5. Verify data integrity
6. Deprecate old contract

## Contract Address

After deployment, record the contract address:
- **Testnet**: `CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX`
- **Mainnet**: `CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX`

Update backend configuration in `.env`:
```env
SNAPSHOT_CONTRACT_ADDRESS=CXXXXXXXXXXXXX
STELLAR_NETWORK=testnet  # or mainnet
```
