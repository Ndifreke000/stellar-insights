use soroban_sdk::{contracterror, log, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidEpoch = 4,
    InvalidEpochZero = 5,
    InvalidEpochTooLarge = 6,
    DuplicateEpoch = 7,
    EpochMonotonicityViolated = 8,
    ContractPaused = 9,
    ContractNotPaused = 10,
    InvalidHash = 11,
    InvalidHashZero = 12,
    SnapshotNotFound = 13,
    AdminNotSet = 14,
    GovernanceNotSet = 15,
    RateLimitExceeded = 16,
    TimelockNotExpired = 17,
    ActionNotFound = 18,
    ActionExpired = 19,
    ActionAlreadyExecuted = 20,
    MultiSigNotInitialized = 21,
    InvalidThreshold = 22,
    SignerNotAdmin = 23,
    UnknownActionType = 24,
}

impl Error {
    pub fn log_context(self, env: &Env, context: &str) -> Self {
        log!(env, "[Error #{}] {:?} - {}", self as u32, self, context);
        self
    }

    pub fn description(self) -> &'static str {
        match self {
            Error::AlreadyInitialized => "Contract has already been initialized",
            Error::NotInitialized => "Contract has not been initialized",
            Error::Unauthorized => "Caller is not authorized",
            Error::AdminNotSet => "Admin address has not been initialized",
            Error::GovernanceNotSet => "Governance address has not been set",
            Error::InvalidEpoch => "Invalid epoch value",
            Error::DuplicateEpoch => "A snapshot for this epoch already exists",
            Error::EpochMonotonicityViolated => "Epoch must be strictly greater than the latest",
            Error::SnapshotNotFound => "No snapshot found for the requested epoch",
            Error::ContractPaused => "Contract is currently paused",
            Error::InvalidHash => "Invalid hash value",
            Error::InvalidEpochZero => "Epoch must be greater than 0",
            Error::InvalidEpochTooLarge => "Epoch exceeds maximum allowed value",
            Error::ContractNotPaused => "Contract is not paused",
            Error::InvalidHashZero => "Hash must not be all zeros",
            Error::RateLimitExceeded => "Submission rate limit exceeded",
            Error::TimelockNotExpired => "Timelock period has not yet expired",
            Error::ActionNotFound => "Governance action not found",
            Error::ActionExpired => "Governance action has expired",
            Error::ActionAlreadyExecuted => "Governance action has already been executed",
            Error::MultiSigNotInitialized => "MultiSig configuration has not been initialized",
            Error::InvalidThreshold => "Invalid multisig threshold value",
            Error::SignerNotAdmin => "Signer is not a registered multisig admin",
            Error::UnknownActionType => "Unknown action type",
        }
    }

    pub fn code(self) -> u32 {
        self as u32
    }
}
