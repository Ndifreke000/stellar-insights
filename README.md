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

## Distributed Locking and Fencing

### Lock Acquisition

Distributed locks use Redis `SET key value NX PX ttl` to acquire a lock, with
a Lua script for atomic release/extension. In single-instance mode (no
`REDIS_URL`), locks always succeed for local development.

### Fencing Tokens

When a lock is acquired in distributed mode, a **fencing token** is assigned:

1. The token is a monotonically increasing integer stored in Redis under
   `fence:<lock-key>`
2. Each new lock acquisition increments the token
3. The token is returned in the [`LockGuard`](crate::distributed_lock::LockGuard)

### Why Fencing is Required

Without fencing, a paused leader can continue writing after its lease expires
and another leader is elected. This classic Redlock pitfall leads to:

- **Duplicate snapshot submissions** (same data submitted twice)
- **Duplicate digests** (same calculation run multiple times)
- **Data inconsistencies** from interleaved writes

### Usage Pattern

```ignore
// Acquire lock and get fencing token
let guard = lock.try_acquire("role:myjob", Duration::from_secs(30)).await?;
let token = guard.fence_token().expect("distributed mode");

// Present token with side-effecting operations
db.write_with_fencing(data, token).await?;

// On extension failure, stop immediately
if !guard.extend(ttl).await {
    return; // Stop writing - we lost leadership
}
```

### Storage Layer Enforcement

The storage layer should reject operations with stale tokens:

```ignore
async fn write_with_fencing(&self, data: Data, token: i64) -> Result<()> {
    let current = self.get_fence_token("role:myjob").await?;
    if token < current {
        return Err(StaleLeaderError { token, current }.into());
    }
    // Proceed with write...
}
```

## Job Scheduling

### Persistent Job Runs

Jobs now run with persistence through the `job_runs` and `job_run_configs` tables:

| Table | Description |
| --- | --- |
| `job_runs` | Tracks each scheduled execution with status, attempt, lease, and timing |
| `job_run_configs` | Stores scheduling configuration (interval, catch-up policy, enabled) |

### Exactly-Once Semantics

Jobs are claimed using a lease mechanism:
1. **Claim**: Workers attempt to claim a pending run with a 30-minute lease
2. **Extend**: The lease is extended every `ttl/3` while the job runs
3. **Retry**: Expired leases can be claimed by another instance
4. **Complete**: On success/failure, the run is marked and lease is cleared

This ensures:
- **Exactly-once execution** under normal conditions
- **Automatic failover** when a leader dies mid-job
- **Idempotent retries** with attempt counting

### Catch-Up Policy

When an instance misses scheduled runs (e.g., during downtime):

| Policy | Behavior |
| --- | --- |
| `all` | Schedules all missed runs |
| `latest_only` | Schedules only the most recent missed run (default) |

Configured per-job via `job_run_configs.catch_up_policy`.

### Configuration

| Env var | Default | Meaning |
| --- | --- | --- |
| `JOB_*_ENABLED` | `true` | Enable/disable specific jobs |
| `JOB_*_INTERVAL_SECONDS` | per-job | Interval between runs |

Jobs automatically record runs in `job_runs` table.

### Admin API

| Endpoint | Description |
| --- | --- |
| `GET /admin/jobs/runs` | List job runs (with pagination) |
| `GET /admin/jobs/runs/{id}` | Get specific run details |
| `GET /admin/jobs/config` | List all job configurations |
| `GET /admin/jobs/config/{job_name}` | Get job configuration |
| `PUT /admin/jobs/config/{job_name}` | Update job configuration |
| `POST /admin/jobs/schedule` | Schedule a manual run |
| `GET /admin/jobs/runs/counts` | Get run counts by status |

## Backups and Restore Verification

### Consistent Online Backups

Backups are created using SQLite's online backup API (`BACKUP TO` command) which
produces a consistent snapshot even while writes are happening on the source
database. This avoids the corruption risk of simple file copies taken during
concurrent writes.

- **Backup schedule:** Daily at `BACKUP_SCHEDULE_HOUR_UTC` (default: 2 AM UTC)
- **Restore drill:** Nightly at 4 AM UTC to verify backups are restorable

### Configuration

| Env var | Default | Meaning |
| --- | --- | --- |
| `BACKUP_ENABLED` | `false` | Enable backup system |
| `BACKUP_DB_PATH` | (derived from `DATABASE_URL`) | Path to source database |
| `BACKUP_DIR` | `./backups` | Directory to store backups |
| `BACKUP_RETENTION_DAYS` | `30` | Days to retain backups |
| `BACKUP_SCHEDULE_HOUR_UTC` | `2` | Hour (UTC) to run backup |
| `BACKUP_ENCRYPTION_KEY` | (none) | 64-char hex AES-256 key for encrypted backups |

### What Backups Include

1. **Consistent snapshot** via SQLite `BACKUP TO` command
2. **SHA-256 checksum** in `.sha256` sidecar file
3. **Optional encryption** with AES-256-GCM when `BACKUP_ENCRYPTION_KEY` is set

### Verification Process

Every backup is verified automatically:
1. Compute SHA-256 and compare to sidecar
2. Open as read-only SQLite and run `PRAGMA integrity_check`
3. For encrypted backups, decrypt to temp file first

### Restore Drill

The nightly restore drill job:
1. Finds the latest backup
2. Restores to a scratch database in the temp directory
3. Runs `PRAGMA integrity_check` on the restored DB
4. Reports results and alerts on failure

### Monitoring

