use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, Symbol};

// ============================================================================
// Event Topics - Short symbols for efficient on-chain storage
// ============================================================================

/// Topic for snapshot submission events
pub const SNAPSHOT_SUBMITTED: Symbol = symbol_short!("SNAP_SUB");

/// Topic for snapshot lifecycle events (for filtering)
pub const SNAPSHOT_LIFECYCLE: Symbol = symbol_short!("SNAP_LFE");

// ============================================================================
// Event Structures
// ============================================================================

/// Event emitted when an analytics snapshot is successfully submitted.
/// 
/// This event enables off-chain indexing services to:
/// - Track all snapshot submissions in real-time
/// - Verify data integrity by comparing stored hash with off-chain data
/// - Build historical records of analytics epochs
/// 
/// # Fields
/// - `hash`: The 32-byte SHA-256 hash of the analytics snapshot data
/// - `epoch`: The epoch identifier for this snapshot (positive integer)
/// - `timestamp`: Ledger timestamp when the snapshot was recorded on-chain
/// - `submitter`: Address of the admin who submitted the snapshot
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotSubmitted {
    /// SHA-256 hash of the analytics snapshot data
    pub hash: BytesN<32>,
    /// Epoch identifier for this snapshot
    pub epoch: u64,
    /// Ledger timestamp when the snapshot was recorded
    pub timestamp: u64,
    /// Address of the admin who submitted the snapshot
    pub submitter: Address,
}

impl SnapshotSubmitted {
    /// Create and publish a SnapshotSubmitted event
    /// 
    /// # Arguments
    /// * `env` - Contract environment
    /// * `hash` - 32-byte SHA-256 hash of the snapshot
    /// * `epoch` - Epoch identifier
    /// * `timestamp` - Ledger timestamp
    /// * `submitter` - Address of the submitter
    /// 
    /// # Event Format
    /// Topic: (SNAPSHOT_SUBMITTED, SNAPSHOT_LIFECYCLE)
    /// Data: SnapshotSubmitted struct containing hash, epoch, timestamp, submitter
    pub fn publish(
        env: &Env,
        hash: BytesN<32>,
        epoch: u64,
        timestamp: u64,
        submitter: Address,
    ) {
        let event = SnapshotSubmitted {
            hash,
            epoch,
            timestamp,
            submitter,
        };

        // Publish with multiple topics for flexible filtering
        // Indexers can filter by SNAPSHOT_SUBMITTED or SNAPSHOT_LIFECYCLE
        env.events()
            .publish((SNAPSHOT_SUBMITTED, SNAPSHOT_LIFECYCLE), event);
    }
}

/// Legacy event structure for backwards compatibility
/// 
/// This event emitted when an analytics snapshot is successfully submitted.
/// Deprecated: Use `SnapshotSubmitted` for new implementations.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyticsSnapshotSubmitted {
    /// Epoch identifier for this snapshot
    pub epoch: u64,
    /// SHA-256 hash of the analytics snapshot
    pub hash: BytesN<32>,
    /// Ledger timestamp when the snapshot was recorded
    pub timestamp: u64,
}

impl AnalyticsSnapshotSubmitted {
    /// Publish this event to the blockchain (legacy format)
    /// 
    /// # Deprecated
    /// Use `SnapshotSubmitted::publish()` instead for new implementations.
    pub fn publish(env: &Env, epoch: u64, hash: BytesN<32>, timestamp: u64) {
        let event = AnalyticsSnapshotSubmitted {
            epoch,
            hash: hash.clone(),
            timestamp,
        };

        env.events().publish((SNAPSHOT_SUBMITTED,), event);
    }
}

// ============================================================================
// Event Helper Functions
// ============================================================================

/// Emit a snapshot submitted event with full payload
/// 
/// This is the primary function for emitting snapshot lifecycle events.
/// It ensures the event payload matches the stored data exactly.
/// 
/// # Arguments
/// * `env` - Contract environment
/// * `hash` - The exact hash that was stored
/// * `epoch` - The exact epoch that was stored
/// * `timestamp` - The exact timestamp that was stored
/// * `submitter` - The address of the caller who submitted
pub fn emit_snapshot_submitted(
    env: &Env,
    hash: BytesN<32>,
    epoch: u64,
    timestamp: u64,
    submitter: Address,
) {
    SnapshotSubmitted::publish(env, hash, epoch, timestamp, submitter);
}
