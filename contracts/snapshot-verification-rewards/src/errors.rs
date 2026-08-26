use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract already initialized
    AlreadyInitialized = 1,
    /// Caller is not authorized
    Unauthorized = 2,
    /// Admin address not set
    AdminNotSet = 3,
    /// Contract is paused
    ContractPaused = 4,
    /// Epoch must be greater than zero
    InvalidEpoch = 5,
    /// Hash must not be all zeros
    InvalidHash = 6,
    /// Snapshot for this epoch has already been registered
    EpochAlreadyRegistered = 7,
    /// No snapshot registered for this epoch
    EpochNotFound = 8,
    /// Epoch is inactive and no longer open for verification
    EpochInactive = 9,
    /// Verifier has already submitted for this epoch
    AlreadyVerified = 10,
    /// No reward points to claim
    NoRewardsToClaim = 11,
    /// Reward amount must be greater than zero
    InvalidRewardAmount = 12,
}
