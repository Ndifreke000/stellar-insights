# Implement Network Context Middleware

Short and specific summary: Create middleware to extract and validate network context from HTTP headers.

---

## Project Context

This task is part of Phase 1: Backend Refactoring for the PayRaider multi-network architecture. The goal is to enable runtime network switching between testnet and mainnet by implementing proper network context management in the Rust backend.

---

## Task Type

- [ ] Bug Fix
- [x] Feature
- [ ] Refactor
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
- [ ] Testing

---

## File Locations Involved

### Existing Files

```bash
/backend/src/main.rs
/backend/src/error.rs
```

### New Files

```bash
/backend/src/middleware/network_context.rs
/backend/src/middleware/mod.rs
```

---

## Detailed Requirements

### Acceptance Criteria

1. Middleware MUST extract `X-Stellar-Network` header from incoming requests
2. Middleware MUST validate network value against allowed values: `testnet`, `mainnet`
3. Middleware MUST reject requests with invalid or missing network context (HTTP 400)
4. Middleware MUST inject validated network context into request extensions
5. Middleware MUST log network context with request ID for debugging
6. Middleware MUST support default network fallback (configurable via env var)

### Implementation Details

**NetworkContext struct:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StellarNetwork {
    Testnet,
    Mainnet,
}

pub struct NetworkContext {
    pub network: StellarNetwork,
}
```

**Middleware logic:**
- Extract header using `req.headers().get("x-stellar-network")`
- Parse and validate value
- Store in request extensions: `req.extensions_mut().insert(NetworkContext { network })`
- Return 400 Bad Request for invalid values

**Error handling:**
- Add `InvalidNetworkContext` variant to AppError enum
- Return descriptive error messages

---

## Dependencies

- None (foundational task)

---

## Testing Requirements

- Unit tests for network parsing (valid/invalid values)
- Integration tests for middleware behavior
- Test default network fallback
- Test error responses for invalid headers

---

## Estimated Effort

**2-3 hours**

---

## Priority

**P0 - Critical** (blocks all other network-aware features)

---

## Labels

`backend`, `middleware`, `network-switching`, `phase-1`
