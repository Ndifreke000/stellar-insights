# Archived Soroban contracts

These directories are **not** workspace members and are **not** built or tested in CI:

- `access-control`, `analytics`, `escrow`, `governance`, `governance-voting`
- `multi-admin`, `multi-sig-wallet`, `pausable`, `snapshot-verification-rewards`
- `time-locked-transactions`, `token-swap`, `upgrade`

They were removed from the active workspace in #2227 because nothing outside
`contracts/` references, deploys, or calls them. The backend only integrates
with `payraider` via `SNAPSHOT_CONTRACT_ID`.

To restore a contract to the workspace, add it back to `contracts/Cargo.toml`
`members` and wire it into a real product flow first.
