use soroban_sdk::contracterror;

/// Contract-specific errors for Governance Contract
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Caller is not authorized to perform this action
    UnauthorizedCaller = 1,
    /// Proposal not found
    ProposalNotFound = 2,
    /// Voting is not active for this proposal
    VotingNotActive = 3,
    /// Voting period has not ended yet
    VotingPeriodNotEnded = 4,
    /// Voter has already voted on this proposal
    AlreadyVoted = 5,
    /// Proposal has already been finalized
    AlreadyFinalized = 6,
    /// Admin address not initialized
    AdminNotSet = 7,
    /// Proposal has not passed and cannot be executed
    ProposalNotPassed = 9,
    /// Invalid proposal title
    InvalidTitle = 10,
}