Prometheus metrics:
- `backup_verifications_total{result="success\|integrity_check_failed\|checksum_mismatch\|stored_checksum_mismatch"}`
- `backup_last_success_timestamp_seconds`
- `backup_size_bytes`

### Recovery Steps

If a backup fails verification or the restore drill detects corruption:

1. **Check logs** for the specific failure reason
2. **Alerts** are sent via the configured webhook system
3. **Retry** with the previous backup (rotation happens daily)
4. **Investigate** the source of corruption before accepting a failed backup

### Alerting

- **Backup failure:** Logged with `ALERT: Backup failed` and sent to webhook
- **Restore drill failure:** Logged with `ALERT: Restore drill failed` and sent to webhook

Setting a `postgresql://` `DATABASE_URL` does **not** switch databases — it
fails to parse as `SqliteConnectOptions` and the backend refuses to start.
Adding Postgres support is a code change. Whether to make it is being tracked
in issue #1876.

## API versioning & deprecation

All `/api/*` routes pass through a single consolidated versioning middleware (`backend/src/middleware/api_versioning.rs`) backed by a declarative route metadata table (`ApiVersionPolicy`).

- **Version resolution:** Explicit URL prefix (`/api/v1/...`, `/api/v2/...`) takes precedence over `Accept: application/vnd.stellar-insights.v<N>+json` negotiation. Unsupported versions return `400 UNSUPPORTED_API_VERSION` and past-sunset endpoints return `410 ENDPOINT_SUNSET` in the standard error shape.
- **Standard headers:** Current routes emit `API-Version`, `X-API-Version`, and `Vary: Accept`. Deprecated routes additionally emit RFC 9745 `Deprecation: @<unix-timestamp>`, RFC 8594 `Sunset: <HTTP-date>`, and `Link: <...>; rel="deprecation"`.
- Full policy and migration guide: [`docs/API_VERSIONING.md`](docs/API_VERSIONING.md).

## Batch endpoints & field selection security

`POST /api/v1/batch` and `POST /api/preflight/batch` re-enforce every outer security control per sub-request:

- **No middleware bypass:** Each sub-request is dispatched through the full Axum router with the verified outer caller identity (`Authorization` / `ConnectInfo<SocketAddr>`), stripping spoofable headers (`X-Forwarded-For`, `X-Real-IP`, `X-User-Id`, etc.), blocking nested `/batch` recursion and path traversal, and enforcing admin IP whitelists and RBAC (`AdminUser` → `403` for non-admin callers).
- **Per-item rate limiting & cost caps:** A batch of `N` sub-requests consumes `N` rate-limit units across both the caller bucket and API key bucket, and rejects oversized or overly costly batches at the boundary (`BATCH_SIZE_EXCEEDED`, `BATCH_COST_EXCEEDED`).
- **Role-aware field selection:** `?fields=...` validates field syntax and enforces `FieldAccessPolicy` so forbidden secrets and admin-only fields cannot be extracted by non-admin callers (`403 FORBIDDEN_FIELD_SELECTION`).
- Full security model and audit notes: [`docs/BATCH_ENDPOINTS_SECURITY.md`](docs/BATCH_ENDPOINTS_SECURITY.md).

## Shared frontend realtime client

The frontend maintains **one WebSocket connection per browser tab** via the `RealtimeClient` singleton and `RealtimeProvider` context (`frontend/src/lib/websocket.ts`, `frontend/src/contexts/RealtimeContext.tsx`).

- All realtime hooks (`useWebSocket`, `useRealtimeCorridors`, `useRealtimeAnchors`, `useRealtimeCollaboration`, `NotificationContext`) share that single socket using ref-counted channel subscriptions that automatically unsubscribe on unmount.
- Reconnection uses jittered exponential backoff and pauses when the tab is hidden (`visibilitychange`) or offline, while incoming messages patch the React Query cache in place.
- Architecture details: [`docs/REALTIME_CLIENT.md`](docs/REALTIME_CLIENT.md).

## Virtualised streaming tables

Corridor (`/corridors` table view) and live transaction (`/transactions`) tables use `@tanstack/react-virtual` (`VirtualizedCorridorTable`, `VirtualizedTransactionTable`) with:

- Stable row keys, `React.memo` rows/cells, and normalized row patching (`usePatchableRows`) so live WebSocket updates re-render **only** the affected row instead of the full table.
- Server-driven sorting, filtering, and cursor pagination across `/api/corridors` and `/api/rpc/payments`.
- Profiling benchmarks and design notes: [`docs/VIRTUALIZED_TABLES.md`](docs/VIRTUALIZED_TABLES.md).

## Deployed contracts

The backend integrates with **`payraider`** via `SNAPSHOT_CONTRACT_ID`.
[`contracts/.env.testnet`](contracts/.env.testnet) is generated by
`deploy-contracts-testnet.sh` — prefer reading it over copying IDs from here.

| Contract | Env var | Testnet ID (historical) |
|---|---|---|
| PayRaider | `PAYRAIDER_CONTRACT_ID` / `SNAPSHOT_CONTRACT_ID` | `CAPHQZ4BBT43HU5EUSJAOPKWB66HGLTN4AKJUALV3R2RXS4A6IOXWUTL` |

All other contract crates live under `contracts/` and are versioned
independently.

## 🧪 Local safety checks

Run `./scripts/safety-checks.sh` to verify:

- No secrets in source (via `.secrets.baseline`)
- No `unwrap`/`expect` in production code
- No debug prints (`eprintln!`, `println!`)
- No hardcoded credentials

## License

MIT — see [`LICENSE`](LICENSE) for details.
