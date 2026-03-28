# access-control

Reusable role-based access control (RBAC) library contract for Soroban.

## Purpose

Provides a generic permission system that other contracts can depend on. Defines three roles and a function-level permission model, allowing fine-grained access control without duplicating auth logic across contracts.

## Roles

| Role | Description |
|---|---|
| `Admin` | Full access; implicitly passes all permission checks |
| `Operator` | Operational access; permissions granted explicitly per function |
| `Viewer` | Read-only access; permissions granted explicitly per function |

## Public Interface

| Function | Description |
|---|---|
| `initialize(admin)` | One-time setup, grants Admin role to the initializer |
| `grant_role(caller, user, role)` | Assign a role to a user (Admin only) |
| `revoke_role(caller, user, role)` | Remove a role from a user (Admin only) |
| `has_role(user, role)` | Check if a user holds a role |
| `grant_permission(caller, role, function)` | Allow a role to call a function symbol |
| `check_permission(user, function)` | Returns true if user may call the function |

## Usage

Other contracts import this as a dependency and call `check_permission` via cross-contract invocation:

```rust
// In dependent contract's Cargo.toml
access-control = { path = "../access-control" }
```

## Dependencies

- `soroban-sdk 21.0.0`
