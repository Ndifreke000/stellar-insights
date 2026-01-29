# Snapshot Hash Generation Service

This document describes the implementation of the Snapshot Hash Generation Service for issue #122, which creates SHA-256 hashes of analytics snapshots for on-chain submission.

## Overview

The service fulfills all acceptance criteria:

1. ✅ **Aggregate all metrics** - Collects anchor and corridor metrics from database
2. ✅ **Serialize to deterministic JSON** - Creates canonical JSON representation
3. ✅ **Compute SHA-256 hash** - Generates cryptographic hash of the snapshot
4. ✅ **Store hash in database** - Persists snapshot and hash for audit trail
5. ✅ **Submit to smart contract** - Submits hash to Soroban contract on Stellar
6. ✅ **Verify submission success** - Confirms on-chain storage and retrieval

## Architecture

### Core Components

1. **SnapshotService** (`src/services/snapshot.rs`)
   - Main service orchestrating the complete workflow
   - Aggregates metrics from database
   - Handles deterministic serialization and hashing
   - Manages database storage and contract submission

2. **ContractService** (`src/services/contract.rs`)
   - Handles Soroban smart contract interactions
   - Submits hashes to on-chain contract
   - Verifies successful submission
   - Includes retry logic and error handling

3. **AnalyticsSnapshot** (`src/snapshot/schema.rs`)
   - Data structure representing a complete snapshot
   - Contains anchor and corridor metrics
   - Supports deterministic normalization

### Database Schema

The service uses the existing `snapshots` table:

```sql
CREATE TABLE snapshots (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    data TEXT NOT NULL,        -- Canonical JSON
    hash TEXT,                 -- SHA-256 hash (hex)
    epoch INTEGER,             -- Epoch identifier
    timestamp TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

## Usage

### Basic Usage

```rust
use stellar_insights::services::snapshot::SnapshotService;
use stellar_insights::database::Database;
use std::sync::Arc;

// Initialize service
let db = Arc::new(Database::new("sqlite:stellar_insights.db").await?);
let snapshot_service = SnapshotService::new(db, None);

// Generate snapshot
let result = snapshot_service.generate_and_submit_snapshot(epoch).await?;

println!("Generated hash: {}", result.hash);
println!("Stored with ID: {}", result.snapshot_id);
```

### With Contract Submission

```rust
use stellar_insights::services::contract::ContractService;

// Initialize with contract service
let contract_service = Some(Arc::new(ContractService::from_env()?));
let snapshot_service = SnapshotService::new(db, contract_service);

// Generate and submit to blockchain
let result = snapshot_service.generate_and_submit_snapshot(epoch).await?;

if let Some(submission) = result.submission_result {
    println!("Transaction hash: {}", submission.transaction_hash);
    println!("Verification: {}", result.verification_successful);
}
```

### HTTP API

The service is exposed via HTTP endpoints:

```bash
# Generate snapshot
POST /api/snapshots/generate
{
  "epoch": 12345,
  "submit_to_contract": true
}

# Response
{
  "epoch": 12345,
  "timestamp": "2024-01-15T10:30:00Z",
  "hash": "a1b2c3d4...",
  "schema_version": 1,
  "anchor_count": 5,
  "corridor_count": 12,
  "submission": {
    "transaction_hash": "tx123...",
    "ledger": 98765,
    "contract_timestamp": 1705312200
  }
}
```

## Implementation Details

### 1. Metrics Aggregation

The service aggregates metrics from two main sources:

**Anchor Metrics:**
- Success/failure rates
- Transaction volumes
- Settlement times
- Reliability scores

**Corridor Metrics:**
- Asset pair transaction data
- Liquidity depth
- Settlement latency
- Success rates

### 2. Deterministic Serialization

Key features ensuring determinism:
- All arrays sorted by UUID
- JSON keys in alphabetical order
- Consistent floating-point representation
- No extra whitespace
- ISO 8601 timestamp format

### 3. SHA-256 Hash Generation

- Uses `sha2` crate for cryptographic hashing
- Operates on canonical JSON bytes
- Returns both byte array and hex string formats
- Reproducible: same input always produces same hash

### 4. Database Storage

Snapshots are stored with:
- Unique UUID identifier
- Complete canonical JSON data
- SHA-256 hash (hex encoded)
- Epoch number for temporal ordering
- Timestamp metadata

### 5. Smart Contract Integration

The service integrates with a Soroban smart contract:
- Submits 32-byte hash with epoch number
- Handles transaction signing and submission
- Implements retry logic with exponential backoff
- Verifies successful on-chain storage

### 6. Verification Process

After submission, the service:
- Waits for transaction confirmation
- Queries contract to verify hash exists
- Confirms epoch and hash match
- Reports verification success/failure

## Configuration

### Environment Variables

```bash
# Database
DATABASE_URL=sqlite:stellar_insights.db

# Contract Service (optional)
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
SNAPSHOT_CONTRACT_ID=CBGTG4JJFEQE3SPBGQFP3X5HM46N47LXZPXQACVKB7QA6X2XB2IG5CTA
STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
STELLAR_SOURCE_SECRET_KEY=S...
```

### Service Initialization

```rust
// Database only (no contract submission)
let snapshot_service = SnapshotService::new(db, None);

// With contract service
let contract_service = Some(Arc::new(ContractService::from_env()?));
let snapshot_service = SnapshotService::new(db, contract_service);
```

## Testing

### Unit Tests

The service includes comprehensive tests:

```bash
cargo test snapshot_service
```

### Integration Tests

```bash
cargo test test_snapshot_generation_without_contract
cargo test test_database_storage
cargo test test_deterministic_serialization
cargo test test_hash_computation
```

### Demo

Run the complete workflow demo:

```bash
cargo run --example snapshot_generation_demo
```

## Error Handling

The service provides detailed error handling:

- **Aggregation errors**: Database connection issues, missing data
- **Serialization errors**: JSON formatting problems
- **Hash computation errors**: Cryptographic failures
- **Storage errors**: Database write failures
- **Contract errors**: Network issues, transaction failures
- **Verification errors**: On-chain query problems

## Performance Considerations

- **Database queries**: Optimized with indexes on key columns
- **Memory usage**: Streaming for large datasets
- **Network calls**: Retry logic with exponential backoff
- **Concurrent access**: Thread-safe design with Arc<>

## Security

- **Hash integrity**: SHA-256 provides cryptographic security
- **Deterministic output**: Prevents hash manipulation
- **Database storage**: Audit trail for all snapshots
- **Contract verification**: On-chain immutability
- **Error handling**: No sensitive data in error messages

## Monitoring and Observability

The service includes comprehensive logging:

```rust
use tracing::{info, warn, error, debug};

info!("Generated snapshot hash: {}", hash);
warn!("Contract service not configured");
error!("Failed to submit to contract: {}", error);
debug!("Aggregated {} metrics", count);
```

## Future Enhancements

Potential improvements:
- Automated scheduling for regular snapshots
- Snapshot comparison and diff functionality
- Multiple contract support for redundancy
- Compression for large snapshots
- Metrics export in multiple formats
- Real-time snapshot streaming

## Conclusion

The Snapshot Hash Generation Service successfully implements all acceptance criteria for issue #122, providing a robust, secure, and verifiable system for creating cryptographic hashes of analytics snapshots and submitting them to the Stellar blockchain.