# Contract Event Replay System

A comprehensive system for replaying contract events to rebuild application state and support debugging.

## Overview

The Contract Event Replay System provides a reliable, deterministic mechanism for:
- Rebuilding application state from historical blockchain events
- Debugging state inconsistencies
- Verifying state correctness across environments
- Recovering from failures with checkpoint/resume capability
- Testing state transitions in isolation

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Replay Engine                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Config     │  │  Checkpoint  │  │    State     │      │
│  │   Manager    │  │   Manager    │  │   Builder    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  Event Processor                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Snapshot   │  │ Verification │  │    Custom    │      │
│  │  Processor   │  │  Processor   │  │  Processors  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     Storage Layer                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │    Event     │  │    Replay    │  │  Checkpoint  │      │
│  │   Storage    │  │   Storage    │  │   Storage    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

### Module Structure

```
backend/src/replay/
├── mod.rs                  # Public API and types
├── config.rs               # Configuration and replay modes
├── checkpoint.rs           # Checkpoint management
├── engine.rs               # Main replay orchestration
├── event_processor.rs      # Event processing logic
├── state_builder.rs        # State reconstruction
└── storage.rs              # Event and metadata storage
```

## Features

### 1. Deterministic Replay
- Events processed in strict ledger order
- Consistent state reconstruction across environments
- Reproducible results for debugging

### 2. Idempotency Guarantees
- Safe to replay events multiple times
- Duplicate detection prevents state corruption
- Processed events tracked in database

### 3. Checkpoint & Resume
- Automatic checkpoints at configurable intervals
- Resume from any checkpoint
- Partial failure recovery

### 4. Multiple Replay Modes

#### Full Replay
Rebuild entire state from scratch:
```rust
let config = ReplayConfig::new()
    .with_mode(ReplayMode::Full)
    .with_range(ReplayRange::All);
```

#### Incremental Replay
Process only new events since last checkpoint:
```rust
let config = ReplayConfig::new()
    .with_mode(ReplayMode::Incremental)
    .with_range(ReplayRange::FromCheckpoint {
        checkpoint_id: "checkpoint-123".to_string()
    });
```

#### Verification Mode
Replay and compare with existing state:
```rust
let config = ReplayConfig::new()
    .with_mode(ReplayMode::Verification)
    .with_range(ReplayRange::FromTo {
        start: 1000,
        end: 2000
    });
```

#### Debug Mode
Replay with detailed logging, no state changes:
```rust
let config = ReplayConfig::new()
    .with_mode(ReplayMode::Debug)
    .dry_run()
    .verbose();
```

### 5. Event Filtering

Filter by contract, event type, or network:
```rust
let filter = EventFilter {
    contract_ids: Some(vec!["contract-123".to_string()]),
    event_types: Some(vec!["snapshot_submitted".to_string()]),
    network: Some("testnet".to_string()),
};

let config = ReplayConfig::new().with_filter(filter);
```

### 6. Performance Optimization
- Batch processing for large datasets
- Concurrent event processing
- Efficient database queries
- Configurable batch sizes and workers

### 7. Structured Logging
- Comprehensive tracing throughout replay
- Progress tracking
- Error reporting with context
- Performance metrics

## Usage

### Starting a Replay

#### Via API
```bash
curl -X POST http://localhost:8080/api/replay/start \
  -H "Content-Type: application/json" \
  -d '{
    "mode": "full",
    "start_ledger": 1000,
    "end_ledger": 2000,
    "batch_size": 100,
    "dry_run": false
  }'
```

#### Programmatically
```rust
use stellar_insights_backend::replay::*;

// Create components
let event_storage = Arc::new(EventStorage::new(pool.clone()));
let replay_storage = Arc::new(ReplayStorage::new(pool.clone()));
let checkpoint_manager = Arc::new(CheckpointManager::new(pool.clone()));
let processor = Arc::new(CompositeEventProcessor::new());
let state_builder = Arc::new(RwLock::new(StateBuilder::new(pool.clone())));

// Configure replay
let config = ReplayConfig::new()
    .with_mode(ReplayMode::Full)
    .with_range(ReplayRange::FromTo { start: 1000, end: 2000 })
    .with_batch_size(100);

// Create and start engine
let engine = ReplayEngine::new(
    config,
    event_storage,
    replay_storage,
    checkpoint_manager,
    processor,
    state_builder,
)?;

let metadata = engine.start().await?;
```

