# Stellar Insights

Stellar Insights is an analytics and preflight API for Stellar assets, with a Rust backend, a Next.js frontend, and TypeScript/Python SDKs.

## Getting started

```bash
git clone https://github.com/Ndifreke000/stellar-insights.git
cd stellar-insights
# backend (Rust, SQLite created on first start)
cd backend && cp .env.example .env && cargo run
cargo test
# frontend
cd ../frontend && npm install && npm run dev
# SDKs
cd ../sdk/typescript && npm install && npm test
cd ../python && pip install -e .[dev] && pytest
```

Try the core endpoint: `curl -s "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"` (port per your `.env`).

## Rate limiting

All API routes are protected by a single rate limiting middleware. Every response — including successful `200` responses and rate-limited `429` responses — carries the standard IETF `RateLimit-*` headers so SDKs and clients can implement backoff without parsing error bodies:

| Header | Description |
| --- | --- |
| `RateLimit-Limit` | Maximum number of requests allowed in the current window. |
| `RateLimit-Remaining` | Number of requests remaining in the current window. |
| `RateLimit-Reset` | Seconds until the current window resets. |

When a request is rate limited, the response is a `429 Too Many Requests` with the same `RateLimit-*` headers (with `RateLimit-Remaining: 0`) plus a `Retry-After` header indicating how long the client should wait before retrying.

Example:

```bash
curl -i "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
# HTTP/1.1 200 OK
# RateLimit-Limit: 60
# RateLimit-Remaining: 59
# RateLimit-Reset: 42
```

A bare `CODE` (e.g. `USDC`) is still accepted and resolves to the corridor for
that code; when several issuers exist for the same code, use `CODE:ISSUER` to
select the one you mean. Malformed asset strings are rejected at the boundary
with the standard error response.

## Pro subscription renewal and grace period

Pro access is time-limited. To avoid silent expiry — where an integrator's
production traffic suddenly drops to free-tier limits — the backend sends
renewal reminders and keeps a short grace period after expiry.

- **Reminders.** A scheduler notifies the account owner (email and/or webhook)
  **7 days** and **1 day** before the Pro subscription expires. Each reminder
  is sent at most once per window, so restarts do not duplicate notifications.
- **Grace period.** After the expiry timestamp passes, Pro entitlements remain
  active for a configurable grace window (default **3 days**) so a late renewal
  does not interrupt production. Once the grace period ends, the account falls
  back to free-tier limits.

Both are configured via environment variables:

| Env var | Default | Meaning |
| --- | --- | --- |
| `SUBSCRIPTION_REMINDER_DAYS` | `7,1` | Comma-separated days-before-expiry at which reminders fire |
| `SUBSCRIPTION_GRACE_PERIOD_DAYS` | `3` | Days Pro stays active after expiry before falling back to free tier |

