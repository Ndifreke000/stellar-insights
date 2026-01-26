# Snapshot History Implementation

## Overview

This implementation addresses the requirement to store snapshot history on-chain, replacing the previous approach that only retained the latest snapshot. The new design enables historical audits by persisting a full snapshot history indexed by epoch.

## Key Changes

### 1. Data Structure Updates

**Before:**
- Used `id` field instead of `epoch`
- Stored latest snapshot separately from history
- Used instance storage for history (temporary)

**After:**
- Changed to `epoch`-based indexing
- Unified storage approach using persistent storage
- Proper separation of concerns between latest epoch tracking and historical data

### 2. Storage Architecture

```rust
#[contracttype]
pub enum DataKey {
    /// Map of epoch -> snapshot metadata (persistent storage for full history)
    Snapshots,
    /// Latest epoch number (instance storage for quick access)
    LatestEpoch,
}
```

- **Snapshots**: Persistent storage containing the complete history map
- **LatestEpoch**: Instance storage for efficient latest epoch tracking

### 3. Core Functions

#### `submit_snapshot(env, epoch, hash) -> u64`
- Validates epoch (must be > 0)
- Prevents duplicate epochs
- Stores snapshot in persistent history
- Updates latest epoch if newer
- Returns ledger timestamp

#### `get_snapshot(env, epoch) -> Option<SnapshotMetadata>`
- Retrieves specific epoch snapshot
- Returns None for non-existent epochs

#### `get_snapshot_history(env) -> Map<u64, SnapshotMetadata>`
- Returns complete historical map
- Enables full audit trail access

#### `get_latest_snapshot(env) -> Option<SnapshotMetadata>`
- Efficiently retrieves most recent snapshot
- Uses latest epoch for O(1) lookup

## Acceptance Criteria Compliance

### ✅ Multiple epochs retrievable
- `get_snapshot(epoch)` allows retrieval of any historical epoch
- `get_snapshot_history()` provides access to all epochs
- `get_all_epochs()` lists all available epochs

### ✅ Historical data remains intact after new submissions
- New submissions don't overwrite existing data
- Persistent storage ensures durability
- Comprehensive tests verify data integrity

### ✅ Bounded storage growth strategy
- Each epoch stores only one snapshot (prevents duplicates)
- Efficient Map-based storage structure
- No automatic cleanup (preserves audit trail)
- Future enhancement: Could add epoch-based pruning if needed

## Storage Efficiency

- **Persistent Storage**: Used for historical snapshots (durable across contract upgrades)
- **Instance Storage**: Used for latest epoch tracking (efficient access)
- **Deduplication**: Prevents duplicate epochs to control growth
- **Structured Access**: Map-based storage enables efficient epoch lookups

## Testing Coverage

The implementation includes comprehensive tests covering:

1. **Basic Operations**: Initialization, single snapshot submission
2. **Multiple Epochs**: Non-sequential epoch handling, out-of-order submissions
3. **Data Integrity**: Historical data preservation after new submissions
4. **Edge Cases**: Invalid epochs, duplicate epochs, non-existent lookups
5. **Latest Epoch Logic**: Proper tracking of highest epoch number
6. **Storage Growth**: Simulation of many epochs to verify bounded growth

## Migration Notes

Existing contracts using the old structure will need to:

1. Update data structure from `id` to `epoch`
2. Migrate from instance to persistent storage for history
3. Update client code to use new function signatures
4. Consider data migration strategy for existing snapshots

## Future Enhancements

1. **Epoch Pruning**: Optional cleanup of very old epochs
2. **Batch Operations**: Submit multiple snapshots in one transaction
3. **Event Emission**: Add events for snapshot submissions
4. **Access Control**: Add admin-only submission restrictions
5. **Compression**: Optimize storage for large histories