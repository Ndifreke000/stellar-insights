# Session Management System

## Overview

The session management system provides robust session tracking with device awareness, timeout management, and multi-session support. Each user can have multiple active sessions (across different devices), with the ability to view, revoke individual sessions, or revoke all sessions at once.

## Session Model

Each session is uniquely identified and tracks:

- **Session ID**: Unique identifier for the session
- **User ID**: Associated user
- **Refresh Token JTI**: JWT ID of the refresh token for this session
- **Device User Agent**: Browser/client information
- **IP Address**: Client IP address at session creation
- **Timestamps**:
  - `created_at`: Session creation time
  - `last_activity_at`: Last activity timestamp (updated on token refresh)
  - `expires_at`: Absolute expiration time
  - `revoked_at`: Revocation time (if revoked)
- **Timeouts**:
  - `idle_timeout_seconds`: Inactivity timeout (default: 3600 seconds / 1 hour)
  - `max_lifetime_seconds`: Maximum session lifetime (default: 604800 seconds / 7 days)

## Timeout Behavior

### Idle Timeout
A session expires if no activity occurs for `idle_timeout_seconds`. Activity is defined as:
- Token refresh requests

### Absolute Lifetime
A session expires at `expires_at`, regardless of activity, after `max_lifetime_seconds` from creation.

### Expiry Verification
The `get_active_session()` method checks both conditions:
1. Absolute lifetime: `now > expires_at`
2. Idle timeout: `now > (last_activity_at + idle_timeout_seconds)`

If either condition is true, the session is considered expired.

## API Endpoints

### Login
```
POST /api/auth/login
{
  "username": "user@example.com",
  "password": "password"
}

Response:
{
  "access_token": "...",
  "refresh_token": "...",
  "expires_in": 3600
}
```
Creates a new session with device/IP tracking.

### Refresh Token
```
POST /api/auth/refresh
{
  "refresh_token": "..."
}

Response:
{
  "access_token": "...",
  "expires_in": 3600
}
```
Extends the session by updating `last_activity_at` and issuing a new access token.

### List Sessions
```
GET /api/auth/sessions
Authorization: Bearer <access_token>

Response:
{
  "sessions": [
    {
      "id": "session-123",
      "device_user_agent": "Mozilla/5.0...",
      "ip_address": "192.168.1.1",
      "created_at": "2026-01-15T10:30:00Z",
      "last_activity_at": "2026-01-15T10:45:00Z",
      "expires_at": "2026-01-22T10:30:00Z",
      "is_current": false
    }
  ]
}
```

### Revoke Session
```
DELETE /api/auth/sessions/:session_id
Authorization: Bearer <access_token>

Response: 204 No Content
```

### Revoke All Other Sessions
```
POST /api/auth/sessions/revoke-others
Authorization: Bearer <access_token>

Response:
{
  "message": "Other sessions revoked"
}
```
Revokes all sessions except the current one (useful for "log out from other devices").

## Database Schema

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    refresh_token_jti TEXT NOT NULL UNIQUE,
    device_user_agent TEXT,
    ip_address TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_activity_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    idle_timeout_seconds INTEGER NOT NULL,
    max_lifetime_seconds INTEGER NOT NULL,
    revoked_at TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

## Integration with JWT

Access tokens include:
- `session_id`: The associated session ID
- Standard JWT claims (sub, exp, iat, etc.)

Refresh tokens include:
- `session_id`: The associated session ID
- `sid`: Refresh token JTI (for validation)

When a refresh token is used, the session's `last_activity_at` is updated.

## Security Considerations

1. **Fail-Closed on Expiry**: Expired sessions are not reactivated; clients must re-authenticate.
2. **Device Tracking**: IP and user agent stored for forensic/audit purposes (see audit log).
3. **Idle Timeout**: Configurable per-session to support different use cases (web sessions vs. mobile apps).
4. **Multi-Session Support**: Users can have multiple sessions; revoking one does not affect others.

## Configuration

Session timeouts are configurable when creating a session:

```rust
session_service.create_session(
    user_id,
    refresh_token_jti,
    device_user_agent,
    ip_address,
    Some(3600),    // idle_timeout_seconds
    Some(604800),  // max_lifetime_seconds
).await?;
```

Default values:
- Idle timeout: 3600 seconds (1 hour)
- Max lifetime: 604800 seconds (7 days)

## Testing

Run session tests:
```bash
cargo test session_management_test
```

Tests verify:
- Session creation and retrieval
- Expiry logic (absolute and idle timeouts)
- Revocation of single and multiple sessions
- Session touch (activity update)
- Device/IP tracking
