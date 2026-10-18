# Tamper-Proof Audit Log for Admin Actions

## Overview

A tamper-proof, append-only audit log records all admin actions and security events with cryptographic integrity checking via hash chaining.

## Features

- **Append-Only**: No update/delete API exposed; logs can only be added
- **Hash Chaining**: Each entry includes SHA-256 hash of previous entry + current data
- **Integrity Verification**: Walk entire chain to detect tampering
- **Full Context**: Session ID, device user-agent, IP, event type
- **Queryable**: Filter by user, action, date range, status

## Schema

```sql
CREATE TABLE admin_audit_log (
    id TEXT PRIMARY KEY,
    timestamp TIMESTAMP NOT NULL,
    action VARCHAR(100) NOT NULL,
    resource VARCHAR(255) NOT NULL,
    user_id TEXT NOT NULL,
    status VARCHAR(20) NOT NULL,
    details JSONB,
    hash TEXT NOT NULL,
    session_id TEXT,
    device_user_agent TEXT,
    ip_address TEXT,
    event_type VARCHAR(50)
);
```

## Entry Types

### Admin Actions
- `action_name`: Description of what was done
- `resource`: What was modified (user ID, config key, etc.)
- `status`: success, failure, denied
- `details`: JSON with action-specific context

### Security Events
- `event_type = "2fa_enrollment"`: User enrolled in 2FA
- `event_type = "2fa_verification_failed"`: Failed TOTP/backup code
- `event_type = "ip_whitelist_add"`: Admin added IP to whitelist
- `event_type = "ip_whitelist_remove"`: Admin removed IP
- `event_type = "session_revoked"`: User revoked a session

## Hash Chaining

Each entry stores:
```
hash = SHA256(previous_hash | id | timestamp | action | resource | user_id | status | details)
```

First entry: `hash = SHA256(id | timestamp | action | ...)`

## Integrity Verification

```
GET /admin/audit-log/verify-integrity

Response:
{
  "is_valid": true,
  "total_entries": 1245,
  "invalid_entries": [],
  "message": "Audit log integrity verified: 1245 entries in valid hash chain"
}
```

If any entry is modified, its hash no longer matches the recomputed value, and the chain breaks from that point forward.

## API Endpoints

### Query Audit Log
```
GET /admin/audit-log?user_id=user123&action=login&status=success&limit=50&offset=0

Response: Array of audit entries with timestamps, IPs, sessions, events
```

### Verify Integrity
```
POST /admin/audit-log/verify-integrity

Response: Integrity check results (valid, total, invalid count, message)
```

## Logging Events

The audit log is automatically populated when:

1. **Admin performs action**: Logged with action, resource, status
2. **2FA event occurs**: Enrollment, verification success/fail
3. **IP whitelist changes**: Add/remove with admin user ID
4. **Session revoked**: User or timeout
5. **Access denied**: IP whitelist rejection, 2FA failure

All events include:
- Timestamp (UTC)
- Acting user ID
- Session ID (if applicable)
- Client IP (from request)
- Device user-agent (if available)
- Event type (if security event)

## Security Properties

1. **Tamper-Evident**: Modifying any entry invalidates all subsequent hashes
2. **Non-Repudiation**: user_id + timestamp prove who did what when
3. **Immutable**: Only append operations; no delete/update
4. **Auditable**: Full context for forensic analysis
5. **Detectable**: Integrity verification catches tampering

## Retention

Audit entries are retained indefinitely per security best practices. Periodic exports/archival recommended for long-term storage.

## Testing

Verify integrity:
```bash
curl -X POST https://api.example.com/admin/audit-log/verify-integrity \
  -H "Authorization: Bearer <token>"
```

Query for specific events:
```bash
curl "https://api.example.com/admin/audit-log?event_type=2fa_enrollment" \
  -H "Authorization: Bearer <token>"
```
