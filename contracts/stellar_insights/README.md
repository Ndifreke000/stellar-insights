# stellar_insights _(prototype)_

> **Note:** This is the original prototype of the analytics snapshot contract. The canonical production contract is [`analytics/`](../analytics/). This directory is retained for historical reference only.

## Purpose

Records analytics snapshot hashes on-chain with epoch-based versioning, admin authorization, and emergency pause. Functionally equivalent to `analytics/` but without governance integration.

## Differences from `analytics/`

- No governance contract integration (`set_governance`, `set_admin_by_governance`, `set_paused_by_governance`)
- Uses a typed `Error` enum with `Result` return types instead of panics
- Emits structured events via a dedicated `events` module

## Dependencies

- `soroban-sdk 21.0.0`
