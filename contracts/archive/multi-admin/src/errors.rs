use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract already initialized
    AlreadyInitialized = 1,
    /// Caller is not an admin
    Unauthorized = 2,
    /// Caller's role is insufficient for this operation
    InsufficientRole = 3,
    /// Admin address not found
    AdminNotFound = 4,
    /// Admin address already exists
    AdminAlreadyExists = 5,
    /// Cannot remove or demote the last SuperAdmin
    CannotRemoveLastSuperAdmin = 6,
    /// Contract is paused
    ContractPaused = 7,
    /// Epoch must be greater than zero
    InvalidEpoch = 8,
    /// Hash must not be all zeros
    InvalidHash = 9,
    /// Snapshot for this epoch already exists
    DuplicateEpoch = 10,
    /// Epoch must be strictly greater than the latest recorded epoch
    EpochMonotonicityViolated = 11,
    /// Snapshot not found
    SnapshotNotFound = 12,
}
