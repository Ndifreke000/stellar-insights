# Snapshot Contract - Multi-Admin & Role-Based Permissions

## Overview
This contract supports multiple admin addresses and role-based permissions for secure and flexible management.

## Features
- **Multiple Admins:**
  - Initialize with one or more admin addresses.
  - Add or remove admins (cannot remove last admin).
- **Role-Based Permissions:**
  - Admins can perform privileged actions (upgrade, migrate, add/remove admins).
  - Role checks are extensible for future roles and permissions.

## Key Methods
- `initialize(admins: Vec<Address>)`: Initialize contract with multiple admins.
- `get_admins() -> Vec<Address>`: Get all current admin addresses.
- `add_admin(caller: Address, new_admin: Address)`: Add a new admin (caller must be admin).
- `remove_admin(caller: Address, admin_to_remove: Address)`: Remove an admin (caller must be admin, cannot remove last admin).
- `is_admin(env: Env, addr: Address) -> bool`: Check if address is an admin.
- `check_permission(env: Env, addr: Address, function: &str) -> bool`: Check if address has permission for a function (currently, admin = all permissions).

## Usage
- Only admins can upgrade, migrate, or manage admin addresses.
- All admin changes are logged as contract events.

## Extending Permissions
- Integrate with the `access-control` contract for more granular roles and permissions.

## Events
- `INIT`: Contract initialized with admins.
- `ADM_ADD`: Admin added.
- `ADM_REM`: Admin removed.
- `UPGRADED`: Contract upgraded.
- `MIGRATED`: Migration performed.

---
For more details, see the source code and tests.
