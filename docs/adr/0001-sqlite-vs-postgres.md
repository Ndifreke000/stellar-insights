# ADR 0001 — SQLite vs PostgreSQL for production write concurrency

- **Status:** Accepted
- **Decision:** Stay on SQLite. Do not add PostgreSQL support at this time.
- **Issue:** #1876
- **Supersedes:** the implicit assumption, visible in `.env.example`, `README.md`,
  `PROJECT_DESCRIPTION.md` and `CODE_DOCUMENTATION.md`, that Postgres was the
  production database. It never was — see #1877.

---

## Context

The backend is hard-wired to SQLite, and not by configuration:

| Evidence | Detail |
|---|---|
| `backend/Cargo.toml` | `sqlx` enables only the `sqlite` feature |
| `backend/src/database.rs` | `SqlitePool`, `SqliteConnectOptions`, `SqliteJournalMode` used directly |
| `backend/migrations/` | 37 migrations, **29 of which** use SQLite-specific SQL |

A `postgresql://` `DATABASE_URL` does not select Postgres. It fails to parse as
`SqliteConnectOptions`, and the backend refuses to start with
`Invalid DATABASE_URL`. Switching is a code change, not a config change.

The concern raised in #1876 is real: SQLite permits exactly one writer at a
time. Readers don't block under WAL, but writes serialise globally, which is a
fundamentally different model from Postgres MVCC.

## What actually limits us today

The single-writer lock is the *theoretical* limit. Before it is reached, two
concrete properties of the current configuration matter more:

### 1. No `busy_timeout` is set

`create_pool` configures WAL, pool sizes, timeouts and statement logging — but
never `busy_timeout`. SQLite's default is **0**: a connection that finds the
write lock held returns `SQLITE_BUSY` **immediately** rather than waiting.

So under concurrent writes the failure mode is not "slow", it is "errors" — and
they surface as spurious 500s under exactly the load where the system should be
degrading gracefully.

### 2. The pool is sized as though writes were parallel

`.env.example` ships `DB_POOL_MAX_CONNECTIONS=100`. Against a database with one
write lock, 100 connections do not buy write throughput; they buy 100 ways to
contend for the same lock. For reads the pool is useful and 100 is fine.

**Neither of these is an argument for Postgres.** They are an argument that
SQLite here has not yet been configured like a production SQLite deployment.
Migrating to Postgres to fix an unset `busy_timeout` would be replacing a
one-line change with a multi-week one.

## Decision

**Stay on SQLite**, for this workload, with the hardening below.

### Why SQLite fits this workload

The write path is not user-driven. Writes come from RPC ingestion, corridor
aggregation, and backfill jobs — background pipelines whose throughput is
already bounded upstream by `RPC_RATE_LIMIT_REQUESTS_PER_MINUTE=90` and
`RPC_MAX_TOTAL_RECORDS`. The API surface is overwhelmingly read-heavy
(corridors, anchors, network stats, analytics, exports), and reads under WAL do
not block on the writer.

An analytics platform ingesting at ~90 requests/minute is nowhere near SQLite's
write ceiling. The operational simplicity — no separate server, no connection
tuning across a network boundary, trivial backup as a file copy — is worth
keeping until there is measured evidence of write pressure.

### Why not Postgres now

- 29 of 37 migrations use SQLite-flavoured SQL (`INSERT OR IGNORE`,
  `AUTOINCREMENT`, `CURRENT_TIMESTAMP` defaults, `datetime()`). None of these
  translate 1:1, so this is a migration rewrite, not a feature flag.
- `sqlx` compile-time query verification is bound to the database backend;
  supporting both means every `query!` invocation must typecheck against both,
  or the macros must be abandoned for runtime queries.
- It buys nothing measurable until the write path is demonstrably contended,
  and we do not currently measure that (see below).

## Consequences

### Required hardening (SQLite as a production database)

- [x] **Set `busy_timeout`.** Included in this change: 5 s by default,
      overridable via `DB_BUSY_TIMEOUT_MS`. Turns lock contention from an
      immediate error into a bounded wait.
- [ ] **Right-size the write pool.** 100 connections is a read-side number.
      Worth splitting reader and writer pools if contention appears.
- [ ] **Durability story.** SQLite's backup is a file copy, but a *consistent*
      one needs care under WAL. Litestream (or an equivalent) gives continuous
      replication; `backup.rs` currently schedules snapshots without it.

### Revisit this decision when

Any of these is a trigger to reopen, and they are deliberately measurable rather
than a matter of taste:

1. `SQLITE_BUSY` / lock-timeout errors appear in production logs at all — with
   `busy_timeout` set, seeing them means waits are exceeding 5 s.
2. Sustained write volume exceeds roughly 50 writes/second, or ingestion rate
   limits are raised by an order of magnitude.
3. A second writer process is introduced (a separate indexer or worker binary
   holding its own connection to the same file).
4. Horizontal scaling of the backend becomes a requirement — multiple backend
   instances cannot share a SQLite file across hosts, and this is the trigger
   most likely to actually force the change.

### Documentation

`.env.example`, `README.md`, `PROJECT_DESCRIPTION.md` and
`CODE_DOCUMENTATION.md` were corrected in #1877 to describe SQLite accurately.
This ADR is the single source of truth for *why*.
