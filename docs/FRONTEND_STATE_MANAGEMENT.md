# Frontend State Management

Each kind of state has one owner:

| Kind of state | Tool | Where |
|---------------|------|-------|
| **Server state**: anything that comes from the API | React Query | `src/lib/react-query/queries.ts` |
| **Global client state**: UI, cross-page preferences | Zustand | `src/lib/zustand/store.ts` |
| **URL state**: shareable filters, selected ids | `useSearchParams` / route params | the page |
| **Local UI state**: open/closed, input text | `useState` | the component |
| Theme, locale, keyboard shortcuts, user preferences | existing React contexts | `src/contexts/` |

If data came from the backend, it belongs in React Query. Don't copy it into `useState`
or Zustand, because that creates a second copy that goes stale.

## Server state (React Query)

### Reading data

Use the resource hooks. Don't call `fetch`/`api.get` in a `useEffect`.

```tsx
import { useCorridors, useCorridorDetail } from "@/lib/react-query/queries";

const { data, isPending, isError } = useCorridors({ sort_by: "health_score", limit: 50 });
const corridors = data?.data ?? [];
```

Hooks available: `useCorridors`, `useCorridorDetail`, `useCorridorDetails` (several in parallel),
`useAnchors`, `useAnchorDetail`, `useProposals`. `useSep24*` lives in `src/hooks/useSep24.ts`.

This is what makes the approach work:

- **One request per resource.** Components that ask for the same key share one in-flight request and one cache entry.
- **One `QueryClient`.** `ReactQueryProvider` creates it once per browser tab (`getQueryClient`). Never create a client in render.
- **Caching.** Default `staleTime` is 5 min and metrics hooks use 60 s. Cached data survives for 24 h to support offline use.

### Adding a resource

1. Add the fetch function in `src/lib/api/*`.
2. Add a key factory in `src/lib/react-query/keys.ts`, nested under the resource (`["things", "list", params]`, `["things", "detail", id]`).
3. Add a `useThing…` hook in `queries.ts`.

Never put a key array inline in a component. Keys from the factory are what make invalidation reliable.

### Cache invalidation

| Trigger | What to do |
|---------|------------|
| Mutation succeeds | `useApiMutation(fn, { invalidateQueries: [queryKeys.things] })`, or `queryClient.invalidateQueries({ queryKey: queryKeys.things })` |
| WebSocket event | Pass the `useRealtimeCacheSync()` callbacks to the realtime hooks (below) |
| Reconnect after offline | Automatic (`refetchOnReconnect`) |

Invalidate the **narrowest prefix** that covers the change. For example,
`queryKeys.corridorLists` refreshes every list but leaves detail views alone.

### Realtime updates

```tsx
const sync = useRealtimeCacheSync();
useRealtimeCorridors({ corridorKeys: [id], onCorridorUpdate: sync.onCorridorUpdate });
```

`onCorridorUpdate` patches the cached corridor detail in place with `setQueryData`, so every view of
that corridor updates together. It also marks corridor lists stale. `onAnchorUpdate` invalidates the
anchor list and that anchor's detail.

### Errors and fallbacks

Query errors are logged centrally by the `QueryCache` in `provider.tsx`. Retries skip 4xx responses and
back off exponentially otherwise. Pages that show demo data when the backend is down compute it from
`isError` at render time, so mock data never goes into the cache.

## Client state (Zustand)

`useAppStore` holds UI state: sidebar, modals, loading flags, form drafts, filters, notifications.

**Always select.** Selectors that return objects must use `useShallow`, otherwise Zustand v5 sees a new
object on every render and re-renders in a loop:

```tsx
import { useShallow } from "zustand/react/shallow";

const addNotification = useAppStore((s) => s.addNotification);               // single value
const { formData, setFormData } = useAppStore(
  useShallow((s) => ({ formData: s.formData, setFormData: s.setFormData })), // several values
);
```

The exported `useUIState`, `useFormState`, `useNotificationState`, `useAppActions`, and similar hooks already do this.
Never call `useAppStore()` with no selector in a component.

Only `sidebarCollapsed`, `formData` and `filters` are persisted to `localStorage`. Auth/session state is
**not** persisted. It's derived from the wallet/token at runtime.

## Devtools (development only)

- **React Query Devtools**: floating button, bottom-left.
- **Redux DevTools extension**: shows the Zustand store as "Stellar Insights App Store".
- **Console**: `window.__stateDevtools.logState()`, `.logQueries()`, `.getQueryData(key)`,
  `.invalidateAll()`, `.getPerformance()`.

To check for duplicate API calls, open React Query Devtools. Each resource should show one
query per distinct key, and the network tab should show one request per key per `staleTime` window.

## Pagination

List endpoints return `PaginatedResponse<T>` (`src/lib/api/pagination.ts`). See `docs/API_PAGINATION.md`.