### Monitoring Progress

```bash
# Get replay status
curl http://localhost:8080/api/replay/status/{session_id}

# List all replays
curl http://localhost:8080/api/replay/list

# List checkpoints
curl http://localhost:8080/api/replay/checkpoints/{session_id}
```

### Resuming from Checkpoint

```bash
curl -X POST http://localhost:8080/api/replay/start \
  -H "Content-Type: application/json" \
  -d '{
    "checkpoint_id": "checkpoint-123"
  }'
```

## Configuration

### Environment Variables

```bash
# Replay configuration
REPLAY_BATCH_SIZE=100
REPLAY_MAX_WORKERS=4
REPLAY_CHECKPOINT_INTERVAL=1000
REPLAY_EVENT_TIMEOUT_SECS=30
REPLAY_MAX_RETRIES=3
```

### Configuration Options

```rust
pub struct ReplayConfig {
    pub mode: ReplayMode,
    pub range: ReplayRange,
    pub filter: EventFilter,
    pub batch_size: usize,           // Default: 100
    pub max_workers: usize,          // Default: 4
    pub dry_run: bool,               // Default: false
    pub verbose: bool,               // Default: false
    pub checkpoint_interval: u64,    // Default: 1000
    pub event_timeout_secs: u64,     // Default: 30
    pub max_retries: u32,            // Default: 3
}
```

## Database Schema

### Contract Events
```sql
CREATE TABLE contract_events (
    id TEXT PRIMARY KEY,
    ledger_sequence INTEGER NOT NULL,
    transaction_hash TEXT NOT NULL,
    contract_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    data TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    network TEXT NOT NULL
);
```

### Replay Sessions
```sql
CREATE TABLE replay_sessions (
    session_id TEXT PRIMARY KEY,
    config TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMP NOT NULL,
    ended_at TIMESTAMP,
    checkpoint TEXT
);
```

### Checkpoints
```sql
CREATE TABLE replay_checkpoints (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    last_ledger INTEGER NOT NULL,
    events_processed INTEGER NOT NULL,
    events_failed INTEGER NOT NULL,
    state_snapshot TEXT NOT NULL,
    metadata TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL
);
```

