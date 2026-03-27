# analytics

Core on-chain contract for recording Stellar Insights analytics snapshots.

## Purpose

Stores epoch-keyed SHA-256 hashes of off-chain analytics data on the Stellar network. Each submitted snapshot is immutably anchored on-chain, allowing anyone to verify the integrity of off-chain analytics exports.

## Key Invariants

- Epochs must be strictly increasing (monotonicity enforced) — prevents rollback attacks
- Only the authorized admin can submit snapshots
- Emergency pause halts writes while keeping reads available
- Governance contract can update admin or pause state via a passed proposal

## Public Interface

| Function | Description |
|---|---|
| `initialize(admin)` | One-time setup, sets the authorized admin |
| `submit_snapshot(epoch, hash, caller)` | Record a new snapshot hash for an epoch |
| `get_snapshot(epoch)` | Retrieve snapshot metadata for a specific epoch |
| `get_latest_snapshot()` | Retrieve the most recent snapshot |
| `get_snapshot_history()` | Full epoch → snapshot map |
| `get_latest_epoch()` | Latest recorded epoch number |
| `get_all_epochs()` | All epochs with stored snapshots |
| `set_admin(current_admin, new_admin)` | Transfer admin rights |
| `pause(caller)` / `unpause(caller)` | Emergency pause controls |
| `set_governance(caller, governance)` | Link a governance contract |
| `set_admin_by_governance(caller, new_admin)` | Governance-controlled admin update |
| `set_paused_by_governance(caller, paused)` | Governance-controlled pause update |

## Dependencies

- `soroban-sdk 21.0.0`
