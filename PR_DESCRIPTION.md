## Description

Implements SEP-10 (Stellar Web Authentication) to enable secure wallet-based authentication using Stellar accounts without passwords.

Closes #233

## Overview

This PR adds complete SEP-10 authentication support, allowing users to authenticate using their Stellar wallets (Freighter, Albedo, xBull, Rabet) instead of traditional passwords.

## Backend Changes

### Core Implementation
- **SEP-10 Service** (`backend/src/auth/sep10_simple.rs`)
  - Challenge generation with unique nonces
  - Challenge verification and signature validation
  - Session management with Redis
  - Replay protection
  - Time-bound challenges (5 minutes validity)
  - 7-day session expiry

### API Endpoints (`backend/src/api/sep10.rs`)
- `GET /api/sep10/info` - Server information
- `POST /api/sep10/auth` - Request challenge transaction
- `POST /api/sep10/verify` - Verify signed challenge
- `POST /api/sep10/logout` - Invalidate session

### Middleware (`backend/src/auth/sep10_middleware.rs`)
- Token validation for protected routes
- User extraction from authenticated sessions

### Tests (`backend/tests/sep10_test.rs`)
- Challenge generation tests
- Validation tests
- Error handling tests

## Frontend Changes

### Authentication Service (`frontend/src/services/sep10Auth.ts`)
- Complete SEP-10 authentication flow
- Multi-wallet support (Freighter, Albedo, xBull, Rabet)
- Challenge request and signing
- Verification handling
- Client-side validation

### Enhanced Wallet Context (`frontend/src/components/lib/wallet-context.tsx`)
- Wallet connection management
- SEP-10 authentication state
- Session persistence with localStorage
- Token expiry handling
- Multi-device support

### Updated UI (`frontend/src/components/wallet-connect.tsx`)
- Visual authentication status indicator
- Authenticate button
- Logout functionality
- Improved user experience

### Demo Page (`frontend/src/app/sep10-demo/page.tsx`)
- Interactive demonstration
- Step-by-step authentication flow
- Testing interface
- Educational content

### Tests (`frontend/src/services/__tests__/sep10Auth.test.ts`)
- Service method tests
- Wallet integration tests
- Error handling tests

## Documentation

Comprehensive documentation added:
- **SEP10_AUTHENTICATION.md** - Complete technical documentation
- **SEP10_SETUP.md** - Quick setup guide
- **SEP10_MIGRATION_GUIDE.md** - Migration from password auth
- **SEP10_IMPLEMENTATION_SUMMARY.md** - Implementation overview
- **SEP10_CHECKLIST.md** - Deployment checklist
- **SEP10_QUICK_REFERENCE.md** - Developer quick reference

## Key Features

✅ **Secure Authentication**
- Cryptographic signature verification
- Replay protection with nonce consumption
- Time-bound challenges and sessions
- Domain binding and validation

✅ **Multi-Wallet Support**
- Freighter (most popular)
- Albedo (web-based)
- xBull (mobile & desktop)
- Rabet (browser extension)

✅ **User Experience**
- One-click authentication
- No passwords required
- Visual authentication status
- Persistent sessions (7 days)
- Multi-device support

✅ **Developer Experience**
- Clean API design
- Comprehensive documentation
- Example implementations
- Test coverage
- Easy integration

✅ **Production Ready**
- Comprehensive error handling
- Security best practices
- Performance optimized
- Standards compliant (SEP-10)

## Security Features

- **Replay Protection**: Unique nonces consumed after verification
- **Time Bounds**: 5-minute challenge validity, 7-day sessions
- **Domain Validation**: Home domain verification
- **Secure Sessions**: Redis-backed session storage
- **Token Security**: Cryptographically secure tokens

## Compatibility

- ✅ Non-breaking changes
- ✅ Coexists with existing password authentication
- ✅ Separate endpoints (`/api/sep10/*` vs `/api/auth/*`)
- ✅ Separate middleware
- ✅ Shared Redis infrastructure

## Testing

### Backend
```bash
cd backend
cargo test sep10
```

### Frontend
```bash
cd frontend
npm test sep10Auth
```

### Manual Testing
1. Start Redis: `redis-server`
2. Start backend: `cd backend && cargo run`
3. Start frontend: `cd frontend && npm run dev`
4. Visit: `http://localhost:3000/sep10-demo`

## Configuration Required

### Backend Environment Variables
```bash
SEP10_SERVER_PUBLIC_KEY=GXXXXXX...
STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
SEP10_HOME_DOMAIN=localhost
REDIS_URL=redis://localhost:6379
```

### Frontend Environment Variables
```bash
NEXT_PUBLIC_API_URL=http://localhost:8080
```

## Migration Path

For existing applications:
1. Deploy SEP-10 alongside existing auth
2. Allow users to link Stellar accounts
3. Gradually migrate routes
4. Eventually deprecate password auth (optional)

See `SEP10_MIGRATION_GUIDE.md` for detailed migration strategy.

## Standards Compliance

Follows [SEP-10 specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md):
- Challenge transaction structure
- Time bounds validation
- Signature verification
- Replay protection
- Domain binding

## Files Changed

### Backend
- `backend/src/auth/sep10_simple.rs` (new)
- `backend/src/auth/sep10_middleware.rs` (new)
- `backend/src/api/sep10.rs` (new)
- `backend/src/auth.rs` (modified)
- `backend/src/api/mod.rs` (modified)
- `backend/Cargo.toml` (modified)
- `backend/tests/sep10_test.rs` (new)

### Frontend
- `frontend/src/services/sep10Auth.ts` (new)
- `frontend/src/components/lib/wallet-context.tsx` (modified)
- `frontend/src/components/wallet-connect.tsx` (modified)
- `frontend/src/app/sep10-demo/page.tsx` (new)
- `frontend/src/services/__tests__/sep10Auth.test.ts` (new)

### Documentation
- `backend/SEP10_AUTHENTICATION.md` (new)
- `SEP10_SETUP.md` (new)
- `SEP10_MIGRATION_GUIDE.md` (new)
- `SEP10_IMPLEMENTATION_SUMMARY.md` (new)
- `SEP10_CHECKLIST.md` (new)
- `SEP10_QUICK_REFERENCE.md` (new)

## Next Steps

After merging:
1. Configure environment variables
2. Test with real Stellar wallets
3. Deploy to staging
4. Gather user feedback
5. Plan production rollout

## References

- [SEP-10 Specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md)
- [Stellar Developer Docs](https://developers.stellar.org)
- Issue #233
