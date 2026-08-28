# Two-Factor Authentication (2FA)

## Overview

Two-Factor Authentication (2FA) using Time-based One-Time Passwords (TOTP) adds an additional security layer to user accounts. After successful password authentication, users must provide a valid TOTP code (or backup code) to complete the login process.

## TOTP Algorithm

- **Time Step**: 30 seconds
- **Digit Count**: 6-digit codes
- **Algorithm**: HMAC-SHA1
- **Compatible With**: Google Authenticator, Microsoft Authenticator, Authy, and other standard TOTP apps

## Session State

When a user logs in with 2FA enabled, the session enters a "pending 2FA verification" state:

1. User provides username/password
2. Password verified successfully
3. Session created with `2fa_verified = false`
4. User receives temporary access token (limited scope)
5. User submits TOTP or backup code
6. If valid, session upgraded to `2fa_verified = true`
7. User receives full-scope access token

## Enrollment Flow

### 1. Initiate Enrollment
```
POST /api/auth/2fa/enroll/initiate
Authorization: Bearer <access_token>

Response:
{
  "otpauth_uri": "otpauth://totp/stellar-insights:user@example.com?secret=...",
  "secret": "JBSWY3DPEBLW64TMMQ"
}
```

The `otpauth_uri` is used to generate a QR code for scanning into authenticator apps. The raw `secret` is provided as a backup in case QR scanning fails.

### 2. Confirm Enrollment
```
POST /api/auth/2fa/enroll/confirm
Authorization: Bearer <access_token>
{
  "totp_secret": "JBSWY3DPEBLW64TMMQ",
  "verification_code": "123456"
}

Response:
{
  "backup_codes": [
    "123456",
    "234567",
    ...
  ],
  "message": "2FA enrollment confirmed. Save your backup codes in a secure location."
}
```

The user must verify ownership of the authenticator by providing a valid TOTP code. Only then is 2FA activated and backup codes generated.

## Login Flow with 2FA

### Step 1: Password Authentication
```
POST /api/auth/login
{
  "username": "user@example.com",
  "password": "password"
}

Response (if 2FA enabled):
{
  "access_token": "...",  // limited scope
  "refresh_token": "...",
  "expires_in": 3600,
  "requires_2fa": true
}
```

### Step 2: TOTP Verification
```
POST /api/auth/2fa/verify
Authorization: Bearer <access_token>
{
  "code": "123456"
}

Response:
{
  "message": "2FA verification successful",
  "access_token": "..."  // full-scope token
}
```

### Alternative: Backup Code Verification
```
POST /api/auth/2fa/backup-code
Authorization: Bearer <access_token>
{
  "code": "123456"
}

Response:
{
  "message": "Backup code verified.",
  "access_token": "..."  // full-scope token
}
```

## Backup Codes

**Purpose**: Backup codes allow users to access their accounts if their authenticator device is lost or unavailable.

- **Generated**: At enrollment time (10 codes)
- **Format**: 6-digit numeric codes
- **Storage**: Hashed in database (SHA-256)
- **Usage**: One-time use only
- **Recovery**: Generate new codes via `POST /api/auth/2fa/regenerate-backup`

### Security Properties

- Backup codes are **hashed** before storage (SHA-256), so the database compromise does not leak usable codes
- Each code is **single-use**: after verification, marked as `used_at` timestamp
- Users should store backup codes in a **secure location** (password manager, printed and locked away)

## Rate Limiting

The 2FA verification endpoint implements rate limiting to prevent brute-force attacks on 6-digit codes:

- **Limit**: 5 failed attempts per minute per session
- **Lockout**: 15-minute lockout after 5 consecutive failures
- **Header**: `Retry-After` returned with remaining seconds

## Disabling 2FA

```
POST /api/auth/2fa/disable
Authorization: Bearer <access_token>
{
  "verification_code": "123456"  // or "backup_code": "123456"
}

Response:
{
  "message": "2FA disabled"
}
```

Users must provide a valid TOTP or backup code as confirmation before disabling 2FA (prevents accidental/unauthorized disabling).

## Regenerating Backup Codes

```
POST /api/auth/2fa/regenerate-backup
Authorization: Bearer <access_token>

Response:
{
  "backup_codes": [...],
  "message": "Backup codes regenerated"
}
```

Regenerating backup codes **invalidates all previous codes**. Users should store new codes immediately.

## Database Schema

```sql
CREATE TABLE user_2fa_secrets (
    user_id TEXT PRIMARY KEY,
    encrypted_secret TEXT NOT NULL,    -- TOTP secret (AES-256-GCM encrypted)
    is_enabled BOOLEAN NOT NULL,       -- Whether 2FA is active
    enrolled_at TIMESTAMP,             -- Enrollment timestamp
    backup_codes_generated_at TIMESTAMP,-- Last backup code generation time
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE user_2fa_backup_codes (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    hashed_code TEXT NOT NULL UNIQUE,  -- SHA-256 hash of code
    used_at TIMESTAMP,                 -- Consumed timestamp (NULL = unused)
    created_at TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id) REFERENCES user_2fa_secrets(user_id)
);
```

## Encryption

TOTP secrets are **encrypted at rest** using AES-256-GCM before storage:

- **Key**: `ENCRYPTION_KEY` environment variable
- **Cipher**: AES-256-GCM
- **Nonce**: 96 bits (randomly generated per encryption)
- **Format**: `base64(nonce):base64(ciphertext)`

On retrieval, the encrypted secret is decrypted before use in TOTP validation.

## Audit Logging

All 2FA events are logged to the audit log with `event_type = "2fa_*"`:

- `2fa_enrollment_initiated`: User starts 2FA enrollment
- `2fa_enrollment_confirmed`: User confirms and activates 2FA
- `2fa_verification_success`: Successful TOTP/backup code verification
- `2fa_verification_failed`: Failed verification (tracks brute-force attempts)
- `2fa_disabled`: User disables 2FA
- `2fa_backup_regenerated`: User generates new backup codes

See [AUDIT_LOG.md](./AUDIT_LOG.md) for audit logging details.

## Testing

Run 2FA tests:
```bash
cargo test twofa_test
```

Tests verify:
- TOTP secret generation and QR code URI format
- 2FA enrollment flow
- Backup code generation and one-time use enforcement
- Disabling and regenerating backup codes
- Invalid code rejection

## Security Considerations

1. **Secret Storage**: TOTP secrets are encrypted at rest
2. **Backup Codes**: Single-use, hashed in database
3. **Rate Limiting**: Prevents brute-force attacks on TOTP codes
4. **Session State**: "Pending 2FA" sessions have limited scope until verified
5. **No Timing Leaks**: Code verification uses constant-time comparison
6. **Audit Trail**: All 2FA events logged for security review
