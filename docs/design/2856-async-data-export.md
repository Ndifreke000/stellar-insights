# Design note: async data export jobs with resumable, signed downloads

Tracking issue: #2856
Status: proposed

## Problem

Exports are built inside the HTTP request. Large exports hold a worker and a DB
connection for the whole request, and they hit the request timeout. The current
safeguards work by refusing large exports: ranges are capped at 90 days and rows at 5,000.
That keeps the server up, but nobody can export a year of payments.

Goal: a user asks for an export and gets a job id straight away. A worker writes the file
to object storage in chunks, in CSV or Parquet. The user is notified when it finishes and
downloads it over a signed, expiring URL that supports HTTP `Range`, so an interrupted
download can resume.

## Current state

Code checked on `main` at `d60acbb2`.

- [`api/export.rs`](../../backend/src/api/export.rs) (825 lines) has `export_corridors`,
  `export_anchors` and `export_payments`, mounted at `/api/v1/export/*` in
  [`api/v1/mod.rs`](../../backend/src/api/v1/mod.rs).
  - Limits: `MAX_EXPORT_RANGE_DAYS = 90`, `MAX_EXPORT_ROWS = 5_000`.
  - Payments CSV streams through `Body::from_stream`, but it is still capped at 5,000 rows
    and runs inside the request.
  - JSON and Excel load the whole result set into memory (`serde_json::to_vec` and
    `rust_xlsxwriter`).
  - `sanitize_csv_field` guards against CSV and formula injection. The new writer must keep it.
- `middleware/expensive_rate_limiter.rs` puts `/api/export/*` in the expensive bucket.
- [`api/gdpr.rs`](../../backend/src/api/gdpr.rs) already has a request-then-download shape
  (`data_export_requests`, status `pending`, `download_url = /api/gdpr/export/{id}/download`).
  However, nothing processes those requests and the download route does not exist. This
  design builds the job runner that GDPR export can reuse.
- Dependencies already present: `hmac`, `sha2`, `lettre`, `redis`, and the `webhooks` module.
  Nothing for object storage or Parquet yet.

## Design

### Overview

```
POST /api/v1/exports ──► validate ──► INSERT export_jobs (queued) ──► 202 {id}
                                              │
              ExportWorker (N per instance) ◄─┘ claim with lease
                     │
   keyset-paginated query (10k rows/batch)
                     │
   CSV / Parquet encoder ──► multipart upload (8 MiB parts) ──► object store
                     │                                                │
     progress row update every batch                                  │
                     ▼                                                │
     status=succeeded ──► notify (in-app + webhook + optional email)  │
                                                                      │
GET /api/v1/exports/{id}/download ──► 302 presigned URL (15 min) ─────┘ Range-capable
```

The handlers only validate input and read or write `export_jobs`. All of the logic lives
in `services/export/` (`job_store.rs`, `worker.rs`, `writers.rs`, `storage.rs`) so it can
be tested without HTTP.

### API

All routes require authentication (JWT or API key). Jobs belong to the caller.

| Method and path | Purpose | Success |
|---|---|---|
| `POST /api/v1/exports` | Create a job | `202` + job |
| `GET /api/v1/exports` | List my jobs (paginated per [API_PAGINATION.md](../API_PAGINATION.md)) | `200` |
| `GET /api/v1/exports/{id}` | Status and progress | `200` |
| `DELETE /api/v1/exports/{id}` | Cancel a queued or running job, or delete the finished file | `204` |
| `GET /api/v1/exports/{id}/download` | Redirect to a freshly signed URL | `302` |

Request:

```json
{
  "dataset": "payments",          // payments | corridors | anchors
  "format": "parquet",            // csv | parquet
  "start_date": "2025-11-01T00:00:00Z",
  "end_date":   "2026-11-01T00:00:00Z",
  "corridor_id": null
}
```

Job response:

```json
{
  "id": "0f6c…",
  "status": "running",            // queued | running | succeeded | failed | cancelled | expired
  "dataset": "payments",
  "format": "parquet",
  "progress": { "rows_written": 1830000, "rows_estimated": 4100000, "percent": 44 },
  "created_at": "…", "started_at": "…", "finished_at": null,
  "file": null,                    // on success: { "size_bytes", "sha256", "content_type" }
  "download_url": null,            // on success: "/api/v1/exports/0f6c…/download"
  "expires_at": null,              // file deletion time (default +7 days)
  "error": null                    // { code, message } in the standard ApiError shape
}
```

Validation at the boundary returns the standard `ApiError` shape from
[`error.rs`](../../backend/src/error.rs):

| Condition | Status | `code` |
|---|---|---|
| Unknown `dataset` or `format` | 400 | `EXPORT_INVALID_PARAMS` |
| `start_date > end_date` | 400 | `EXPORT_INVALID_RANGE` |
| Range above `ASYNC_EXPORT_MAX_RANGE_DAYS` (default 366) | 400 | `EXPORT_RANGE_TOO_LARGE` |
| More than `ASYNC_EXPORT_MAX_ACTIVE_PER_USER` (default 2) queued or running | 429 | `EXPORT_TOO_MANY_ACTIVE` |
| Job not owned by the caller | 404 | `NOT_FOUND` (do not reveal that it exists) |
| Download requested before success | 409 | `EXPORT_NOT_READY` |
| File expired | 410 | `EXPORT_EXPIRED` |

