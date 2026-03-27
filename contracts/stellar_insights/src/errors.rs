use soroban_sdk::{contracterror, log, Env};

/// Contract-specific errors for Stellar Insights Analytics Contract
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract has already been initialized
    AlreadyInitialized = 1,
    /// Contract has not been initialized
    NotInitialized = 2,
    /// Caller is not authorized to perform this action
    Unauthorized = 3,
    /// Generic invalid epoch
    InvalidEpoch = 4,
    /// Epoch must be greater than 0
    InvalidEpochZero = 5,
    /// Epoch exceeds maximum allowed value
    InvalidEpochTooLarge = 6,
    /// Snapshot for this epoch already exists
    DuplicateEpoch = 7,
    /// Epoch must be strictly greater than the latest recorded epoch
    EpochMonotonicityViolated = 8,
    /// Contract is paused
    ContractPaused = 9,
    /// Contract is not paused
    ContractNotPaused = 10,
    /// Generic invalid hash
    InvalidHash = 11,
    /// Hash must not be all zeros
    InvalidHashZero = 12,
    /// No snapshot found for the requested epoch
    SnapshotNotFound = 13,
    /// Admin address not initialized
    AdminNotSet = 14,
    /// Governance address not set
    GovernanceNotSet = 15,
    /// Submission rate limit exceeded
    RateLimitExceeded = 16,
    /// Timelock period has not yet expired
    TimelockNotExpired = 17,
    /// Governance action not found
    ActionNotFound = 18,
    /// Governance action has expired
    ActionExpired = 19,
    /// Governance action has already been executed
    ActionAlreadyExecuted = 20,
    /// Caller is not authorized (legacy alias kept for compatibility)
    UnauthorizedCaller = 21,
    /// Invalid hash size (must be 32 bytes)
    InvalidHashSize = 22,
}

impl Error {
    /// Log contextual information alongside the error for easier debugging.
    /// Returns self so it can be used inline with `?` via `.log_context()`.
    pub fn log_context(self, env: &Env, context: &str) -> Self {
        log!(env, "[Error #{}] {:?} - {}", self as u32, self, context);
        self
    }

    /// Human-readable description of the error code.
    pub fn description(self) -> &'static str {
        match self {
            Error::AlreadyInitialized => "Contract has already been initialized",
            Error::NotInitialized => "Contract has not been initialized",
            Error::Unauthorized => "Caller is not authorized",
            Error::InvalidEpoch => "Invalid epoch value",
            Error::InvalidEpochZero => "Epoch must be greater than 0",
            Error::InvalidEpochTooLarge => "Epoch exceeds maximum allowed value",
            Error::DuplicateEpoch => "Snapshot for this epoch already exists",
            Error::EpochMonotonicityViolated => "Epoch must be strictly greater than the latest",
            Error::ContractPaused => "Contract is currently paused",
            Error::ContractNotPaused => "Contract is not paused",
            Error::InvalidHash => "Invalid hash value",
            Error::InvalidHashZero => "Hash must not be all zeros",
            Error::SnapshotNotFound => "No snapshot found for the requested epoch",
            Error::AdminNotSet => "Admin address has not been initialized",
            Error::GovernanceNotSet => "Governance address has not been set",
            Error::RateLimitExceeded => "Submission rate limit exceeded",
            Error::TimelockNotExpired => "Timelock period has not yet expired",
            Error::ActionNotFound => "Governance action not found",
            Error::ActionExpired => "Governance action has expired",
            Error::ActionAlreadyExecuted => "Governance action has already been executed",
            Error::UnauthorizedCaller => "Caller is not authorized to perform this action",
            Error::InvalidHashSize => "Invalid hash size (must be 32 bytes)",
        }
    }

    /// Numeric code for the error, useful for off-chain indexing.
    pub fn code(self) -> u32 {
        self as u32
    }
}
