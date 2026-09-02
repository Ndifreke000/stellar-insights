# Data Classification & Encryption Strategy

## Overview

This document classifies all sensitive data in payraider and defines encryption requirements per storage layer and data category.

## Data Classification Levels

| Level | Definition | Encryption | Examples |
|-------|-----------|-----------|----------|
| **CRITICAL** | Cryptographic keys, authentication credentials | Required (AES-256-GCM) | Encryption keys, JWT secrets, OAuth tokens |
| **HIGH** | Personally Identifiable Information (PII), financial data | Required (AES-256-GCM) | User emails, IP addresses, wallet addresses, API keys |
| **MEDIUM** | Business data that could be used for competitive advantage | Optional (database-level encryption) | Corridor metrics, asset configurations, rate limits |
| **LOW** | Public or non-sensitive operational data | Optional (database-level encryption) | API endpoint names, error logs, health check status |

## Database Data Classification

### Users Table
| Column | Classification | Rationale | Encryption Requirement |
|--------|-----------------|-----------|----------------------|
| `id` | MEDIUM | Used to link PII | Database-level |
| `username` | MEDIUM | Potential PII | Database-level |
| `password_hash` | LOW | Hashes are not reversible; no encryption needed | None |
| `created_at` | LOW | Timestamp metadata | Database-level |
| `updated_at` | LOW | Timestamp metadata | Database-level |

### API Keys Table
| Column | Classification | Rationale | Encryption Requirement |
|--------|-----------------|-----------|----------------------|
| `id` | HIGH | Unique identifier for sensitive resource | AES-256-GCM |
| `name` | MEDIUM | User-facing label | Database-level |
| `key_prefix` | MEDIUM | Used for display in UI (only safe suffix) | Database-level |
| `key_hash` | HIGH | Hash of the actual API key; critical security control | AES-256-GCM |
| `wallet_address` | HIGH | User's Stellar wallet; financial PII | AES-256-GCM |
| `scopes` | MEDIUM | API permissions metadata | Database-level |
| `status` | MEDIUM | Operational state | Database-level |
| `created_at` | LOW | Timestamp | Database-level |
| `last_used_at` | MEDIUM | Activity tracking; mild PII | Database-level |
| `expires_at` | MEDIUM | Policy metadata | Database-level |
| `revoked_at` | MEDIUM | Policy metadata | Database-level |

### User Consents Table (GDPR)
| Column | Classification | Rationale | Encryption Requirement |
|--------|-----------------|-----------|----------------------|
| `user_id` | HIGH | Foreign key to PII | AES-256-GCM |
| `consent_type` | MEDIUM | Type of consent (analytics, marketing, etc.) | Database-level |
| `ip_address` | HIGH | PII; used to track consent location | AES-256-GCM |
| `user_agent` | HIGH | Identifies device/browser; mild PII | AES-256-GCM |
| `granted_at` / `revoked_at` | MEDIUM | GDPR compliance timestamps | Database-level |

### Data Export/Deletion Request Tables (GDPR)
| Column | Classification | Rationale | Encryption Requirement |
|--------|-----------------|-----------|----------------------|
| `user_id` | HIGH | PII linkage | AES-256-GCM |
| `download_token` | CRITICAL | Bearer token for data download; acts like a secret | AES-256-GCM |
| `file_path` | HIGH | May contain PII references | AES-256-GCM |
| `reason` | MEDIUM | Optional user-provided text | Database-level |

### OAuth / Webhooks Table
| Column | Classification | Rationale | Encryption Requirement |
|--------|-----------------|-----------|----------------------|
| `client_secret` | CRITICAL | OAuth secret; credentials | AES-256-GCM |
| `webhook_secret` | CRITICAL | HMAC key for webhook validation | AES-256-GCM |
| `access_token` / `refresh_token` | CRITICAL | OAuth bearer tokens | AES-256-GCM |
| `callback_url` | MEDIUM | Webhook endpoint URL | Database-level |

### Vault Audit Log Table
| Column | Classification | Rationale | Encryption Requirement |
|--------|-----------------|-----------|----------------------|
| `user_id` | HIGH | PII linkage | AES-256-GCM |
| `secret_path` | MEDIUM | Vault path accessed | Database-level |
| `action` | MEDIUM | Action performed (read, write, etc.) | Database-level |
| `ip_address` | HIGH | Access location PII | AES-256-GCM |

## Encryption Strategy by Storage Layer

### 1. Database Encryption at Rest (Baseline)

**Implementation:** The backend is SQLite-only in every environment
(`docs/adr/0001-sqlite-vs-postgres.md`) -- there is no separate dev/production
database technology, only different storage backing the same SQLite file:
- **Local development:** SQLite does not natively support encryption at
  rest. File-level encryption is expected (OS-level or filesystem-level)
- **Production (ECS/k8s):** the SQLite file lives on an EFS volume (ECS) or
  a PVC (k8s). EFS encryption at rest is enabled by default
  (`aws_efs_file_system.backend_data`, `encrypted = true` in
  `terraform/modules/compute/ecs/main.tf`); PVC-level encryption depends on
  the cluster's StorageClass/CSI driver configuration

**Benefit:** Protects against raw disk/backup access without key access.

### 2. Application-Level Field Encryption (Critical & HIGH Data)

**Algorithm:** AES-256-GCM (authenticated encryption)
- Unique IV/nonce per encryption operation
- HMAC authentication included in ciphertext
- No raw ECB/CBC modes

