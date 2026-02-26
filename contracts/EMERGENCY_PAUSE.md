# Emergency Pause Functionality

All Stellar Insights smart contracts now include emergency pause functionality to protect against security incidents and enable rapid response to critical issues.

## Overview

The emergency pause feature allows authorized administrators to temporarily halt write operations on contracts while maintaining read access. This is crucial for:

- **Security Incidents**: Quickly stop malicious activity
- **Bug Discovery**: Prevent exploitation of discovered vulnerabilities
- **Maintenance**: Safely perform critical updates
- **Regulatory Compliance**: Meet emergency shutdown requirements

## Features

### Pause State Management
- **Pause**: Immediately stops all write operations (snapshot submissions)
- **Unpause**: Resumes normal contract operations
- **Status Check**: Query current pause state without authentication

### Access Control
- Only the contract admin can pause/unpause
- Requires authentication via `require_auth()`
- Admin address set during contract initialization

### Read Operations Unaffected
- All read functions remain available during pause
- Historical data access continues normally
- Status queries work as expected

## Contract-Specific Implementation

### 1. Stellar Insights Contract (`stellar_insights`)

**New Functions:**
```rust
// Pause the contract (admin only)
pub fn pause(env: Env, caller: Address) -> Result<(), Error>

// Unpause the contract (admin only)
pub fn unpause(env: Env, caller: Address) -> Result<(), Error>

// Check pause status (public)
pub fn is_paused(env: Env) -> bool
```

**Error Handling:**
- Returns `Error::ContractPaused` when attempting operations on paused contract
- Returns `Error::UnauthorizedCaller` if non-admin tries to pause/unpause
- Returns `Error::AdminNotSet` if admin not initialized

**Affected Operations:**
- `submit_snapshot()` - Blocked when paused

### 2. Analytics Contract (`analytics`)

**New Functions:**
```rust
// Pause the contract (admin only)
pub fn pause(env: Env, caller: Address)

// Unpause the contract (admin only)
pub fn unpause(env: Env, caller: Address)

// Check pause status (public)
pub fn is_paused(env: Env) -> bool
```

**Panic Conditions:**
- Panics with "Contract is paused for emergency maintenance" when attempting operations on paused contract
- Panics with "Unauthorized: only the admin can pause the contract" if non-admin tries to pause/unpause

**Affected Operations:**
- `submit_snapshot()` - Blocked when paused

### 3. Snapshot Contract (`snapshot-contract`)

**New Functions:**
```rust
// Initialize contract with admin (must be called first)
pub fn initialize(env: Env, admin: Address)

// Pause the contract (admin only)
pub fn pause(env: Env, caller: Address)

// Unpause the contract (admin only)
pub fn unpause(env: Env, caller: Address)

// Check pause status (public)
pub fn is_paused(env: Env) -> bool

// Get admin address (public)
pub fn get_admin(env: Env) -> Option<Address>
```

**Panic Conditions:**
- Panics with "Contract is paused for emergency maintenance" when attempting operations on paused contract
- Panics with "Unauthorized: only the admin can pause the contract" if non-admin tries to pause/unpause

**Affected Operations:**
- `submit_snapshot()` - Blocked when paused

## Usage Examples

### Pausing a Contract

```rust
use soroban_sdk::{Address, Env};

// Admin pauses the contract
let admin = Address::from_string("GADMIN...");
contract.pause(&env, &admin);

// Verify pause state
assert!(contract.is_paused(&env));
```

### Unpausing a Contract

```rust
// Admin unpauses the contract
let admin = Address::from_string("GADMIN...");
contract.unpause(&env, &admin);

// Verify active state
assert!(!contract.is_paused(&env));
```

### Checking Pause State

```rust
// Anyone can check pause state
let is_paused = contract.is_paused(&env);

if is_paused {
    println!("Contract is currently paused");
} else {
    println!("Contract is active");
}
```

### Handling Paused State

```rust
// Stellar Insights Contract (Result-based)
match contract.submit_snapshot(&env, epoch, hash, caller) {
    Ok(timestamp) => println!("Snapshot submitted at {}", timestamp),
    Err(Error::ContractPaused) => println!("Contract is paused, try again later"),
    Err(e) => println!("Error: {:?}", e),
}

// Analytics/Snapshot Contracts (Panic-based)
// Wrap in try-catch or check is_paused() first
if !contract.is_paused(&env) {
    contract.submit_snapshot(&env, epoch, hash, caller);
} else {
    println!("Contract is paused, cannot submit");
}
```

## Emergency Response Workflow

### 1. Incident Detection
```bash
# Monitor contract events and logs
# Detect suspicious activity or vulnerability
```

### 2. Immediate Pause
```rust
// Admin immediately pauses the contract
contract.pause(&env, &admin_address);
```

### 3. Investigation
```rust
// Read operations still work for forensics
let latest = contract.latest_snapshot(&env);
let history = contract.get_snapshot_history(&env);
// Analyze data to understand the incident
```

### 4. Resolution
```bash
# Deploy fixes or take corrective action
# Verify security measures are in place
```

### 5. Resume Operations
```rust
// Admin unpauses after resolution
contract.unpause(&env, &admin_address);
```

## Security Considerations

### Admin Key Management
- Store admin private keys in secure hardware wallets
- Use multi-signature schemes for production
- Implement key rotation procedures
- Never expose admin keys in code or logs

### Monitoring
- Set up alerts for pause/unpause events
- Monitor contract state regularly
- Log all administrative actions
- Track failed pause attempts

### Testing
- Test pause functionality in development
- Verify all write operations are blocked when paused
- Confirm read operations continue during pause
- Practice emergency response procedures

### Governance
- Document pause decision criteria
- Establish clear escalation procedures
- Define maximum pause duration policies
- Require post-incident reports

## Integration with Backend

The backend should monitor pause state and handle gracefully:

```rust
// Check pause state before submitting
if contract.is_paused(&env) {
    return Err("Contract is paused for maintenance");
}

// Handle pause errors
match contract.submit_snapshot(&env, epoch, hash, caller) {
    Err(Error::ContractPaused) => {
        // Queue for retry after unpause
        queue_for_retry(epoch, hash);
    }
    // ... handle other cases
}
```

## Deployment Checklist

- [ ] Initialize contract with secure admin address
- [ ] Verify admin can pause/unpause
- [ ] Test that write operations fail when paused
- [ ] Confirm read operations work when paused
- [ ] Set up monitoring for pause events
- [ ] Document admin contact procedures
- [ ] Establish emergency response team
- [ ] Create runbook for pause scenarios

## Limitations

- Pause only affects write operations (snapshot submissions)
- Read operations cannot be paused (by design)
- Admin must be set during initialization
- No time-based auto-unpause (manual unpause required)
- Single admin model (consider multi-sig for production)

## Future Enhancements

Potential improvements for future versions:

1. **Multi-Signature Admin**: Require multiple admins to pause/unpause
2. **Time-Locked Pause**: Automatic unpause after specified duration
3. **Partial Pause**: Pause specific functions while allowing others
4. **Pause Reasons**: Store reason codes for audit trails
5. **Emergency Contacts**: On-chain notification system
6. **Gradual Resume**: Rate-limited operations after unpause

## Support

For questions or issues related to emergency pause functionality:

- Review contract source code in `contracts/` directory
- Check test files for usage examples
- Consult security documentation
- Contact the development team for critical incidents