Reminders are delivered through the same notification channel used for other
account events (email when configured, otherwise the account's webhook).

## Wallet sign-in (SEP-10)

The dashboard signs wallets in with [SEP-10](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md).
The **home domain is server-authoritative**: the frontend fetches it from the
backend config endpoint (`GET /api/v1/auth/config`) and passes it to the SEP-10
challenge request. The client no longer derives the home domain from
`window.location.hostname`, so sign-in works on preview deployments and
localhost, and a client cannot pick the domain that is validated. The backend
ignores any client-supplied home domain and always uses its configured value
(`SEP10_HOME_DOMAIN`).

## GDPR erasure

Confirmed account-deletion requests are executed asynchronously once their
scheduled date passes (see `docs/GDPR_DATA_INVENTORY.md` for the full
field → store → retention inventory). Erasure covers the primary database,
Redis keys, notification stores, and per-user data keys: session IP/user-agent
fields are encrypted under a per-user DEK, so destroying that key
crypto-shreds the user's data out of every backup snapshot. Each run writes
per-store steps and an erasure certificate, retrievable at
`GET /api/gdpr/deletion/{id}/erasure` (requires the `gdpr` feature).

## Database

**SQLite only.** This is a compile-time property, not a configuration one:
`sqlx` is built with only the `sqlite` feature, `backend/src/database.rs` uses
`SqlitePool`/`SqliteConnectOptions` directly, and every migration is written in
SQLite-flavoured SQL.

Setting a `postgresql://` `DATABASE_URL` does **not** switch databases — it
fails to parse as `SqliteConnectOptions` and the backend refuses to start.
Adding Postgres support is a code change. Whether to make it is being tracked
in issue #1876.

## Deployed contracts

The backend integrates with **`payraider`** via `SNAPSHOT_CONTRACT_ID`.
[`contracts/.env.testnet`](contracts/.env.testnet) is generated by
`deploy-contracts-testnet.sh` — prefer reading it over copying IDs from here.

| Contract | Env var | Testnet ID (historical) |
|---|---|---|
| PayRaider | `PAYRAIDER_CONTRACT_ID` / `SNAPSHOT_CONTRACT_ID` | `CAPHQZ4BBT43HU5EUSJAOPKWB66HGLTN4AKJUALV3R2RXS4A6IOXWUTL` |

All other contract crates live under `c

## 🧪 Local safety chec

/* … truncated 1537 chars — edit only what you need near the top … */
## Rate limiting

All API routes are protected by a single rate limiting middleware. Every response — including successful `200` responses and rate-limited `429` responses — carries the standard IETF `RateLimit-*` headers so SDKs and clients can implement backoff without parsing error bodies:

| Header | Description |
| --- | --- |
| `RateLimit-Limit` | Maximum number of requests allowed in the current window. |
| `RateLimit-Remaining` | Number of requests remaining in the current window. |
| `RateLimit-Reset` | Seconds until the current window resets. |

When a request is rate limited, the response is a `429 Too Many Requests` with the same `RateLimit-*` headers (with `RateLimit-Remaining: 0`) plus a `Retry-After` header indicating how long the client should wait before retrying.

Example:

```bash
curl -i "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
# HTTP/1.1 200 OK
# RateLimit-Limit: 60
# RateLimit-Remaining: 59
# RateLimit-Reset: 42
```

## Cost-based rate limiting for expensive exports

Export endpoints (`/api/v1/export/*`) use cost-based rate limiting instead of simple request counting. This better reflects the actual database load of different export requests.

Each export is assigned a cost in "units" based on:
- Number of corridors requested (single vs all)
- Date range width (days × cost-per-day)

Clients receive a daily budget based on their tier:
- **Anonymous**: 100 units/day
- **Authenticated** (JWT/API key): 1,000 units/day  
- **Premium**: 10,000 units/day

### Cost examples

| Request | Cost |
|---------|------|
| Single corridor, 30 days | 40 units |
| Single corridor, 365 days | 375 units |
| All corridors, 1 day | ~10,000 units |
| Analytics dashboard | 5 units |

### Response headers

All export responses include cost information:

| Header | Description |
|--------|-------------|
| `X-Cost-Estimate` | Estimated cost units for this request |
| `X-Remaining-Budget` | Budget remaining after this request |
| `X-Total-Budget` | Client's total daily budget |
| `X-Tier` | Client's tier |
| `X-Budget-Refresh-At` | Unix timestamp of budget refresh |

### Budget exhaustion

If a client exceeds their budget, the response is `429 Too Many Requests` with:
- `Retry-After: 86400` (budget refreshes daily)
- Cost information in headers
- JSON error body explaining the limit

### Example

```bash
# Single corridor export (30-day range)
curl -i "http://localhost:8080/api/v1/export/corridors?format=csv&start_date=2024-11-01&end_date=2024-11-30"

# Response includes cost info:
# X-Cost-Estimate: 40
# X-Remaining-Budget: 960
# X-Total-Budget: 1000
```

For full details, see `docs/COST_RATE_LIMITING.md`.
## Rate limiting

All API routes are protected by a single rate limiting middleware. Every response — including successful `200` responses and rate-limited `429` responses — carries the standard IETF `RateLimit-*` headers so SDKs and clients can implement backoff without parsing error bodies:

| Header | Description |
| --- | --- |
| `RateLimit-Limit` | Maximum number of requests allowed in the current window. |
| `RateLimit-Remaining` | Number of requests remaining in the current window. |
| `RateLimit-Reset` | Seconds until the current window resets. |

When a request is rate limited, the response is a `429 Too Many Requests` with the same `RateLimit-*` headers (with `RateLimit-Remaining: 0`) plus a `Retry-After` header indicating how long the client should wait before retrying.

Example:

```bash
curl -i "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
# HTTP/1.1 200 OK
# RateLimit-Limit: 60
# RateLimit-Remaining: 59
# RateLimit-Reset: 42
```

## Cost-based rate limiting for expensive exports

Export endpoints (`/api/v1/export/*`) use cost-based rate limiting instead of simple request counting. This better reflects the actual database load of different export requests.

Each export is assigned a cost in "units" based on:
- Number of corridors requested (single vs all)
- Date range width (days × cost-per-day)

Clients receive a daily budget based on their tier:
- **Anonymous**: 100 units/day
- **Authenticated** (JWT/API key): 1,000 units/day  
- **Premium**: 10,000 units/day

### Cost examples

| Request | Cost |
|---------|------|
| Single corridor, 30 days | 40 units |
| Single corridor, 365 days | 375 units |
| All corridors, 1 day | ~10,000 units |
| Analytics dashboard | 5 units |

### Response headers

All export responses include cost information:

| Header | Description |
|--------|-------------|
| `X-Cost-Estimate` | Estimated cost units for this request |
| `X-Remaining-Budget` | Budget remaining after this request |
| `X-Total-Budget` | Client's total daily budget |
| `X-Tier` | Client's tier |
| `X-Budget-Refresh-At` | Unix timestamp of budget refresh |

### Budget exhaustion

If a client exceeds their budget, the response is `429 Too Many Requests` with:
- `Retry-After: 86400` (budget refreshes daily)
- Cost information in headers
- JSON error body explaining the limit

### Example

```bash
# Single corridor export (30-day range)
curl -i "http://localhost:8080/api/v1/export/corridors?format=csv&start_date=2024-11-01&end_date=2024-11-30"

# Response includes cost info:
# X-Cost-Estimate: 40
# X-Remaining-Budget: 960
# X-Total-Budget: 1000
```

For full details, see `docs/COST_RATE_LIMITING.md`.

## Database retention

The backend implements non-blocking retention for event data to prevent SQLite lock contention. Instead of a single large `DELETE` query that can lock the database for seconds, retention runs in bounded batches with configurable delays.

### Features

- **Batch deletion**: Deletes rows in configurable batches (default 1,000 per batch)
- **Sleep between batches**: 100ms default to allow concurrent reads
- **Incremental VACUUM**: Reclaims space gradually without extended write locks
- **Analytics preservation**: Automatically rolls up events to corridor metrics before deletion
- **Configurable retention**: Per-table retention with environment variables

### Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `JOB_RETENTION_ENABLED` | `true` | Enable retention job |
| `JOB_RETENTION_INTERVAL_SECONDS` | `86400` | Run interval (24h) |
| `RETENTION_DEFAULT_DAYS` | `30` | Default retention days |
| `RETENTION_BATCH_SIZE` | `1000` | Rows per deletion batch |
| `RETENTION_SLEEP_MS` | `100` | Sleep between batches |
| `VACUUM_ENABLED` | `true` | Enable incremental VACUUM |
| `VACUUM_PAGES_PER_BATCH` | `100` | Pages to vacuum per batch |
| `VACUUM_MAX_PAGES` | `10000` | Maximum pages to vacuum |

### Example

```bash
# Retention runs automatically every 24 hours
# Logs show batch progress:
# "Deleted 1000 rows from contract_events (batch 42)"
# "Completed retention: 50000 rows deleted in 50 batches, 5234ms"
```

For full details, see `docs/RETENTION.md`.
## Database retention

The backend implements non-blocking retention for event data to prevent SQLite lock contention. Instead of a single large `DELETE` query that can lock the database for seconds, retention runs in bounded batches with configurable delays.

### Features

- **Batch deletion**: Deletes rows in configurable batches (default 1,000 per batch)
- **Sleep between batches**: 100ms default to allow concurrent reads
- **Incremental VACUUM**: Reclaims space gradually without extended write locks
- **Analytics preservation**: Automatically rolls up events to corridor metrics before deletion
- **Configurable retention**: Per-table retention with environment variables

### Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `JOB_RETENTION_ENABLED` | `true` | Enable retention job |
| `JOB_RETENTION_INTERVAL_SECONDS` | `86400` | Run interval (24h) |
| `RETENTION_DEFAULT_DAYS` | `30` | Default retention days |
| `RETENTION_BATCH_SIZE` | `1000` | Rows per deletion batch |
| `RETENTION_SLEEP_MS` | `100` | Sleep between batches |
| `VACUUM_ENABLED` | `true` | Enable incremental VACUUM |
| `VACUUM_PAGES_PER_BATCH` | `100` | Pages to vacuum per batch |
| `VACUUM_MAX_PAGES` | `10000` | Maximum pages to vacuum |

### Example

```bash
# Retention runs automatically every 24 hours
# Logs show batch progress:
# "Deleted 1000 rows from contract_events (batch 42)"
# "Completed retention: 50000 rows deleted in 50 batches, 5234ms"
```

For full details, see `docs/RETENTION.md`.

## Database migrations

The backend uses SQLite migrations to manage schema changes. We follow the **expand/contract pattern** to support rolling deployments without downtime.

### Expand/Contract Pattern

- **Expand phase**: Add new columns/tables (safe during rolling deployments)
- **Contract phase**: Remove old columns/tables (requires all pods upgraded)

### Migration Safety

All destructive migrations (DROP, RENAME) must include a `contract-phase:` marker:

```sql
-- contract-phase: after all pods are upgraded to v1.2.0
-- This migration removes legacy_trust_score after all pods use trust_score
ALTER TABLE anchors DROP COLUMN legacy_trust_score;
```

### Linting

Run the migration linter before committing:

```bash
cd backend
./scripts/lint-migrations.sh
```

CI automatically checks migrations for safety issues.

For full details, see `docs/MIGRATION_SAFETY.md` and `CONTRIBUTING.md`.
