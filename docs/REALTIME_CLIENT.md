# Frontend Realtime WebSocket Client Architecture

This document describes the consolidated realtime WebSocket architecture implemented for Issue #2834.

---

## 1. Overview

Previously, `frontend/src/lib/websocket.ts`, `frontend/src/hooks/useWebSocket.ts`, and `frontend/src/hooks/useRealtimeCollaboration.ts` each opened independent `WebSocket` connections. Mounting multiple realtime hooks on a page created multiple sockets per browser tab, consuming per-IP backend connection slots (`WS_MAX_CONNECTIONS_PER_IP = 10`) and draining mobile batteries.

All realtime hooks now share **one connection manager singleton per tab** (`RealtimeClient` in `frontend/src/lib/websocket.ts`), distributed through `RealtimeProvider` (`frontend/src/contexts/RealtimeContext.tsx`) mounted inside `StateProvider`.

---

## 2. Key Mechanisms

### 2.1 Singleton Connection Manager & Consumer Reference Counting
- `getRealtimeClient(config?)` normalizes URLs so callers passing `undefined`, `""`, or `NEXT_PUBLIC_WS_URL` resolve to the exact same `RealtimeClient` instance.
- Consumers attach via `client.attach()` on mount and detach on unmount. The socket opens on `0 → 1` consumers and closes cleanly on `1 → 0`.

### 2.2 Ref-Counted Channel Subscriptions
- `acquireChannels(channels)` increments reference counts per channel and sends a wire `{ type: "subscribe", channels, last_seq? }` frame **only** when a channel transitions `0 → 1`.
- `releaseChannels(channels)` decrements reference counts and sends `{ type: "unsubscribe", channels }` **only** when a channel transitions `1 → 0`.
- Each `useWebSocket` hook tracks its own acquired channel set so repeated calls are idempotent and every channel is released automatically on unmount.
- Incoming `Sequenced` / `Replayed` messages record the highest `seq` per channel and pass `last_seq` on reconnect for lossless catch-up.

### 2.3 Jittered Exponential Backoff & Visibility Awareness
- Reconnect delays use equal jitter: `delay = round(min(maxDelay, base * factor^(attempt - 1)) * (0.5 + rand * 0.5))`.
- Only one reconnect timer can exist at a time, so a server disconnect triggers **one** reconnect regardless of how many components are subscribed.
- `document.visibilitychange` pauses pending reconnect timers while a tab is hidden and resumes with a single reconnect when visible again; `window` `offline`/`online` events pause retries while offline.

### 2.4 React Query Cache Patching
- `RealtimeProvider` listens once on the shared `RealtimeClient` and patches the React Query cache in place via `syncRealtimeMessageToQueryCache`:
  - `corridor_update` patches `queryKeys.corridorDetail(key)` and matching rows across every `queryKeys.corridorLists` (`useQuery` and `useInfiniteQuery`), preserving object identity of untouched pages and rows.
  - `anchor_update` patches `queryKeys.anchorDetail(id)` and matching rows in `queryKeys.anchorLists`.
  - `snapshot_update` invalidates stale list queries.

---

## 3. Migrated Hooks

- `useWebSocket` (`frontend/src/hooks/useWebSocket.ts`)
- `useRealtimeCorridors` (`frontend/src/hooks/useRealtimeCorridors.ts`)
- `useRealtimeAnchors` (`frontend/src/hooks/useRealtimeAnchors.ts`)
- `useRealtimeCollaboration` (`frontend/src/hooks/useRealtimeCollaboration.ts`)
- `NotificationProvider` (`frontend/src/contexts/NotificationContext.tsx`)
