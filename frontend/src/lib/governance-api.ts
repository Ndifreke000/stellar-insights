import { api } from "./api";
import type {
  Proposal,
  ProposalsListResponse,
  CreateProposalRequest,
  CastVoteRequest,
  Vote,
  Comment,
  ProposalStatus,
} from "@/types/governance";

export async function getProposals(
  status?: ProposalStatus,
  limit?: number,
  offset?: number,
): Promise<ProposalsListResponse> {
  const params = new URLSearchParams();
  if (status) params.append("status", status);
  if (limit !== undefined) params.append("limit", limit.toString());
  if (offset !== undefined) params.append("offset", offset.toString());
  const query = params.toString();
  return api.get<ProposalsListResponse>(
    `/governance/proposals${query ? `?${query}` : ""}`,
  );
}

export async function getProposal(id: string): Promise<Proposal> {
  return api.get<Proposal>(`/governance/proposals/${id}`);
}

export async function createProposal(
  request: CreateProposalRequest,
  authToken: string,
): Promise<Proposal> {
  return api.post<Proposal>("/governance/proposals", request, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
}

export async function castVote(
  proposalId: string,
  request: CastVoteRequest,
  authToken: string,
): Promise<Vote> {
  return api.post<Vote>(
    `/governance/proposals/${proposalId}/vote`,
    request,
    { headers: { Authorization: `Bearer ${authToken}` } },
  );
}

export async function getVotes(proposalId: string): Promise<Vote[]> {
  return api.get<Vote[]>(`/governance/proposals/${proposalId}/votes`);
}

export async function hasVoted(
  proposalId: string,
  address: string,
): Promise<{ has_voted: boolean }> {
  return api.get<{ has_voted: boolean }>(
    `/governance/proposals/${proposalId}/has-voted/${address}`,
  );
}

export async function addComment(
  proposalId: string,
  content: string,
  authToken: string,
): Promise<Comment> {
  return api.post<Comment>(
    `/governance/proposals/${proposalId}/comments`,
    { content },
    { headers: { Authorization: `Bearer ${authToken}` } },
  );
}

export async function getComments(proposalId: string): Promise<Comment[]> {
  return api.get<Comment[]>(`/governance/proposals/${proposalId}/comments`);
}
