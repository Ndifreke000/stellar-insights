use soroban_sdk::contracterror;

/// Contract-specific errors for Stellar Insights Analytics Contract
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Caller is not authorized to perform this action
    UnauthorizedCaller = 1,
    /// Snapshot for this epoch already exists
    DuplicateEpoch = 2,
    /// Invalid hash size (must be 32 bytes)
    InvalidHashSize = 3,
    /// Epoch must be greater than 0
    InvalidEpoch = 4,
    /// Admin address not initialized
    AdminNotSet = 5,
    /// No snapshot found for the requested epoch
    SnapshotNotFound = 6,
}
