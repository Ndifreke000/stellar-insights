import { describe, it, expectTypeOf } from "vitest";
import type {
  ContractInfo,
  GovernanceProposal,
  Snapshot,
  SnapshotMetadata,
  VoteTally,
  ProposalStatus,
  VoteChoice,
  ParameterAction,
  PublicMetadata,
} from "./types.js";

describe("Soroban contract types", () => {
  it("GovernanceProposal has correct structure", () => {
    const proposal: GovernanceProposal = {
      id: 1n,
      proposer: "GAAAA",
      title: "Test Proposal",
      target_contract: "GBBBB",
      new_wasm_hash: "0000000000000000000000000000000000000000000000000000000000000000",
      status: { tag: "Active" },
      created_at: 1000n,
      voting_ends_at: 2000n,
    };
    expectTypeOf(proposal.id).toEqualTypeOf<bigint>();
    expectTypeOf(proposal.status.tag).toEqualTypeOf<"Active" | "Passed" | "Failed" | "Executed">();
  });

  it("VoteTally has correct structure", () => {
    const tally: VoteTally = {
      votes_for: 10n,
      votes_against: 5n,
      votes_abstain: 2n,
      total_voters: 17n,
    };
    expectTypeOf(tally.votes_for).toEqualTypeOf<bigint>();
  });

  it("Snapshot has correct structure", () => {
    const snapshot: Snapshot = {
      hash: "abcd1234",
      epoch: 100n,
      timestamp: 1000n,
    };
    expectTypeOf(snapshot.epoch).toEqualTypeOf<bigint>();
    expectTypeOf(snapshot.hash).toEqualTypeOf<string>();
  });

  it("SnapshotMetadata has correct structure", () => {
    const metadata: SnapshotMetadata = {
      epoch: 100n,
      timestamp: 1000n,
      hash: "abcd1234",
      submitter: "GAAAA",
      ledger_sequence: 1000,
      expires_at: 2000n,
    };
    expectTypeOf(metadata.ledger_sequence).toEqualTypeOf<number>();
    expectTypeOf(metadata.expires_at).toEqualTypeOf<bigint | null>();
  });

  it("ContractInfo has correct structure", () => {
    const info: ContractInfo = {
      metadata: {
        name: "Governance",
        version: "1.0.0",
        author: "Stellar Insights",
        description: "Governance contract",
        repository: "https://github.com/...",
        license: "MIT",
      },
      initialized: true,
      admin: "GAAAA",
      total_proposals: 10n,
    };
    expectTypeOf(info.total_proposals).toEqualTypeOf<bigint>();
    expectTypeOf(info.admin).toEqualTypeOf<string | null>();
  });

  it("ProposalStatus is a discriminated union", () => {
    const status1: ProposalStatus = { tag: "Active" };
    const status2: ProposalStatus = { tag: "Passed" };
    expectTypeOf(status1.tag).toEqualTypeOf<"Active" | "Passed" | "Failed" | "Executed">();
  });

  it("VoteChoice is a discriminated union", () => {
    const choice: VoteChoice = { tag: "For" };
    expectTypeOf(choice.tag).toEqualTypeOf<"For" | "Against" | "Abstain">();
  });

  it("ParameterAction is a discriminated union", () => {
    const action1: ParameterAction = { tag: "SetAdmin", values: ["GAAAA"] };
    const action2: ParameterAction = { tag: "SetPaused", values: [true] };
    expectTypeOf(action1.values).toEqualTypeOf<[string] | [boolean]>();
  });

  it("PublicMetadata has correct structure", () => {
    const metadata: PublicMetadata = {
      name: "Governance",
      version: "1.0.0",
      author: "Stellar Insights",
      description: "Governance contract",
      repository: "https://github.com/...",
      license: "MIT",
    };
    expectTypeOf(metadata.name).toEqualTypeOf<string>();
  });
});
