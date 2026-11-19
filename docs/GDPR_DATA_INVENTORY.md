# GDPR Data Inventory & Erasure Design

This document is the published data inventory required for GDPR erasure
compliance, plus the design note for the erasure pipeline
(`backend/src/services/erasure.rs`, `backend/src/services/user_keys.rs`).

## Data model

### Erasure job model

A confirmed `data_deletion_requests` row (30-day/7-day cancellation window,
unchanged) is picked up by `ErasureService::run_due()` once
`scheduled_deletion_at` passes. Each attempt creates:

- `erasure_jobs` — one row per attempt (`running` → `completed`/`failed`),
  with `certificate_json` on success.
- `erasure_steps` — one row per store (`sqlite`, `notifications`, `redis`,
  `keys`) recording status, rows affected, detail, and error.

Failed jobs leave the request `scheduled` so the next scheduler tick
retries; every store operation is idempotent (delete/anonymize).

### Per-user data keys (crypto-shredding)

`user_data_keys` stores a random 32-byte DEK per user, wrapped by the
deployment KEK (`ENCRYPTION_KEY` via `CryptoService`). PII columns that must
remain meaningful in backups are encrypted under the DEK with a `udk1:`
prefix:

- `sessions.ip_address`
- `sessions.device_user_agent`

Destroying the DEK row on erasure renders every ciphertext copy
unrecoverable — including inside backup snapshots — without rewriting
archive files. This is the crypto-shredding mechanism for backups.

### API

- `GET /api/gdpr/deletion/{id}/erasure` — latest job for a request the
  caller owns: per-store steps plus the erasure certificate once finished.
  (The whole `/api/gdpr` surface, including this endpoint, is mounted only
  when the backend is built with `--features gdpr`.)

## Data inventory (field → store → retention → erasure action)

| Store | Table / key | Personal fields | Retention | Erasure action |
|---|---|---|---|---|
| SQLite | `users` | `username`, `password_hash` | account lifetime | anonymized (`erased_<uuid>`, hash nulled) |
| SQLite | `sessions` | `ip_address`, `device_user_agent` (DEK-encrypted), `refresh_token_jti` | ≤7 days / revoked | deleted + DEK shredded |
| SQLite | `user_2fa_secrets`, `user_2fa_backup_codes` | TOTP secrets, backup codes | account lifetime | deleted |
| SQLite | `user_consents` | consent records | account lifetime | deleted |
| SQLite | `data_export_requests` | export metadata, `download_token` | until expiry | deleted |
| SQLite | `data_deletion_requests` | request record | legal record | kept, marked `completed` |
| SQLite | `data_processing_log` | processing audit | account lifetime | deleted |
| SQLite | `alert_rules`, `alert_history` | alert prefs/history | account lifetime | deleted |
| SQLite | `corridor_alert_configs`, `corridor_alert_events` | alert prefs/events | account lifetime | deleted |
| SQLite | `api_usage_stats` | usage stats per user | rolling | deleted |
| SQLite | `oauth_tokens`, `oauth_authorizations`, `oauth_clients` | OAuth grants/tokens | token lifetime | deleted |
| SQLite | `webhooks` | callback `url`, `secret` | account lifetime | deleted |
| SQLite | `token_revocations` | revoked JTIs | until expiry | deleted |
| SQLite | `snapshot_verifications`, `user_rewards` | user-linked verification/reward rows | account lifetime | deleted |
| SQLite | `device_tokens` | push `device_token` (keyed by `account`) | until unregistered | deleted |
| SQLite | `admin_audit_log`, `vault_audit_log`, `audit_log` | actor `user_id` | audit retention | **anonymized** (`user_id='erased'`) |
| SQLite | `user_data_keys` | wrapped DEK | until erasure | destroyed (crypto-shred) |
| SQLite | `api_keys` | `wallet_address`, `key_hash`, `name` | key lifetime | **not covered** — keyed by `wallet_address`, not `user_id`; see Known gaps |
| Redis | `*{user_id}*` | any cached/session key embedding the user id | TTL-bound | SCAN + UNLINK |
| Email/notifications | runtime-discovered `*email*`/`*notification*` tables | recipient addresses | varies | deleted if a user-keyed table exists; none exist today |
| Backups (`BACKUP_DIR/*.db`) | full SQLite snapshots | everything above | `BACKUP_RETENTION_DAYS` (30d) | crypto-shredded via DEK destruction |
| Structured logs | stdout + `LOG_DIR` files | accidental PII in fields | rotation (30 files) | `redact_pii_line` at emission |

## Erasure certificate

On completion each job stores a certificate (`certificate_json`, also
returned by the API) containing: request id, user id, `subject_hash`
(SHA-256 of user id for post-erasure correlation), timestamps, per-store
steps with rows affected, `crypto_shredded` flag, and retention notes
documentoring what was anonymized rather than deleted.

## Failure modes

- **Store failure** → step marked `failed`, job `failed`, request stays
  `scheduled` and is retried next tick. All operations idempotent.
- **Redis unavailable** → `redis` step fails; sqlite + key shred still run.
  Request retried; keys step is safe to re-run.
- **Missing table/column** (schema drift) → recorded in step `detail`,
  not fatal.
- **No `ENCRYPTION_KEY`** → `UserKeyService` runs without field
  encryption (passthrough); erasure still deletes rows but
  `crypto_shredded` reports false.

## Known gaps / deferred

- `api_keys` rows are keyed by `wallet_address`, which is not linked to
  `user_id` in the schema; erasure cannot reach them without a
  user→wallet mapping. Tracked as a schema limitation.
- `services/gdpr.rs::execute_deletion` (feature `gdpr`, off by default)
  contains an older, narrower deletion path and is not wired to the
  scheduler; the erasure engine in `services/erasure.rs` is authoritative.
- Log redaction is pattern-based; field-level `Redacted` usage remains
  the preferred mechanism for new log sites.