**Existing synchronous endpoints** keep working and keep their limits. When a request is
over the limit, the 400 now carries `details.async_endpoint = "/api/v1/exports"` so clients
can find the new path. Excel stays synchronous only (see Deferred).

### Data model

Migration `052_create_export_jobs.sql`:

```sql
CREATE TABLE IF NOT EXISTS export_jobs (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL,
    dataset         TEXT NOT NULL,
    format          TEXT NOT NULL,
    params          TEXT NOT NULL,           -- validated request JSON
    status          TEXT NOT NULL,           -- queued|running|succeeded|failed|cancelled|expired
    attempts        INTEGER NOT NULL DEFAULT 0,
    lease_owner     TEXT,                    -- worker instance id
    lease_until     TEXT,                    -- lease expiry; reclaimable after this
    rows_written    INTEGER NOT NULL DEFAULT 0,
    rows_estimated  INTEGER,
    object_key      TEXT,
    size_bytes      INTEGER,
    sha256          TEXT,
    error_code      TEXT,
    error_message   TEXT,
    created_at      TEXT NOT NULL,
    started_at      TEXT,
    finished_at     TEXT,
    expires_at      TEXT
);
CREATE INDEX IF NOT EXISTS idx_export_jobs_queue ON export_jobs (status, created_at);
CREATE INDEX IF NOT EXISTS idx_export_jobs_user  ON export_jobs (user_id, created_at DESC);
```

### Worker

- **Claiming.** `UPDATE export_jobs SET status='running', lease_owner=?, lease_until=now+60s,
  attempts=attempts+1 WHERE id = (SELECT id FROM export_jobs WHERE status='queued' OR
  (status='running' AND lease_until < now) ORDER BY created_at LIMIT 1) RETURNING *`.
  This is safe across instances. A heartbeat extends the lease every 20s.
- **Concurrency.** `EXPORT_WORKER_CONCURRENCY` (default 2) per instance. Workers use a
  separate DB pool with 2 connections, so exports cannot exhaust the API pool
  (see [db-pool-metrics.md](../db-pool-metrics.md)).
- **Reading.** Keyset pagination, `WHERE (created_at, id) > (?, ?) ORDER BY created_at, id LIMIT 10000`.
  There is no `OFFSET` and no long-held cursor, so SQLite writers are never blocked for long.
  `rows_estimated` comes from a `COUNT(*)` at the start.
- **Cancellation.** The worker checks `status` between batches. When it sees `cancelled`,
  it aborts the multipart upload.
- **Retries.** A transient failure (storage 5xx, DB busy) releases the lease, and the job
  is picked up again from the beginning. After `attempts >= 3` it becomes `failed`.
  Restarting from the beginning, rather than resuming mid-file, keeps Parquet simple: the
  footer depends on every row group. The cost is acceptable at the sizes we expect.

### Writers and storage

