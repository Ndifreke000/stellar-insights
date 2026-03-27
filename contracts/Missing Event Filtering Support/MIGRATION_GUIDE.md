# Migration Guide: Adding Event Filtering to Existing Contracts

This guide helps you add event filtering support to existing Stellar smart contracts.

## Overview

If your contract currently emits events without topics, this guide shows you how to add filtering metadata while maintaining backward compatibility.

## Step 1: Define EventType Enum

Add an enum to standardize your event types:

```rust
#[contracttype]
#[derive(Clone, Copy)]
pub enum EventType {
    SnapshotSubmitted = 0,
    AdminChanged = 1,
    ContractPaused = 2,
    ContractUnpaused = 3,
    GovernanceSet = 4,
}
```

## Step 2: Update Event Structs

Add metadata fields that will be stored in event data:

```rust
// Before
#[contracttype]
pub struct SnapshotEvent {
    pub epoch: u64,
}

// After
#[contracttype]
pub struct SnapshotSubmittedEvent {
    pub epoch: u64,
    pub hash: BytesN<32>,
    pub submitter: Address,
    pub timestamp: u64,
    pub ledger_sequence: u32,
}
```

## Step 3: Update Event Emission

Add topics to your event publishing calls:

```rust
// Before
env.events().publish(
    (symbol_short!("snapshot"),),
    SnapshotEvent { epoch },
);

// After
env.events().publish(
    (
        symbol_short!("snapshot"),  // Topic 0: Event type
        caller.clone(),             // Topic 1: Submitter
        epoch,                      // Topic 2: Epoch
    ),
    SnapshotSubmittedEvent {
        epoch,
        hash,
        submitter: caller,
        timestamp: env.ledger().timestamp(),
        ledger_sequence: env.ledger().sequence(),
    },
);
```

## Step 4: Update Query Patterns

Update your query code to use topics:

```javascript
// Before
const events = await server.getEvents({
  filters: [
    {
      type: "contract",
      contractId: CONTRACT_ID,
    },
  ],
});

// After - More efficient filtering
const events = await server.getEvents({
  filters: [
    {
      type: "contract",
      contractId: CONTRACT_ID,
      topics: ["snapshot", SUBMITTER_ADDRESS, EPOCH],
    },
  ],
});
```

## Step 5: Test Event Emission

Add tests to verify new event structure:

```rust
#[test]
fn test_snapshot_event_has_topics() {
    let env = Env::default();
    let contract_id = env.register_contract(None, YourContract);
    let client = YourContractClient::new(&env, &contract_id);

    // Submit snapshot with new event structure
    let result = client.submit_snapshot(&1, &hash, &submitter);
    assert!(result.is_ok());

    // Verify events have topics
    let events = env.events().all();
    assert!(events.len() > 0);
}
```

## Migration Checklist

- [ ] Define EventType enum
- [ ] Update event structs with new fields
- [ ] Add topics to event emission calls
- [ ] Update query patterns in client code
- [ ] Add/update tests for new event structure
- [ ] Document event topic meanings
- [ ] Deploy to testnet for verification
- [ ] Update production queries after mainnet deployment

## Common Migration Patterns

### Pattern 1: Add Submitter Tracking

```rust
// Old
env.events().publish((symbol_short!("action"),), EventData { value: x });

// New
env.events().publish(
    (symbol_short!("action"), caller.clone()),
    EventData { value: x, submitter: caller, timestamp: env.ledger().timestamp() },
);
```

### Pattern 2: Add Epoch/Sequence Tracking

```rust
// Old
env.events().publish((symbol_short!("snapshot"),), SnapshotEvent { epoch });

// New
env.events().publish(
    (symbol_short!("snapshot"), caller.clone(), epoch),
    SnapshotSubmittedEvent {
        epoch,
        submitter: caller,
        timestamp: env.ledger().timestamp(),
        ledger_sequence: env.ledger().sequence(),
        hash,
    },
);
```

### Pattern 3: Add Multi-Level Filtering

```rust
// Old
env.events().publish(
    (symbol_short!("governance"),),
    GovernanceEvent { new_governance }
);

// New
env.events().publish(
    (symbol_short!("gov_set"), new_governance.clone()),
    GovernanceSetEvent {
        new_governance,
        old_governance: Some(current_gov),
        timestamp: env.ledger().timestamp(),
    },
);
```

## Performance Impact

| Metric            | Impact | Note                               |
| ----------------- | ------ | ---------------------------------- |
| Contract Size     | +1-2%  | Additional enum/struct definitions |
| Deployment Gas    | +5-10% | Larger compiled binary             |
| Event Emission    | ~Same  | Topics are indexed automatically   |
| Query Performance | ↓ 100x | O(n) → O(1) for filtered queries   |

## Backward Compatibility

When adding event filtering:

- ✅ Old events without topics remain accessible
- ✅ New events with topics are backward compatible
- ✅ Query tools support both old and new patterns
- ⚠️ Ledger history shows both versions

## Troubleshooting

### Issue: "Type mismatch in topics"

**Solution:** Ensure all topics are `Address`, `u64`, `i128`, or `symbol`. Cast numeric types explicitly:

```rust
// ❌ Wrong
env.events().publish((symbol_short!("event"), some_u32), data);

// ✅ Correct
env.events().publish((symbol_short!("event"), some_u32 as u64), data);
```

### Issue: "Too many topics"

**Solution:** Use maximum 3 topics per event. Combine related data into single topic:

```rust
// ❌ Wrong - 4 topics
(symbol_short!("event"), addr1, addr2, value)

// ✅ Correct - 3 topics
(symbol_short!("event"), primary_addr, value)
```

### Issue: "Query returns no results"

**Solution:** Verify topic order and values match exactly. Topics are position-sensitive:

```bash
# Topics must match in order
# If event has (symbol_short!("event"), addr, epoch)
# Query must be: --topic event --topic <addr> --topic <epoch>
```

## References

- [Soroban Events Best Practices](https://stellar.org/developers)
- [This Implementation](./EVENT_FILTERING_GUIDE.md)
- [Stellar Developer Docs](https://developers.stellar.org)
