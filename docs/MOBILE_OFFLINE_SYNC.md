# Mobile: offline watchlist sync with conflict resolution

> Design doc for issue **#2846**. Status: **proposed**, not yet implemented.

## Problem

Users edit watchlists on the go. Edits made offline are queued and later replayed blindly. Nothing reconciles them with the server or with edits from another device, so two devices can end up with different watchlists forever.

## Current state

| File | Behaviour |
| --- | --- |
| [`mobile/src/services/watchlist.ts`](../mobile/src/services/watchlist.ts) | Entries live in **AsyncStorage** (`@watchlist/entries`). The header comment says MMKV, but the code uses AsyncStorage. Failed writes are put in `@watchlist/pending_ops`, with one op per corridor (last op wins). There is no version or timestamp on ops. `alertThreshold` changes are not synced at all. |
| [`mobile/src/services/network.ts`](../mobile/src/services/network.ts) | On reconnect it runs `processOfflineQueue()` and `syncWatchlistToBackend()` in parallel. There is no debounce, so a flapping connection replays many times. |
| [`mobile/src/services/database.ts`](../mobile/src/services/database.ts) | AsyncStorage read cache with a `@db_version` migration hook (v1). It has no change log. |
| Backend | **No `/api/v1/watchlist` route exists** (searching `backend/src` for `watchlist` finds nothing). Today every sync call fails and stays queued for good. |

Failure modes today:
1. Device A removes `USDC:NGN` offline while device B changes its threshold online. When A comes back, its blind `DELETE` wins or loses depending on timing, and B never learns about the removal.
2. Re-adding a corridor that was removed on another device brings back a stale `lastStatus`.
3. `GET /watchlist` is never called, so a fresh install never receives the server copy.

## Design

### Data model: per-record server version + tombstones

A vector clock is more than this needs. The server is the single authority and the data is a small set keyed by `corridorId`, so a **server-assigned version per record** (optimistic concurrency) is enough and easier to reason about.

**Backend table** (new migration):

```sql
CREATE TABLE watchlist_entries (
    account          TEXT NOT NULL,           -- Stellar G-address (SEP-10 subject)
    corridor_id      TEXT NOT NULL,           -- "USDC:NGN"
    alert_threshold  TEXT NOT NULL,
    deleted          INTEGER NOT NULL DEFAULT 0,  -- tombstone
    version          INTEGER NOT NULL,        -- per-account monotonic counter
    updated_at       TEXT NOT NULL,
    updated_by_device TEXT,
    PRIMARY KEY (account, corridor_id)
);
CREATE INDEX idx_watchlist_account_version ON watchlist_entries(account, version);
```

`version` comes from a per-account counter (`watchlist_cursors(account, version)`), so "everything since cursor N" is a single indexed range query. Tombstones are kept for 90 days and then pruned. A client whose cursor is older than the prune horizon gets a full snapshot.

**Mobile change log** (moved to MMKV, which is encrypted and synchronous):

```ts
interface LocalEntry  { corridorId; alertThreshold; deleted: boolean; baseVersion: number | null; lastStatus; addedAt }
interface ChangeLogOp { opId: string /* uuid */; corridorId; kind: 'upsert' | 'delete'; alertThreshold?; baseVersion: number | null; createdAt }
// plus: watchlist:cursor (last server version seen)
```

Every local edit updates `LocalEntry` and appends a `ChangeLogOp`. Ops for the same corridor are **coalesced** (upsert then delete becomes delete, and two upserts become the latest one) but keep the **oldest** `baseVersion`, so a conflict is still detected.

### Sync protocol

One endpoint, authenticated with the SEP-10 bearer token. The account comes from the token, never from the body.

```
POST /api/v1/watchlist/sync
{
  "cursor": 41,
  "device_id": "…",
  "ops": [
    { "op_id": "uuid", "corridor_id": "USDC:NGN", "kind": "delete",  "base_version": 37 },
    { "op_id": "uuid", "corridor_id": "USDC:BRL", "kind": "upsert", "alert_threshold": "down_only", "base_version": null }
  ]
}
→ 200
{
  "results": [
    { "op_id": "…", "status": "applied",  "version": 42 },
    { "op_id": "…", "status": "conflict", "server": { "corridor_id": "USDC:BRL", "alert_threshold": "any_change", "deleted": false, "version": 40 } }
  ],
  "changes": [ /* all records with version > cursor, including tombstones */ ],
  "cursor": 42
}
```

