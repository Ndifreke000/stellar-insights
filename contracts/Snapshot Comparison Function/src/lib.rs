use soroban_sdk::{contract, contractimpl, contracttype, Env, Map, Symbol, symbol_short, Result as SorobanResult};

// Error types
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    NotInitialized = 1,
    SnapshotNotFound = 2,
    InvalidEpochRange = 3,
    ChainVerificationFailed = 4,
}

// Storage key enum
#[derive(Clone)]
pub enum DataKey {
    Snapshots,
}

// Snapshot metadata structure
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotMetadata {
    pub epoch: u64,
    pub hash: Symbol,
    pub timestamp: u64,
    pub submitter: Symbol,
}

// Snapshot diff structure for comparison results
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotDiff {
    pub epoch_a: u64,
    pub epoch_b: u64,
    pub hash_match: bool,
    pub timestamp_diff: i64,
    pub submitter_match: bool,
}

#[contract]
pub struct SnapshotComparison;

#[contractimpl]
impl SnapshotComparison {
    /// Initialize the contract with empty snapshots map
    pub fn init(env: Env) -> SorobanResult<()> {
        let snapshots: Map<u64, SnapshotMetadata> = Map::new(&env);
        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);
        Ok(())
    }

    /// Store a snapshot
    pub fn store_snapshot(
        env: Env,
        epoch: u64,
        hash: Symbol,
        timestamp: u64,
        submitter: Symbol,
    ) -> SorobanResult<()> {
        let mut snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .map_err(|_| Error::NotInitialized)?;

        let snapshot = SnapshotMetadata {
            epoch,
            hash,
            timestamp,
            submitter,
        };

        snapshots.set(epoch, snapshot);
        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);

        const LEDGERS_TO_EXTEND: u32 = 518_400;
        env.storage().persistent().extend_ttl(
            &DataKey::Snapshots,
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

        Ok(())
    }

    /// Get a specific snapshot by epoch
    pub fn get_snapshot(env: Env, epoch: u64) -> SorobanResult<SnapshotMetadata> {
        const LEDGERS_TO_EXTEND: u32 = 518_400;
        if env.storage().persistent().has(&DataKey::Snapshots) {
            env.storage().persistent().extend_ttl(
                &DataKey::Snapshots,
                LEDGERS_TO_EXTEND,
                LEDGERS_TO_EXTEND,
            );
        }
        let snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .map_err(|_| Error::NotInitialized)?;

        snapshots
            .get(epoch)
            .ok_or(Error::SnapshotNotFound)
    }

    /// Compare two snapshots and return their differences
    pub fn compare_snapshots(
        env: Env,
        epoch_a: u64,
        epoch_b: u64,
    ) -> SorobanResult<SnapshotDiff> {
        if epoch_a == epoch_b {
            return Err(Error::InvalidEpochRange);
        }

        let snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .map_err(|_| Error::NotInitialized)?;

        let snapshot_a = snapshots
            .get(epoch_a)
            .ok_or(Error::SnapshotNotFound)?;
        let snapshot_b = snapshots
            .get(epoch_b)
            .ok_or(Error::SnapshotNotFound)?;

        Ok(SnapshotDiff {
            epoch_a,
            epoch_b,
            hash_match: snapshot_a.hash == snapshot_b.hash,
            timestamp_diff: (snapshot_b.timestamp as i64) - (snapshot_a.timestamp as i64),
            submitter_match: snapshot_a.submitter == snapshot_b.submitter,
        })
    }

    /// Verify the integrity of a snapshot chain (monotonicity and validity)
    pub fn verify_snapshot_chain(
        env: Env,
        start_epoch: u64,
        end_epoch: u64,
    ) -> SorobanResult<bool> {
        if start_epoch >= end_epoch {
            return Err(Error::InvalidEpochRange);
        }

        let snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .map_err(|_| Error::NotInitialized)?;

        // Verify monotonicity and integrity of snapshot chain
        for epoch in start_epoch..end_epoch {
            let current = snapshots
                .get(epoch)
                .ok_or(Error::SnapshotNotFound)?;
            let next = snapshots
                .get(epoch + 1)
                .ok_or(Error::SnapshotNotFound)?;

            // Check that timestamps are strictly monotonic (increasing)
            if next.timestamp <= current.timestamp {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_snapshot_comparison() {
        let env = Env::default();
        
        // Initialize
        SnapshotComparison::init(env.clone()).unwrap();

        // Store two snapshots
        SnapshotComparison::store_snapshot(
            env.clone(),
            1,
            symbol_short!("hash1"),
            1000,
            symbol_short!("alice"),
        )
        .unwrap();

        SnapshotComparison::store_snapshot(
            env.clone(),
            2,
            symbol_short!("hash2"),
            2000,
            symbol_short!("bob"),
        )
        .unwrap();

        // Compare snapshots
        let diff = SnapshotComparison::compare_snapshots(env, 1, 2).unwrap();

        assert_eq!(diff.epoch_a, 1);
        assert_eq!(diff.epoch_b, 2);
        assert_eq!(diff.hash_match, false); // hash1 != hash2
        assert_eq!(diff.timestamp_diff, 1000); // 2000 - 1000
        assert_eq!(diff.submitter_match, false); // alice != bob
    }

    #[test]
    fn test_snapshot_comparison_matching() {
        let env = Env::default();
        
        // Initialize
        SnapshotComparison::init(env.clone()).unwrap();

        // Store two identical snapshots
        SnapshotComparison::store_snapshot(
            env.clone(),
            1,
            symbol_short!("hash1"),
            1000,
            symbol_short!("alice"),
        )
        .unwrap();

        SnapshotComparison::store_snapshot(
            env.clone(),
            2,
            symbol_short!("hash1"),
            3000,
            symbol_short!("alice"),
        )
        .unwrap();

        // Compare snapshots
        let diff = SnapshotComparison::compare_snapshots(env, 1, 2).unwrap();

        assert_eq!(diff.hash_match, true);
        assert_eq!(diff.submitter_match, true);
        assert_eq!(diff.timestamp_diff, 2000);
    }

    #[test]
    fn test_snapshot_chain_verification() {
        let env = Env::default();
        
        // Initialize
        SnapshotComparison::init(env.clone()).unwrap();

        // Store snapshots with monotonically increasing timestamps
        SnapshotComparison::store_snapshot(
            env.clone(),
            1,
            symbol_short!("hash1"),
            1000,
            symbol_short!("alice"),
        )
        .unwrap();

        SnapshotComparison::store_snapshot(
            env.clone(),
            2,
            symbol_short!("hash2"),
            2000,
            symbol_short!("bob"),
        )
        .unwrap();

        SnapshotComparison::store_snapshot(
            env.clone(),
            3,
            symbol_short!("hash3"),
            3000,
            symbol_short!("charlie"),
        )
        .unwrap();

        // Verify chain is valid (should return true)
        let is_valid = SnapshotComparison::verify_snapshot_chain(env, 1, 3).unwrap();
        assert_eq!(is_valid, true);
    }

    #[test]
    fn test_snapshot_chain_verification_failed() {
        let env = Env::default();
        
        // Initialize
        SnapshotComparison::init(env.clone()).unwrap();

        // Store snapshots with NON-monotonic timestamps
        SnapshotComparison::store_snapshot(
            env.clone(),
            1,
            symbol_short!("hash1"),
            1000,
            symbol_short!("alice"),
        )
        .unwrap();

        SnapshotComparison::store_snapshot(
            env.clone(),
            2,
            symbol_short!("hash2"),
            800, // Earlier than epoch 1 - violates monotonicity
            symbol_short!("bob"),
        )
        .unwrap();

        // Verify chain is invalid (should return false)
        let is_valid = SnapshotComparison::verify_snapshot_chain(env, 1, 2).unwrap();
        assert_eq!(is_valid, false);
    }

    #[test]
    fn test_snapshot_not_found() {
        let env = Env::default();
        
        // Initialize
        SnapshotComparison::init(env.clone()).unwrap();

        // Try to get a snapshot that doesn't exist
        let result = SnapshotComparison::get_snapshot(env, 999);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_epoch_range() {
        let env = Env::default();
        
        // Initialize
        SnapshotComparison::init(env.clone()).unwrap();

        // Try to compare same epoch
        let result = SnapshotComparison::compare_snapshots(env.clone(), 1, 1);
        assert!(result.is_err());

        // Try to verify with invalid range
        let result = SnapshotComparison::verify_snapshot_chain(env, 2, 1);
        assert!(result.is_err());
    }
}
