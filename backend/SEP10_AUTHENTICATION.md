# SEP-10 Stellar Web Authentication

This document describes the implementation of SEP-10 (Stellar Web Authentication) in the Stellar Analytics platform.

## Overview

SEP-10 provides a standard way for Stellar wallets to authenticate with web services using cryptographic signatures instead of passwords. This implementation follows the [SEP-10 specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md).

## Architecture

### Backend Components

1. **Sep10Service** (`backend/src/auth/sep10.rs`)
   - Generates challenge transactions
   - Verifies signed transactions
   - Manages authenticated sessions
   - Implements replay protection

2. **API Endpoints** (`backend/src/api/sep10.rs`)
   - `GET /api/sep10/info` - Server information
   - `POST /api/sep10/auth` - Request challenge
   - `POST /api/sep10/verify` - Verify signed challenge
   - `POST /api/sep10/logout` - Invalidate session

3. **Middleware** (`backend/src/auth/sep10_middleware.rs`)
   - Validates SEP-10 session tokens
   - Extracts authenticated user information
   - Protects authenticated routes

### Frontend Components

1. **Sep10AuthService** (`frontend/src/services/sep10Auth.ts`)
   - Requests challenges from server
   - Signs challenges using wallet integrations
   - Verifies signed transactions
   - Manages authentication flow

2. **WalletContext** (`frontend/src/components/lib/wallet-context.tsx`)
   - Manages wallet connection state
   - Handles SEP-10 authentication flow
   - Stores session tokens
   - Provides authentication hooks

3. **WalletButton** (`frontend/src/components/wallet-connect.tsx`)
   - UI for wallet connection
   - Authentication trigger
   - Session status display

## Authentication Flow

### 1. Challenge Request

Client requests a challenge transaction:

```typescript
const challenge = await sep10AuthService.requestChallenge({
  account: 'GCLIENT...',
  home_domain: 'example.com',
});
```

Server generates a challenge transaction with:
- Source account: Server's public key
- Sequence number: 0
- Time bounds: 5 minutes validity
- ManageData operation with random nonce
- Server signature

### 2. Challenge Signing

Client signs the challenge using their Stellar wallet:

```typescript
const signedXdr = await sep10AuthService.signChallenge(
  challenge.transaction,
  challenge.network_passphrase,
  publicKey
);
```

Supported wallets:
- Freighter
- Albedo
- xBull
- Rabet

### 3. Challenge Verification

Client submits signed transaction to server:

```typescript
const result = await sep10AuthService.verifyChallenge(signedXdr);
// Returns: { token: 'jwt-token', expires_in: 604800 }
```

Server validates:
- Transaction structure
- Time bounds (not expired)
- Sequence number (must be 0)
- Network passphrase
- Server signature presence
- Client signature validity
- Nonce uniqueness (replay protection)

### 4. Session Management

Upon successful verification:
- Server creates session in Redis
- Returns JWT token
- Client stores token in localStorage
- Token used for authenticated requests

## Security Features

### Replay Protection

Each challenge contains a unique random nonce stored in Redis. Once a challenge is verified, the nonce is consumed and cannot be reused.

```rust
async fn validate_and_consume_challenge(&self, account: &str, nonce: &str) -> Result<()> {
    // Check if challenge exists
    // Delete challenge (consume it)
}
```

### Time Bounds Validation

Challenges are valid for 5 minutes. Server validates:
- Current time is within bounds
- Duration is between 5-15 minutes
- Transaction hasn't expired

### Domain Binding

Challenges can include:
- `home_domain`: Server's domain (validated)
- `client_domain`: Wallet's domain (optional)

### Multi-Device Support

Sessions are stored per token, allowing:
- Multiple active sessions per account
- Independent session management
- Device-specific logout

## Configuration

### Backend Environment Variables

```bash
# Server signing key (required)
SEP10_SERVER_SECRET=SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# Network passphrase (required)
STELLAR_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"

# Home domain (required)
SEP10_HOME_DOMAIN=example.com

# Redis connection (required for production)
REDIS_URL=redis://localhost:6379

# JWT secret (for compatibility with existing auth)
JWT_SECRET=your-secret-key
```

### Frontend Environment Variables

```bash
# API base URL
NEXT_PUBLIC_API_URL=http://localhost:8080
```

## Usage Examples

### Backend: Protect Routes with SEP-10

