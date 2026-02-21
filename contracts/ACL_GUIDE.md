# Access Control List (ACL) for Soroban Contracts

## Overview
Fine-grained access control system for Stellar smart contracts with role-based permissions.

## Features
- **Role-Based Access Control (RBAC)**: Admin, Operator, Viewer roles
- **Function-Level Permissions**: Grant specific functions to roles
- **Dynamic Management**: Add/revoke roles and permissions at runtime
- **Admin Override**: Admins have access to all functions

## Contract Functions

### `initialize(admin: Address)`
Initialize ACL with admin user.

### `grant_role(caller: Address, user: Address, role: Role)`
Grant a role to a user (admin only).

### `revoke_role(caller: Address, user: Address, role: Role)`
Revoke a role from a user (admin only).

### `has_role(user: Address, role: Role) -> bool`
Check if user has a specific role.

### `grant_permission(caller: Address, role: Role, function: Symbol)`
Grant function permission to a role (admin only).

### `check_permission(user: Address, function: Symbol) -> bool`
Check if user can execute a function.

## Usage Example

```rust
// Deploy ACL contract
let acl_id = env.register_contract_wasm(None, acl_wasm);
let acl = AccessControlClient::new(&env, &acl_id);

// Initialize with admin
acl.initialize(&admin);

// Grant operator role
acl.grant_role(&admin, &user, &Role::Operator);

// Grant permission to function
acl.grant_permission(&admin, &Role::Operator, &symbol_short!("transfer"));

// Check permission
if acl.check_permission(&user, &symbol_short!("transfer")) {
    // Execute function
}
```

## Integration

```rust
fn protected_function(env: Env, caller: Address) {
    caller.require_auth();
    
    let acl: Address = env.storage().instance().get(&symbol_short!("acl")).unwrap();
    let acl_client = AccessControlClient::new(&env, &acl);
    
    if !acl_client.check_permission(&caller, &symbol_short!("protected")) {
        panic!("Access denied");
    }
    
    // Function logic
}
```

## Build

```bash
cd contracts/access-control
cargo build --target wasm32-unknown-unknown --release
```

## Test

```bash
cargo test
```

## Roles

- **Admin**: Full access, can manage roles and permissions
- **Operator**: Execute permitted functions
- **Viewer**: Read-only access (custom implementation)
