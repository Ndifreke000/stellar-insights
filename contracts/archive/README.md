# Archived Soroban contracts

Everything in this directory is **excluded** from the `contracts/` Cargo
workspace (`exclude = ["archive"]`) and is **not** built, linted, fuzzed,
benchmarked or tested in CI. Changes under `contracts/archive/**` do not
trigger the contract workflows.

| Directory | What it was |
|---|---|
| `access-control` | Generic role registry |
| `analytics` | Earlier snapshot contract, superseded by `payraider` |
| `escrow` | Payment escrow |
| `governance`, `governance-voting` | On-chain proposals and voting |
| `multi-admin`, `multi-sig-wallet` | Multi-party admin / wallet |
| `pausable` | Stand-alone pause module |
| `snapshot-verification-rewards` | Rewards for third-party snapshot verification |
| `time-locked-transactions` | Delayed execution |
| `token-swap` | Token swap |
| `upgrade` | Upgrade helper |
| `tests/` | Fuzz and testnet integration tests for the contracts above |

Only `payraider` is part of the off-ramp product. The backend talks to it via
`SNAPSHOT_CONTRACT_ID`; nothing outside `contracts/` references, deploys or
calls the crates above. They were first dropped from the workspace in #2227
and physically moved here in #2598.

## Restoring a contract

1. Move the directory back to `contracts/<name>` (and its tests from
   `archive/tests/` back to `contracts/tests/`).
2. Add it to `members` in `contracts/Cargo.toml`.
3. Wire it into a real product flow before merging — an unused contract is
   audit surface with no benefit.

The crates still use `soroban-sdk = { workspace = true }`, so to build one in
place you must restore it to the workspace first.
