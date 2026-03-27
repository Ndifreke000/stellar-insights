# governance

Decentralized governance contract for managing Stellar Insights protocol parameters and contract upgrades.

## Purpose

Allows token holders (or any authorized voters) to propose, vote on, and execute changes to governed contracts. Currently targets the `analytics` contract for parameter updates (admin address, pause state) and WASM upgrades.

## Proposal Lifecycle

```
create_proposal → [voting period] → finalize_proposal → execute_proposal
                        ↑
                   cast_vote (For / Against / Abstain)
```

A proposal passes if:
1. Total votes meet or exceed the configured quorum
2. `votes_for > votes_against`

## Public Interface

| Function | Description |
|---|---|
| `initialize(admin, quorum, voting_period)` | One-time setup |
| `create_proposal(proposer, title, target, wasm_hash)` | Submit an upgrade proposal |
| `create_parameter_proposal(proposer, title, target, action)` | Submit a parameter-update proposal |
| `cast_vote(voter, proposal_id, choice)` | Vote For / Against / Abstain |
| `finalize_proposal(proposal_id)` | Tally votes after voting period ends |
| `execute_proposal(caller, proposal_id)` | Execute a passed proposal |
| `get_proposal(proposal_id)` | Retrieve proposal details |
| `get_vote_tally(proposal_id)` | Retrieve vote counts |

## Parameter Actions

When creating a parameter proposal, the `action` field is one of:

- `SetAdmin(Address)` — update the admin of the target contract
- `SetPaused(bool)` — pause or unpause the target contract

## Dependencies

- `soroban-sdk 21.0.0`
- `analytics` (cross-contract calls for proposal execution)
