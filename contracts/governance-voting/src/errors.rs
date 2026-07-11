use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract already initialized
    AlreadyInitialized = 1,
    /// Admin not set
    AdminNotSet = 2,
    /// Caller is not authorized
    Unauthorized = 3,
    /// Proposal not found
    ProposalNotFound = 4,
    /// Voter is not registered
    VoterNotRegistered = 5,
    /// Voter already registered
    AlreadyRegistered = 6,
    /// Voter has already cast a vote on this proposal
    AlreadyVoted = 7,
    /// Proposal is not in an active voting state
    VotingNotActive = 8,
    /// Voting period has not ended yet
    VotingPeriodNotEnded = 9,
    /// Proposal has already been finalized
    AlreadyFinalized = 10,
    /// Proposal title is empty
    InvalidTitle = 11,
    /// Voting weight must be greater than zero
    InvalidVotingWeight = 12,
    /// Proposal ID overflow
    ProposalIdOverflow = 13,
    /// Quorum not met
    QuorumNotMet = 14,
    /// Arithmetic overflow in vote weight calculation
    Overflow = 15,
}