Storage uses the `object_store` crate behind `services/export/storage.rs`. The backends
are S3-compatible (production), `LocalFileSystem` (dev) and `InMemory` (tests). Use the
same config as the snapshot publisher (#2857) so there is one storage setup.

- **CSV.** The `csv` crate, with each field passed through the existing
  `sanitize_csv_field`. Bytes go to a `WriteMultipart` with 8 MiB parts, so memory stays
  bounded regardless of export size.
- **Parquet.** `parquet::arrow::AsyncArrowWriter` over `ParquetObjectWriter`. One row group
  per 10k-row batch, ZSTD compression, and an explicit Arrow schema per dataset. Amounts
  are `Decimal128(38, 7)` to match Stellar's 7 decimal places, not `Float64`.
  Timestamps are `Timestamp(Microsecond, "UTC")`.
- **Object key.** `exports/{user_id}/{job_id}.{csv|parquet}`. This prefix is never public.
- The SHA-256 is computed while streaming and stored, so users can check the download.

New dependencies: `object_store` (with the `aws` feature), `arrow-array`, `arrow-schema`
and `parquet` (with the `async` and `zstd` features). Parquet adds noticeable compile time.
If CI time becomes a problem, put it behind a default-on `export-parquet` cargo feature.

### Signed, expiring, resumable downloads

`GET /api/v1/exports/{id}/download` checks ownership and status, then returns `302` to a
freshly signed URL:

- **S3.** Presigned `GET`, valid for 15 minutes (`EXPORT_URL_TTL_SECONDS`), with
  `response-content-disposition=attachment; filename="payments_2025-11-01_2026-11-01.parquet"`.
  S3 handles `Range` and returns `206` natively.
- **Local backend.** We serve it ourselves at `/api/v1/exports/{id}/file?exp=…&sig=…`.
  The signature is HMAC-SHA256 over `id|exp` with `EXPORT_URL_SIGNING_KEY`, compared in
  constant time. The file is served with `tower_http::services::ServeFile`, which supports
  `Range`, `If-Range` and `206`. `ETag` is set to the stored SHA-256.

Resuming works because the object never changes after success. A client that receives
`403` mid-download (an expired signature) calls `/download` again for a new URL and sends
`Range: bytes=<received>-` with `If-Range: <etag>`. The SDKs do this automatically.

### Notification

When a job succeeds or fails, the worker:

1. writes an in-app notification (the existing notifications page),
2. sends the `export.completed` or `export.failed` webhook event through
   [`webhooks`](../../backend/src/webhooks/mod.rs) if the user has one configured, and
3. sends an email through `lettre` if `EXPORT_EMAIL_NOTIFY=true` and the user has a
   verified email.

Notification failures are logged and counted. They never fail the job.

### Retention

The hourly `export_cleanup` job (in `jobs/scheduler.rs`) deletes objects whose
`expires_at` has passed and marks those jobs `expired`. It also aborts multipart uploads
left behind by crashed workers (they show up as `running` with an expired lease and
`attempts >= 3`).

### Metrics

| Metric | Type | Labels |
|---|---|---|
| `export_jobs_total` | counter | `dataset`, `format`, `status` |
| `export_job_duration_seconds` | histogram | `dataset`, `format` |
| `export_rows_written_total` | counter | `dataset` |
| `export_queue_depth` | gauge | |
| `export_bytes_written_total` | counter | `format` |

## Failure modes

| Failure | Behaviour |
|---|---|
| Worker crash mid-job | The lease expires and another worker reclaims the job and restarts it. The orphaned multipart upload is aborted |
| Object store unavailable | Retried up to 3 attempts with backoff, then `failed` with `EXPORT_STORAGE_UNAVAILABLE` |
| DB busy or locked | Batch retried with jittered backoff. Counts as transient |
| User cancels | Checked between batches. Upload aborted. Status `cancelled` |
| Signed URL expires mid-download | Client gets a fresh URL and resumes with `Range` |
| Download after expiry | `410 EXPORT_EXPIRED` |
| Signing key rotated | Outstanding local-backend URLs fail with 403, and clients get new ones. Supports a two-key overlap per [key-rotation.md](../architecture/key-rotation.md) |
| Range shifts during export because new rows arrive | The snapshot is bounded by `end_date`. Rows written after the job starts are outside the window by construction if `end_date` is in the past. If `end_date` is now, it is clamped to the job start time |

## Test plan

Service-level tests use `object_store::memory::InMemory` and in-memory SQLite. None need HTTP.

1. **Main path, CSV.** 25k payment rows spanning a year. The job succeeds, the object has
   25k data rows, and the stored SHA-256 matches the bytes.
2. **Main path, Parquet.** Read back with `ParquetRecordBatchReader`. Same row count, and
   decimals round-trip exactly.
3. **Year-long export** (the issue's test plan). Seed 2M rows. The HTTP create call
   returns in under 100 ms, and the job completes with worker memory under a fixed bound.
   This is an `#[ignore]` test run in the nightly CI job.
4. **Resume mid-download** (the issue's test plan). With the local backend, request
   `Range: bytes=0-1023` and then `bytes=1024-`. The joined bytes equal the full file, and
   `If-Range` with a stale ETag returns `200` with the full body.
5. **Signature checks.** A tampered `sig`, an expired `exp` and another user's job id all
   return 403 or 404.
6. **Lease reclaim.** Mark a job running with an expired lease. A second worker claims and
   completes it.
7. **Cancellation** between batches aborts the upload and leaves no object behind.
8. **Validation.** Each row of the validation table returns the documented status and code.
9. **CSV injection regression.** A field starting with `=` is prefixed with `'` in the
   async writer, the same as the sync path.
10. **Per-user active limit.** The third concurrent create returns 429.

## Docs to update

- [API_DOCUMENTATION.md](../API_DOCUMENTATION.md) and `openapi.json`: the new
  `/api/v1/exports` routes.
- `sdk/typescript` and `sdk/python`: `createExport`, `waitForExport` and
  `downloadExport(resume=true)`.
- `.env.example`: `EXPORT_STORAGE_URL`, `EXPORT_URL_SIGNING_KEY`, `EXPORT_URL_TTL_SECONDS`,
  `EXPORT_WORKER_CONCURRENCY`, `ASYNC_EXPORT_MAX_RANGE_DAYS`, `EXPORT_EMAIL_NOTIFY`.

## Deferred

- Async Excel. `.xlsx` needs the whole workbook in memory and caps out at about 1M rows,
  so Parquet or CSV is the right format for large exports.
- Moving GDPR export (`data_export_requests`) onto this runner. It is a natural follow-up
  once `export_jobs` exists.
- Resuming mid-file after a worker crash using persisted multipart part ids.
- A Redis-backed queue. The SQLite lease queue is enough at current scale.