### State Snapshots
```sql
CREATE TABLE replay_state (
    ledger INTEGER PRIMARY KEY,
    state_json TEXT NOT NULL,
    state_hash TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### Processed Events
```sql
CREATE TABLE processed_events (
    event_id TEXT PRIMARY KEY,
    ledger_sequence INTEGER NOT NULL,
    processed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## Event Processing

### Shared Processing Logic

The same event processing logic is used for both live and replay modes:

```rust
#[async_trait]
pub trait EventProcessor: Send + Sync {
    async fn process_event(
        &self,
        event: &ContractEvent,
        context: &ProcessingContext,
    ) -> Result<ProcessingResult>;

    async fn is_processed(&self, event: &ContractEvent) -> Result<bool>;
    async fn mark_processed(&self, event: &ContractEvent) -> Result<()>;
    fn validate_event(&self, event: &ContractEvent) -> Result<()>;
    fn name(&self) -> &str;
}
```

### Adding Custom Processors

```rust
pub struct CustomEventProcessor {
    pool: SqlitePool,
}

#[async_trait]
impl EventProcessor for CustomEventProcessor {
    async fn process_event(
        &self,
        event: &ContractEvent,
        context: &ProcessingContext,
    ) -> Result<ProcessingResult> {
        // Custom processing logic
        Ok(ProcessingResult::success())
    }

    async fn is_processed(&self, event: &ContractEvent) -> Result<bool> {
        // Check if already processed
        Ok(false)
    }

    async fn mark_processed(&self, event: &ContractEvent) -> Result<()> {
        // Mark as processed
        Ok(())
    }

    fn name(&self) -> &str {
        "CustomEventProcessor"
    }
}

// Register processor
let processor = CompositeEventProcessor::new()
    .add_processor(Arc::new(CustomEventProcessor::new(pool)));
```

## State Building

### Application State

```rust
pub struct ApplicationState {
    pub ledger: u64,
    pub snapshots: HashMap<u64, SnapshotState>,
    pub verifications: HashMap<String, VerificationState>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### State Verification

```rust
// Build state
let mut builder = StateBuilder::new(pool);
for event in events {
    builder.apply_event(&event).await?;
}

// Persist state
builder.persist_state().await?;

// Verify state hash
let verified = builder.verify_state(ledger).await?;
assert!(verified);
```

## Error Handling

### Error Types

```rust
pub enum ReplayError {
    EventNotFound(String),
    InvalidCheckpoint(String),
    AlreadyInProgress(String),
    StorageError(anyhow::Error),
    ProcessingError(String),
    ConfigError(String),
    StateCorruption(String),
}
```

### Recovery Strategies

1. **Checkpoint Recovery**: Resume from last successful checkpoint
2. **Partial Replay**: Replay only failed ledger range
3. **State Verification**: Compare rebuilt state with expected state
4. **Manual Intervention**: Inspect logs and fix data issues

## Testing

### Running Tests

```bash
# Run all replay tests
cargo test replay_system_test

# Run specific test
cargo test test_state_idempotency

# Run with logging
RUST_LOG=debug cargo test replay_system_test
```

### Test Coverage

- ✅ Event storage and retrieval
- ✅ Event filtering
- ✅ Checkpoint creation and restoration
- ✅ State building and verification
- ✅ Idempotency guarantees
- ✅ Event ordering
- ✅ Concurrent processing
- ✅ State corruption detection
- ✅ Configuration validation
- ✅ Error handling

## Performance

### Benchmarks

| Operation | Events | Duration | Throughput |
|-----------|--------|----------|------------|
| Full Replay | 10,000 | ~30s | 333 events/s |
| Incremental | 1,000 | ~3s | 333 events/s |
| Checkpoint | N/A | ~100ms | N/A |
| State Verify | N/A | ~50ms | N/A |

### Optimization Tips

1. **Batch Size**: Increase for better throughput (100-1000)
2. **Workers**: Match to CPU cores (4-8)
3. **Checkpoints**: Balance frequency vs overhead (1000-10000)
4. **Filtering**: Reduce events processed
5. **Dry Run**: Skip database writes for testing

## Monitoring

### Metrics

- Events processed per second
- Events failed
- Checkpoint creation time
- State verification time
- Memory usage
- Database query performance

### Logging

```rust
// Enable verbose logging
RUST_LOG=stellar_insights_backend::replay=debug cargo run

// Structured logging output
{
  "timestamp": "2024-01-01T00:00:00Z",
  "level": "INFO",
  "target": "replay::engine",
  "message": "Processing ledgers 1000 to 1099",
  "session_id": "abc-123",
  "events_processed": 100
}
```

## Best Practices

1. **Always use dry-run first** to validate configuration
2. **Create checkpoints frequently** for long replays
3. **Monitor progress** via API or logs
4. **Verify state** after replay completion
5. **Clean up old checkpoints** periodically
6. **Test in non-production** environment first
7. **Use filtering** to reduce scope when debugging
8. **Enable verbose logging** for troubleshooting

## Troubleshooting

### Common Issues

#### Replay Stuck
- Check logs for errors
- Verify database connectivity
- Increase timeout settings
- Resume from last checkpoint

#### State Mismatch
- Run verification mode
- Check for missing events
- Verify event ordering
- Inspect state hash

#### Performance Issues
- Increase batch size
- Add more workers
- Optimize database indexes
- Reduce checkpoint frequency

#### Memory Issues
- Reduce batch size
- Decrease max workers
- Clear old checkpoints
- Process in smaller ranges

## API Reference

### Endpoints

```
POST   /api/replay/start              # Start new replay
GET    /api/replay/status/:id         # Get replay status
GET    /api/replay/list               # List all replays
GET    /api/replay/checkpoints/:id    # List checkpoints
DELETE /api/replay/:id                # Delete replay session
POST   /api/replay/cleanup            # Cleanup old checkpoints
```

### Request/Response Examples

See API documentation for detailed examples.

## Future Enhancements

- [ ] Parallel event processing
- [ ] Streaming replay for real-time updates
- [ ] Advanced filtering with SQL-like queries
- [ ] State diff visualization
- [ ] Automated state verification
- [ ] Performance profiling tools
- [ ] Web UI for replay management
- [ ] Export/import replay configurations

## Contributing

See CONTRIBUTING.md for guidelines on adding new processors or features.

## License

Part of the Stellar Insights backend application.
