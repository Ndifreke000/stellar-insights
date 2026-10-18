# Separate Database Schemas for Testnet and Mainnet

Short and specific summary: Implement separate SQLite databases for testnet and mainnet data to prevent cross-network contamination.

---

## Project Context

This task is part of Phase 1: Backend Refactoring for the PayRaider multi-network architecture. The goal is to ensure complete data isolation between testnet and mainnet environments.

---

## Task Type

- [ ] Bug Fix
- [x] Feature
- [x] Refactor
- [ ] Enhancement
- [ ] Documentation
- [ ] Performance
- [ ] Security
- [ ] UI/UX
- [x] Backend
- [ ] DevOps

---

## Action Required

- [x] Create New File(s)
- [x] Update Existing File(s)
- [ ] Remove File(s)
- [ ] Research / Investigation
- [x] Testing

---

## File Locations Involved

### Existing Files

```bash
/backend/src/database.rs
/backend/src/main.rs
/backend/.env.example
```

### New Files

```bash
/backend/src/database/network_pool.rs
/backend/src/database/mod.rs
/backend/migrations/testnet/
/backend/migrations/mainnet/
```

---

## Detailed Requirements

### Acceptance Criteria

1. System MUST maintain separate SQLite database files: `payraider_testnet.db` and `payraider_mainnet.db`
2. System MUST provide separate connection pools for each network
3. System MUST route queries to correct database based on NetworkContext
4. System MUST prevent accidental cross-network queries
5. Migration scripts MUST run independently for each network
6. System MUST expose metrics per network (connection count, query time)

### Implementation Details

**Database structure:**
```rust
pub struct NetworkDatabasePool {
    testnet_pool: Pool<Sqlite>,
    mainnet_pool: Pool<Sqlite>,
}

impl NetworkDatabasePool {
    pub fn get_connection(&self, network: StellarNetwork) -> Result<PoolConnection<Sqlite>> {
        match network {
            StellarNetwork::Testnet => self.testnet_pool.acquire().await,
            StellarNetwork::Mainnet => self.mainnet_pool.acquire().await,
        }
    }
}
```

**Configuration:**
- Add `DATABASE_URL_TESTNET` and `DATABASE_URL_MAINNET` to .env
- Support fallback to single database for development

**Migration strategy:**
- Duplicate existing migrations for both networks
- Add network prefix to migration tracking table

---

## Dependencies

- Issue #001 (Network Context Middleware)

---

## Testing Requirements

- Test connection pool initialization for both networks
- Test query routing based on network context
- Test migration execution for both databases
- Test error handling for missing database files
- Load test connection pool under concurrent requests

---

## Estimated Effort

**6-8 hours**

---

## Priority

**P0 - Critical**

---

## Labels

`backend`, `database`, `network-switching`, `phase-1`