```rust
use axum::{Router, routing::get};
use axum::middleware;

let protected_routes = Router::new()
    .route("/api/protected", get(protected_handler))
    .layer(middleware::from_fn_with_state(
        sep10_service.clone(),
        sep10_auth_middleware
    ));
```

### Backend: Access Authenticated User

```rust
use axum::extract::Extension;
use crate::auth::sep10_middleware::Sep10User;

async fn protected_handler(
    Extension(user): Extension<Sep10User>,
) -> String {
    format!("Hello, {}!", user.account)
}
```

### Frontend: Authenticate User

```typescript
import { useWallet } from '@/components/lib/wallet-context';

function MyComponent() {
  const { authenticateWithSep10, isAuthenticated, authToken } = useWallet();

  const handleAuth = async () => {
    try {
      await authenticateWithSep10();
      console.log('Authenticated!');
    } catch (error) {
      console.error('Authentication failed:', error);
    }
  };

  return (
    <button onClick={handleAuth} disabled={isAuthenticated}>
      {isAuthenticated ? 'Authenticated' : 'Authenticate'}
    </button>
  );
}
```

### Frontend: Make Authenticated Requests

```typescript
const { authToken } = useWallet();

const response = await fetch('/api/protected', {
  headers: {
    'Authorization': `Bearer ${authToken}`,
  },
});
```

## Testing

### Backend Tests

Run SEP-10 tests:

```bash
cd backend
cargo test sep10
```

Tests cover:
- Challenge generation
- Signature verification
- Replay protection
- Session management
- Error cases

### Frontend Tests

Run frontend tests:

```bash
cd frontend
npm test sep10Auth
```

Tests cover:
- Challenge request
- Wallet signing
- Verification
- Full authentication flow
- Error handling

## Wallet Compatibility

### Freighter

Most popular Stellar wallet extension.

```typescript
if (window.freighter) {
  const signedXdr = await window.freighter.signTransaction(xdr, {
    network: networkPassphrase,
    accountToSign: publicKey,
  });
}
```

### Albedo

Web-based wallet with no installation required.

```typescript
if (window.albedo) {
  const result = await window.albedo.tx({
    xdr: challengeXdr,
    network: networkPassphrase,
  });
}
```

### xBull

Mobile and desktop wallet.

```typescript
if (window.xBullSDK) {
  const result = await window.xBullSDK.signTransaction({
    xdr: challengeXdr,
    network: networkPassphrase,
  });
}
```

### Rabet

Browser extension wallet.

```typescript
if (window.rabet) {
  const result = await window.rabet.sign(challengeXdr, networkPassphrase);
}
```

## Migration from Password Auth

The SEP-10 implementation coexists with the existing password-based authentication:

1. **Separate endpoints**: SEP-10 uses `/api/sep10/*` while password auth uses `/api/auth/*`
2. **Separate middleware**: Use `sep10_auth_middleware` or `auth_middleware` as needed
3. **Gradual migration**: Routes can be migrated one at a time
4. **Shared infrastructure**: Both use Redis for session storage

## Troubleshooting

### Challenge Generation Fails

- Check server secret key is valid
- Verify network passphrase matches network
- Ensure Redis is running

### Signature Verification Fails

- Verify wallet signed with correct key
- Check time bounds haven't expired
- Ensure network passphrase matches

### Replay Protection Errors

- Challenge already used
- Redis connection issue
- Challenge expired before signing

### Wallet Not Detected

- Install compatible wallet extension
- Check wallet is unlocked
- Verify wallet supports SEP-10

## Security Considerations

1. **Server Secret Key**: Keep secure, never expose to clients
2. **Time Bounds**: Short validity reduces replay window
3. **HTTPS**: Always use HTTPS in production
4. **Token Storage**: Store tokens securely (httpOnly cookies recommended for production)
5. **Session Expiry**: Implement appropriate session timeouts
6. **Rate Limiting**: Protect challenge endpoint from abuse

## Standards Compliance

This implementation follows:
- [SEP-10 v3.4.0](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md)
- Stellar transaction format
- XDR encoding standards
- Ed25519 signature verification

## Future Enhancements

- [ ] Support for muxed accounts
- [ ] Client domain verification
- [ ] Multi-signature support
- [ ] WebAuthn integration
- [ ] Session refresh mechanism
- [ ] Audit logging
