# React SDK: Suspense-ready hooks with SSR hydration

> Design doc for issue **#2844**. Status: **proposed**, not yet implemented.

## Problem

`@payraider/react` hooks fetch data only in the browser. Next.js (App Router) integrators who want to render corridor data on the server, stream it with `<Suspense>`, or avoid a loading spinner on first paint have no supported way to do that today.

## Current state

| File | Behaviour |
| --- | --- |
| [`sdk/react/src/PayRaiderProvider.tsx`](../sdk/react/src/PayRaiderProvider.tsx) | Client-only context that holds one `PayRaider` client. Has no `"use client"` directive and no notion of initial or prefetched data. |
| [`sdk/react/src/hooks/useCorridor.ts`](../sdk/react/src/hooks/useCorridor.ts), [`useCorridors.ts`](../sdk/react/src/hooks/useCorridors.ts) | `useState` + `useEffect`. Each mounted component fetches on its own (no dedupe, no cache), and nothing renders on the server because effects don't run there. `isLoading` starts as `true`. |
| [`sdk/react/src/hooks/usePreflight.ts`](../sdk/react/src/hooks/usePreflight.ts) | Already has a per-client cache (`WeakMap<PayRaider, Map<key, entry>>`), request dedupe, stale-while-revalidate and `useSyncExternalStore`. Its server snapshot is the same as its client snapshot, so it also renders empty on the server. |
| [`sdk/typescript/src/websocket-manager.ts`](../sdk/typescript/src/websocket-manager.ts) | Ref-counted WebSocket connections keyed by `rpcUrl + network`. The React package does not use it. |

`usePreflight` is the right base to build on. The work is mostly about **making that cache generic and serialisable**, then putting the other hooks on top of it.

## Design

### 1. Generic query cache (`sdk/react/src/cache/`)

Pull the cache out of `usePreflight.ts` into a reusable `QueryCache`:

```ts
interface QueryState<T> { data: T | null; error: Error | null; updatedAt: number; isValidating: boolean }

class QueryCache {
  get<T>(key: QueryKey): QueryState<T>;
  fetch<T>(key: QueryKey, fn: () => Promise<T>): Promise<T>; // dedupes in-flight requests
  set<T>(key: QueryKey, data: T, updatedAt?: number): void;  // used by hydration and realtime
  subscribe(key: QueryKey, cb: () => void): () => void;
  dehydrate(): DehydratedState;                              // JSON-safe snapshot
  hydrate(state: DehydratedState): void;                     // never overwrites newer data
}
```

- Keys look like `["corridor", source, destination]`, `["corridors", params]` and `["preflight", params]`, serialised with the existing `preflightKey` stable-stringify logic.
- Each `PayRaiderProvider` owns one cache, so two providers (mainnet and testnet) never share entries. This matches how the preflight cache is scoped today.
- `MAX_ENTRIES` eviction and the 30 s default `staleTime` carry over unchanged.
- `clearPreflightCache` and `preflightKey` stay exported for backward compatibility.

### 2. Server data helpers (RSC-safe entry point)

Add a separate entry point, `@payraider/react/server`, with no React client APIs in it, so React Server Components can import it:

```ts
// app/corridors/[pair]/page.tsx (Server Component)
import { createClient } from "@payraider/sdk";
import { createServerCache, prefetchCorridor, dehydrate } from "@payraider/react/server";
import { PayRaiderHydrate } from "@payraider/react";

export default async function Page({ params }) {
  const client = createClient("mainnet");
  const cache = createServerCache();               // one per request; never module-global
  await prefetchCorridor(cache, client, "USDC", params.pair);
  return (
    <PayRaiderHydrate state={dehydrate(cache)}>
      <CorridorPanel source="USDC" destination={params.pair} />
    </PayRaiderHydrate>
  );
}
```

Helpers: `prefetchCorridor`, `prefetchCorridors`, `prefetchPreflight`, plus a generic `prefetchQuery(cache, key, fn)`.

Rules:
- A cache must be created **per request**. A module-level cache on the server would leak one user's data into another user's response.
- `dehydrate` only includes successful entries. Errors are not serialised, so the client retries.
- The build adds `src/server.ts` to the `tsup` entries and a `"./server"` entry under `exports` in `package.json`.

### 3. Hydration and Suspense

- `PayRaiderProvider.tsx` and the hooks get a `"use client"` banner (tsup `banner` option or a directive at the top of each file).
- New `<PayRaiderHydrate state>` component that calls `cache.hydrate(state)` **during render** (not in an effect), so the first client render already has the data and **no client refetch happens** while the data is fresh.
- Hooks pass a real `getServerSnapshot` to `useSyncExternalStore` that reads the hydrated cache, so server and client markup match.
- Suspense variants: `useSuspenseCorridor`, `useSuspenseCorridors`, `useSuspensePreflight`. While there is no data they throw the in-flight promise from `cache.fetch` and they return `data: T` (never `null`). Errors are thrown to the nearest error boundary.
- The existing non-suspense hooks keep their return shape (`data | null`, `isLoading`, `refetch`). `useCorridor` and `useCorridors` switch to the shared cache internally, which also gives them request dedupe.
- An alternative considered was a single `{ suspense: true }` option. It was rejected because the return type would have to change depending on a runtime flag.

### 4. Realtime via the shared client

- `useCorridorSubscription(source, destination)` (and a lower-level `useRealtime(topic, handler)`) acquires a connection through `acquireConnection` / `releaseConnection` in `websocket-manager.ts`, so every component on a page shares one socket.
- Incoming events call `cache.set(key, data)`. Every hook reading that key re-renders, and no extra HTTP request is made.
- Subscriptions are opened only in effects, never on the server.

## Docs

- Add a "Server rendering & Suspense" section to [`sdk/react/README.md`](../sdk/react/README.md) with the Next.js App Router example above, a Pages Router (`getServerSideProps` + `dehydrate`) variant, and the per-request-cache warning.
- Add an `examples/nextjs` folder under `sdk/react/` (or a snippet in the README if a full example app is too heavy for CI).

## Tasks

- [ ] Extract `QueryCache` from `usePreflight.ts`; port `usePreflight` to it with no behaviour change (existing tests must pass unchanged)
- [ ] Port `useCorridor` / `useCorridors` to the cache
- [ ] `@payraider/react/server` entry: `createServerCache`, `prefetch*`, `dehydrate`
- [ ] `<PayRaiderHydrate>` + `getServerSnapshot` + `"use client"` banner
- [ ] `useSuspense*` hooks
- [ ] Realtime hook on top of `websocket-manager.ts`
- [ ] README + Next.js example
- [ ] Tests (below)

## Test plan

Vitest with `react-dom/server` (already a dev dependency through `react-dom`):

1. **SSR with prefetched data, no client refetch.** This is the acceptance test. Prefetch with a mocked client, then `renderToString` inside `<PayRaiderHydrate>` and assert that the markup contains the corridor data. Next, `hydrateRoot` on the same markup with a fresh mocked client and assert that `client.corridors.get` was called **0 times** and that there are no hydration-mismatch warnings.
2. Stale hydrated data (older than `staleTime`) renders immediately, then revalidates exactly once.
3. A suspense hook throws a promise, resolves, and renders. An API error reaches the error boundary.
4. Two components with the same key cause one request.
5. Two simultaneous server caches don't share data (request isolation).
6. A realtime event updates every subscriber without an HTTP call, and unmounting releases the socket.

## Acceptance criteria

- SSR and Suspense supported, as tested above
- No breaking change to existing hook return types
- CI green (`npm run lint`, `npm test` in `sdk/react`)
- No new TODOs without a linked issue
