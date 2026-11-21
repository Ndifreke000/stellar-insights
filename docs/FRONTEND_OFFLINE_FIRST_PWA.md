# Frontend: Offline-First PWA with Background Sync for Alerts and Bookmarks

**Issue:** [#2837](https://github.com/Ndifreke000/stellar-insights/issues/2837)  
**Difficulty:** Hard  
**Estimated size:** L — roughly 1–2 weeks  

---

## Summary

The frontend ships PWA scaffolding and an `useOfflineStatus` hook, but offline behaviour stops at detecting connectivity. Any mutation (bookmark creation, alert edit) made while offline is silently lost. Users on unreliable connections — a core demographic for the remittance corridors this product targets — cannot rely on the app. This issue implements a full offline-first loop: local mutations are queued in IndexedDB and synced to the server via the Background Sync API when connectivity returns.

---

## Problem

| File | Lines | Issue |
|---|---|---|
| `frontend/src/lib/pwa.ts` | 41 | Thin PWA utilities; no mutation queuing |
| `frontend/src/hooks/useProgressiveWebApp.ts` | 304 | Detects install/update states; no offline write path |
| `frontend/src/hooks/useOfflineStatus.ts` | 55 | Exposes `isOnline`; mutations are discarded when `false` |
| `frontend/src/hooks/useBookmarks.ts` | 84 | Writes directly to the API; no queue |

---

## Proposed Implementation

### 1. Workbox caching strategies per route

Configure `next-pwa` (or a custom Workbox config) with per-route caching strategies:

| Route pattern | Strategy | Rationale |
|---|---|---|
| Static assets (`/_next/static/**`) | Cache-first | Immutable hashes |
| API reads (`/api/v1/**` GET) | Stale-while-revalidate | Show cached data, refresh in background |
| App shell (`/`) | Network-first with cache fallback | Always fresh when online |
| Images (`/public/**`) | Cache-first with expiry | Low churn |

```ts
// public/sw.ts (Workbox injected)
import { registerRoute } from 'workbox-routing';
import { StaleWhileRevalidate, CacheFirst, NetworkFirst } from 'workbox-strategies';

registerRoute(
  ({ url }) => url.pathname.startsWith('/api/v1') && request.method === 'GET',
  new StaleWhileRevalidate({ cacheName: 'api-reads' })
);
```

### 2. IndexedDB mutation queue

Create `frontend/src/lib/offline-queue.ts` with a typed queue backed by IndexedDB (via [`idb`](https://github.com/jakearchibald/idb)):

```ts
// Stored shape
interface QueuedMutation {
  id: string;           // uuid
  endpoint: string;
  method: 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  body: unknown;
  timestamp: number;
  retries: number;
}
```

React Query mutations wrap this queue: when `isOnline` is `false`, write to the queue instead of calling `fetch`. When `isOnline` becomes `true`, drain the queue in insertion order.

### 3. Background Sync via Service Worker

Register a sync tag in the Service Worker so the browser can retry even if the tab is closed:

```ts
// In useBookmarks.ts (or the queue drainer)
async function requestSync() {
  const registration = await navigator.serviceWorker.ready;
  await registration.sync.register('offline-mutations');
}

// In public/sw.ts
self.addEventListener('sync', (event: SyncEvent) => {
  if (event.tag === 'offline-mutations') {
    event.waitUntil(drainQueue());
  }
});
```

`drainQueue()` reads from IndexedDB, replays each mutation against the API, and removes successful entries.

### 4. Conflict resolution policy

Bookmarks and alerts are user-owned, low-contention resources. The chosen policy is **last-write-wins by client timestamp** with server-side idempotency keys:

- Each queued mutation carries a stable `idempotency-key` header (uuid generated at creation time).
- If a bookmark was deleted server-side while the client had a pending edit, the server returns `404`; the client discards the queued mutation without retrying.
- If two tabs queue conflicting edits, the one with the later `timestamp` wins.

Document this policy in `docs/OFFLINE_CONFLICT_POLICY.md` (a short reference file) and link it from the PR.

### 5. Offline UI states

Offline states to surface:

- **Banner:** persistent top bar when `isOnline === false` ("You're offline. Changes will sync when you reconnect.").
- **Bookmark/alert form:** submit button label changes to "Save for later"; spinner is replaced with a queue icon.
- **Sync indicator:** brief toast when the queue drains ("Synced N changes.").
- **Failure state:** toast with retry option when a queued mutation fails after 3 attempts.

---

## Tasks

- [ ] Workbox caching strategies per route
- [ ] IndexedDB mutation queue with typed schema
- [ ] Background Sync registration and service worker handler
- [ ] Conflict resolution policy (last-write-wins + idempotency keys)
- [ ] Offline UI states (banner, form labels, sync toast, failure toast)
- [ ] Update docs / README where behaviour changes
- [ ] Add or update tests (see test plan)

---

## Test Plan

**Primary (Playwright):** Simulate offline → create bookmark → go online → assert bookmark persists server-side.

```ts
// tests/offline-sync.spec.ts
test('bookmark created offline syncs on reconnect', async ({ page, context }) => {
  await page.goto('/bookmarks');
  await context.setOffline(true);
  await page.getByRole('button', { name: 'Bookmark' }).first().click();
  await expect(page.getByText('Save for later')).toBeVisible();
  await context.setOffline(false);
  await expect(page.getByText('Synced 1 change')).toBeVisible();
  // Assert via API that the bookmark exists
});
```

**Additional cases:**
- Offline alert edit followed by online deletion: queued edit is discarded, not applied.
- Queue drains in insertion order when multiple mutations are queued.
- Service Worker `sync` event fires correctly when tab is reopened after background sync.

---

## Acceptance Criteria

- [ ] Mutations made offline are synced to the server on reconnect (or via Background Sync)
- [ ] Conflict resolution policy is documented
- [ ] CI is green (lint, type-check, tests)
- [ ] No new TODOs without a linked issue

---

## Implementation Notes

Write a design note in the PR covering:

- IndexedDB schema version and migration strategy.
- Which browsers lack the Background Sync API (Safari) and how the drainer falls back to an online event listener.
- Maximum retry count and backoff for failed mutations.

The service worker file must be excluded from Next.js's default bundling. Use `next-pwa`'s `dest: 'public'` or a manual Workbox build step.

---

## Getting Started

```bash
git clone https://github.com/Ndifreke000/stellar-insights.git
cd stellar-insights

# Backend
cd backend && cp .env.example .env && cargo run
cargo test

# Frontend
cd ../frontend && npm install && npm run dev

# SDKs
cd ../sdk/typescript && npm install && npm test
cd ../python && pip install -e ".[dev]" && pytest

# Contracts
cd ../../contracts && cargo test --workspace
```

Verify the core endpoint:

```bash
curl -s "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
```

---

## Definition of Done

- [ ] Code merged to `main` with tests passing
- [ ] All acceptance criteria checked
- [ ] PR description explains the change, links this issue (`Closes #2837`), and lists anything deferred
- [ ] Docs updated where user-facing or integrator-facing behaviour changed
