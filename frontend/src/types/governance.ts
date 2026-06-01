export type ProposalStatus = 'draft' | 'active' | 'passed' | 'failed' | 'executed';

export type VoteChoice = 'for' | 'against' | 'abstain';

export interface Proposal {
  id: string;
  title: string;
  description: string;
  proposalType: string;
  targetContract: string;
  newWasmHash: string;
  status: ProposalStatus;
  createdBy: string;
  onChainId: string | null;
  votingEndsAt: string;
  finalizedAt: string | null;
  executedAt: string | null;
  createdAt: string;
  updatedAt: string;
  votesFor: number;
  votesAgainst: number;
  votesAbstain: number;
  totalVoters: number;
}

export interface ProposalsListResponse {
  proposals: Proposal[];
  total: number;
  limit: number;
  offset: number;
}

export interface Vote {
  id: string;
  proposalId: string;
  voterAddress: string;
  choice: VoteChoice;
  txHash: string;
  votedAt: string;
}

export interface Comment {
  id: string;
  proposalId: string;
  authorAddress: string;
  content: string;
  createdAt: string;
}

export interface CreateProposalRequest {
  title: string;
  description: string;
  targetContract: string;
  newWasmHash: string;
}

export interface CastVoteRequest {
  choice: VoteChoice;
}