**Key Management:**
- Encryption keys sourced from Vault (via `SecretsService`)
- Keys never embedded in code or committed to version control
- Key rotation: Generate new key in Vault, maintain old key for decryption of existing data during rotation period

**Implementation Approach:**
1. Create an `EncryptionService` in the backend
2. Transparent field-level encryption in database access layers (ORM, query builders)
3. Encrypted data stored as hex-encoded ciphertext in columns

**Fields Requiring Application-Level Encryption:**
- `api_keys.key_hash` — API key hashes
- `api_keys.wallet_address` — User wallet addresses
- `user_consents.user_id`, `user_consents.ip_address`, `user_consents.user_agent` — GDPR PII
- `data_export_requests.user_id`, `data_export_requests.download_token`, `data_export_requests.file_path` — GDPR export data
- `data_deletion_requests.user_id` — GDPR deletion audit
- `vault_audit_log.user_id`, `vault_audit_log.ip_address` — Audit trail PII
- `oauth_webhooks.client_secret`, `oauth_webhooks.webhook_secret` — OAuth credentials
- `oauth_webhooks.access_token`, `oauth_webhooks.refresh_token` — OAuth tokens

### 3. Backups Encryption

**Requirement:** Backups must inherit the same encryption guarantees as primary storage.

**Implementation:** see `docs/backup-system.md` for the full backup story
(Litestream continuous S3 replication + `backup.rs` periodic local
snapshots). The Litestream replica inherits the backups S3 bucket's
SSE encryption (`terraform/global/backups.tf`). For local snapshots taken
outside that pipeline, encrypt with `gpg`:
  ```bash
  sqlite3 payraider.db ".backup /tmp/backup.db"
  gpg --symmetric --cipher-algo AES256 -o backup.db.gpg /tmp/backup.db
  ```
- Store backup encryption keys in Vault, separate from database encryption keys

### 4. Cache Encryption

**Requirement:** Redis/memory caches should NOT store sensitive unencrypted data.

**Implementation:**
- Cache only non-sensitive aggregated data (metrics, counts)
- If caching encrypted data: ensure keys are stored separately
- Consider TTL-based cache invalidation to limit exposure window

### 5. Logs & Error Messages

**Requirement:** Ensure no sensitive data leaks in plaintext via logs.

**Implementation:**
- Sanitize all log output (no API keys, tokens, encrypted payloads)
- Use structured logging with defined safe fields
- Never log plaintext of CRITICAL/HIGH classified data
- Redact function parameters containing secrets

## Encrypted Query Handling

### Current Limitation
- Exact-match queries on encrypted fields cannot use indexes or SQL equality
- Example: `SELECT * FROM api_keys WHERE key_hash = 'abc123'` fails because `key_hash` is encrypted

### Solution: Deterministic Encryption Hash

For fields that require both encryption and equality search:
1. Store two columns per sensitive field:
   - `field_encrypted` — AES-256-GCM ciphertext (non-deterministic)
   - `field_search_hash` — HMAC-SHA256(field, search_key) — deterministic for equality search, cannot reverse

2. Example for API key lookup:
   ```sql
   -- Schema
   ALTER TABLE api_keys ADD COLUMN key_hash_encrypted TEXT;
   ALTER TABLE api_keys ADD COLUMN key_hash_search TEXT;  -- HMAC-SHA256
   
   -- Query
   SELECT * FROM api_keys WHERE key_hash_search = HMAC(user_api_key, search_key)
   ```

3. **Tradeoff:** Search hash reveals which encrypted values are identical, but doesn't reveal the plaintext.

## Migration Plan

### Phase 1: Encryption Infrastructure (This PR)
1. Create `EncryptionService` with AES-256-GCM support
2. Add helper functions for transparent encryption/decryption
3. Tests for encryption round-trips and key rotation

### Phase 2: Schema Updates
1. Add encrypted columns for all CRITICAL/HIGH fields
2. Ensure backward compatibility (old plaintext columns kept during transition)
3. Add migration script for batched encryption of existing data

### Phase 3: Rollback-Safe Data Migration
1. Read plaintext data in batches (resumable if interrupted)
2. Encrypt and write to new encrypted columns
3. Verify data integrity
4. Only then delete old plaintext columns (in separate migration)

### Phase 4: Application Code Updates
1. Update ORM/query layers to use encrypted columns
2. Update API endpoints to decrypt before responding
3. Ensure logs don't leak plaintext

## Testing Strategy

1. **Unit Tests:**
   - Encryption/decryption round-trip correctness
   - Different key versions work correctly
   - Nonce/IV uniqueness per operation

2. **Integration Tests:**
   - Encrypted field persists and retrieves correctly from database
   - Raw database read shows ciphertext (not plaintext)
   - Queries on encrypted fields return correct rows (via search hash)

3. **Security Tests:**
   - Verify no sensitive data in logs
   - Verify no plaintext of CRITICAL fields in error messages
   - Key rotation: old and new keys both work during rotation window

## References

- **AES-256-GCM:** [NIST SP 800-38D](https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf)
- **Key Management:** [NIST Key Management](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-57pt1r5.pdf)
- **Deterministic Encryption for Search:** [Format-Preserving Encryption](https://en.wikipedia.org/wiki/Format-preserving_encryption)

## Compliance

- **GDPR:** Encryption satisfies "pseudonymization" requirement for data security
- **CCPA:** Encryption supports secure deletion of encrypted data
- **PCI DSS:** AES-256-GCM meets requirement 3.4 for encryption at rest
