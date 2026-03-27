# Stellar Insights — Smart Contracts

Soroban smart contracts powering the on-chain layer of Stellar Insights. All contracts are written in Rust using [soroban-sdk 21.0.0](https://docs.rs/soroban-sdk).

## Architecture Overview

```
contracts/
├── analytics/          # Core snapshot storage with governance integration
├── governance/         # Decentralized voting and proposal execution
├── access-control/     # Reusable role-based access control (RBAC) library
└── snapshot-contract/  # Advanced snapshot management with upgrade support
```

> `stellar_insights/` is the original prototype of the analytics contract and is kept for reference. The canonical implementation is `analytics/`.

## Contract Dependency Graph

```
governance  ──depends on──▶  analytics
snapshot-contract  ──depends on──▶  access-control
```

## Contracts

### `analytics/`
The primary on-chain contract for recording analytics snapshots. Stores epoch-keyed SHA-256 hashes of off-chain analytics data, enforcing strict epoch monotonicity to prevent rollback attacks. Supports emergency pause and governance-controlled parameter updates.

See [`analytics/README.md`](./analytics/README.md).

### `governance/`
Decentralized governance contract for managing protocol parameters and contract upgrades. Supports proposal creation, three-choice voting (For / Against / Abstain), quorum enforcement, and execution of passed proposals against target contracts (e.g. `analytics`).

See [`governance/README.md`](./governance/README.md).

### `access-control/`
Reusable RBAC library contract. Defines three roles (Admin, Operator, Viewer) and a permission system mapping roles to function symbols. Used as a dependency by `snapshot-contract`.

See [`access-control/README.md`](./access-control/README.md).

### `snapshot-contract/`
Advanced snapshot storage contract with contract versioning, upgrade/migration support, and emergency stop controls. Depends on `access-control` for permission checks.

See [`snapshot-contract/README.md`](./snapshot-contract/README.md).

### `stellar_insights/` _(prototype — not in active workspace)_
Original prototype of the analytics snapshot contract. Functionally similar to `analytics/` but without governance integration. Retained for historical reference; `analytics/` is the production contract.

## Removed Contracts

The following directories were removed as they served no production purpose:

| Directory | Reason |
|---|---|
| `example-contract/` | Hello-world template from Soroban scaffolding, not part of the system |
| `secure-contract/` | Incomplete demo (no `Cargo.toml`, not in workspace) showing ACL integration pattern |

## Building

```bash
# Build all workspace contracts
cd contracts
cargo build --release --target wasm32-unknown-unknown

# Build a single contract
cargo build -p analytics --release --target wasm32-unknown-unknown
```

## Testing

```bash
cd contracts
cargo test
```