- **Idempotent:** `op_id` is stored for 7 days, and a replayed op returns its original result. A retry after a dropped response is therefore safe.
- **Applied** when `base_version` equals the server's current version for that record (or both are null / missing).
- **Conflict** when they differ. The server applies the policy below in the same transaction and returns the winning record.
- The existing `GET/POST/DELETE /api/v1/watchlist` endpoints become thin wrappers over the same service, for web clients.

### Conflict policy

| Local op | Server state since base | Resolution | User-visible? |
| --- | --- | --- | --- |
| upsert | unchanged | apply | no |
| upsert (threshold) | threshold changed elsewhere | **last-writer-wins by server receive time** (the local op wins, since it arrives later) | no |
| upsert | deleted elsewhere | **keep (re-add wins)**, because an explicit add means intent | toast: "USDC:NGN was removed on another device and has been re-added" |
| delete | threshold changed elsewhere | **delete wins** | no |
| delete | already deleted | no-op, report as applied | no |
| upsert (add) | added elsewhere | merge. Keep the earliest `addedAt` and take the threshold from the later write. | no |

The policy is fixed in the backend, so all clients agree. The only conflict shown to the user is *re-add over delete*, through a non-blocking toast and a "Sync activity" row in Settings that lists the last 20 resolved conflicts with an **Undo** action. Undo queues a new op, so there's no special path for it.

### Background sync on reconnect

- `network.ts` calls `watchlistSync.requestSync()` instead of `syncWatchlistToBackend()`. This is debounced (2 s) and single-flight, so a flapping connection doesn't run many syncs at once.
- Sync is also triggered on app foreground (`AppState` → `active`), after login, and through `react-native-background-fetch` (about every 15 min, OS-scheduled).
- Retry uses exponential backoff with jitter (cap 5 min). A 4xx other than 409/429 drops the op and logs it. A 401 stops sync until re-login.
- A fresh install or a cursor past the prune horizon sends `cursor: 0` and gets the full list. Local-only entries without a `baseVersion` are sent as adds.

### Migration from today's storage

A v2 migration in `database.ts` reads `@watchlist/entries` and `@watchlist/pending_ops` from AsyncStorage and writes `LocalEntry` rows with `baseVersion: null` plus a change log with one upsert per entry. It then deletes the old keys. On the first sync the server merges these as adds.

## Tasks

- [ ] Backend: migration, `WatchlistService` with the sync + conflict policy, `POST /api/v1/watchlist/sync`, CRUD wrappers, op-id dedupe table, tombstone pruning job. Add `watchlist_entries` to the GDPR erasure list in `services/erasure.rs`.
- [ ] Mobile: MMKV-backed store + change log, coalescing, `watchlistSync` (single-flight, debounce, backoff), v2 migration
- [ ] Mobile: conflict toast + "Sync activity" screen with Undo
- [ ] Reconnect, foreground and background-fetch triggers
- [ ] Add the endpoint to `docs/API_DOCUMENTATION.md` / OpenAPI. Fix the header comment in `watchlist.ts`.
- [ ] Tests (below)

## Test plan

**Convergence (the acceptance test):** a Rust integration test against an in-memory SQLite backend, driving two simulated clients that use the same sync algorithm as the TS client (or a TS test against a fake server implementing the same policy):

1. A and B both sync to the same state.
2. Both go offline. A deletes `X` and adds `Y`. B changes the threshold of `X` and adds `Y` with a different threshold.
3. They reconnect in both orders (A first, then B first).
4. **Assert:** after each client syncs twice, both local sets are equal to each other and to the server's, and the outcome matches the policy table.

Also:
- Randomised property test: N clients, random ops and random interleavings of sync, always converging.
- Dropped response, then retry with the same `op_id`, applies once.
- Coalescing: add, remove, add while offline sends one upsert with the original `baseVersion`.
- Migration from v1 AsyncStorage data loses no entries.
- A flapping network (10 reconnect events in 1 s) causes 1 sync request.

## Acceptance criteria

- Offline edits from multiple devices sync and converge as tested above
- CI green (lint, type-check, tests)
- No new TODOs without a linked issue
