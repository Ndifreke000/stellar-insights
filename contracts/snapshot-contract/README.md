# snapshot-contract

Advanced snapshot storage contract with contract versioning, upgrade support, and emergency controls.

## Purpose

Stores epoch-keyed snapshot hashes with additional operational features beyond the core `analytics` contract: contract versioning, WASM upgrade preparation, admin transfer, and an emergency stop mechanism (distinct from pause).

## Differences from `analytics/`

| Feature | `analytics` | `snapshot-contract` |
|---|---|---|
| Governance integration | Yes | No |
| Contract versioning | No | Yes |
| WASM upgrade support | No | Yes |
| Emergency stop (halt) | No | Yes |
| Emergency pause | Yes | Yes |
| ACL dependency | No | Yes (`access-control`) |

## Public Interface

| Function | Description |
|---|---|
| `initialize(admin)` | One-time setup |
| `submit_snapshot(epoch, hash, caller)` | Record a snapshot hash |
| `get_snapshot(epoch)` | Retrieve snapshot by epoch |
| `get_latest_snapshot()` | Retrieve the most recent snapshot |
| `verify_snapshot(epoch, hash)` | Verify a hash matches stored value |
| `transfer_admin(new_admin)` | Transfer admin rights |
| `prepare_upgrade(new_wasm_hash)` | Validate and stage a WASM upgrade |
| `stop_contract()` / `resume_contract()` | Emergency halt controls |
| `version()` | Current contract version |
| `check_permission(addr, function)` | ACL permission check |

## Dependencies

- `soroban-sdk 21.0.0`
- `access-control` (for permission checks)
