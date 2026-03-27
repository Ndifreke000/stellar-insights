# Event Filtering Implementation Guide

## Overview

This Stellar smart contract implements proper event filtering support using topics and metadata. Events can now be efficiently queried by specific criteria without scanning all events.

## Event Structure

### Topics (Indexed, Filterable)

Events are published with up to 3 topics that enable efficient filtering:

- **Topic 0**: Event type marker (e.g., `snapshot`, `admin_ch`, `pause`, `unpause`, `gov_set`)
- **Topic 1**: Primary entity (usually an Address or identifier)
- **Topic 2**: Secondary filter (usually epoch, sequence, or category)

### Data (Metadata)

The event struct contains complete data but is not indexed by default:

- `epoch`: The snapshot epoch number
- `hash`: The 32-byte hash
- `submitter`: Address of the submitter
- `timestamp`: Block timestamp
- `ledger_sequence`: Ledger sequence number

## Event Types Implemented

### 1. SnapshotSubmitted Event

**Topics:**

- Topic 0: `symbol_short!("snapshot")`
- Topic 1: `caller` (submitter address)
- Topic 2: `epoch` (epoch number)

**Data:**

```rust
SnapshotSubmittedEvent {
    epoch,
    hash,
    submitter,
    timestamp,
    ledger_sequence,
}
```

**Use Cases:**

- Query all snapshots from a specific submitter
- Query all snapshots in a specific epoch
- Query all snapshots across epochs

### 2. AdminChanged Event

**Topics:**

- Topic 0: `symbol_short!("admin_ch")`
- Topic 1: `new_admin` (new admin address)

### 3. ContractPaused Event

**Topics:**

- Topic 0: `symbol_short!("pause")`

### 4. ContractUnpaused Event

**Topics:**

- Topic 0: `symbol_short!("unpause")`

### 5. GovernanceSet Event

**Topics:**

- Topic 0: `symbol_short!("gov_set")`
- Topic 1: `governance_address`

## Query Examples

### Query events by topic using Stellar CLI

```bash
# Query all snapshot events
stellar contract events --id <CONTRACT_ID> \
  --topic snapshot \
  --network testnet

# Query snapshots from a specific submitter
stellar contract events --id <CONTRACT_ID> \
  --topic snapshot \
  --topic <SUBMITTER_ADDRESS> \
  --network testnet

# Query snapshots for a specific epoch
stellar contract events --id <CONTRACT_ID> \
  --topic snapshot \
  --topic <SUBMITTER_ADDRESS> \
  --topic <EPOCH_NUMBER> \
  --network testnet

# Query admin change events for specific admin
stellar contract events --id <CONTRACT_ID> \
  --topic admin_ch \
  --topic <NEW_ADMIN_ADDRESS> \
  --network testnet
```

### Query events using JavaScript SDK

```javascript
const SorobanClient = require("stellar-sdk");

const server = new SorobanClient.Server("https://soroban-testnet.stellar.org");

// Query snapshots from specific submitter
const events = await server.getEvents({
  filters: [
    {
      type: "contract",
      contractId: "<CONTRACT_ID>",
      topics: [
        "AAAADwAAAA5zbmFwc2hvdA==", // snapshot (base64 encoded symbol)
        "<SUBMITTER_ADDRESS_XDR>",
      ],
    },
  ],
});
```

## Performance Benefits

1. **Faster Queries**: Topics are indexed, enabling O(1) lookups instead of O(n) full scans
2. **Reduced Bandwidth**: Filter events on-chain instead of retrieving all events
3. **Better Scalability**: Supports high-volume event streams
4. **Developer Experience**: Clear, standardized event filtering patterns

## Implementation Checklist

- ✅ EventType enum for standardization
- ✅ Topic-based event emission (up to 3 topics per event)
- ✅ SnapshotSubmittedEvent with filtering metadata
- ✅ Admin, pause/unpause, and governance events
- ✅ Event struct with complete metadata
- ✅ Tests for event emission
- ✅ Query examples and documentation

## Verification Steps

1. **Build the contract:**

```bash
cargo build --target wasm32-unknown-unknown --release
```

2. **Deploy to testnet:**

```bash
stellar contract deploy --wasm target/wasm32-unknown-unknown/release/stellar_snapshot_contract.wasm \
  --source-account <YOUR_ACCOUNT> \
  --network testnet
```

3. **Submit a snapshot:**

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source-account <YOUR_ACCOUNT> \
  --fn submit_snapshot \
  --arg <EPOCH> \
  --arg <HASH> \
  --arg <CALLER_ADDRESS> \
  --network testnet
```

4. **Query events by topic:**

```bash
stellar contract events --id <CONTRACT_ID> \
  --topic snapshot \
  --topic <SUBMITTER_ADDRESS> \
  --network testnet
```

## Notes

- All events are automatically indexed by topics for efficient querying
- Event data is stored in the event metadata (not indexed but queryable)
- Topics use `symbol_short!()` macro for efficient encoding
- Events are immutable and permanent on the ledger
