# @payraider/react

React hooks and components for [PayRaider](https://github.com/Ndifreke000/payraider), built on top of [`@payraider/sdk`](../typescript).

## Install

```bash
npm install @payraider/react @payraider/sdk
```

## Quickstart

```tsx
import { PayRaiderProvider, CorridorReliabilityCard } from "@payraider/react";

export function App() {
  return (
    <PayRaiderProvider network="mainnet">
      <CorridorReliabilityCard source="USDC" destination="BRL" />
    </PayRaiderProvider>
  );
}
```

`PayRaiderProvider` also accepts a pre-built client (useful if your
app already constructs one, e.g. with a custom `baseUrl` or auth token):

```tsx
import { PayRaider } from "@payraider/sdk";
import { PayRaiderProvider } from "@payraider/react";

const client = new PayRaider({ baseUrl: "https://your-backend.example.com" });

<PayRaiderProvider client={client}>...</PayRaiderProvider>;
```

## Hooks

```tsx
import { useCorridor, useCorridors } from "@payraider/react";

function CorridorList() {
  const { data, isLoading, error } = useCorridors();
  // ...
}

function CorridorDetail() {
  const { data, isLoading, error, refetch } = useCorridor("USDC", "BRL");
  // ...
}
```

Both hooks return `{ data, error, isLoading, refetch }` and must be called
under a `<PayRaiderProvider>`.

### `usePreflight` — check a corridor before paying

```tsx
import { usePreflight } from "@payraider/react";

function PayoutButton({ amount }: { amount: number }) {
  const { data, isLoading, isValidating, error, refetch } = usePreflight(
    { source_asset: "USDC", destination_asset: "NGN", amount_usd: amount },
    { staleTime: 30_000 }, // optional; this is the default
  );

  if (isLoading) return <button disabled>Checking corridor…</button>;
  if (error && !data) return <button onClick={refetch}>Retry check</button>;

  return (
    <button disabled={data?.decision === "hold"} title={data?.summary}>
      {data?.decision === "proceed" ? "Pay out" : `Pay out (${data?.decision})`}
      {isValidating && " ↻"}
    </button>
  );
}
```

- **Dedupe.** Every component asking for the same params (in any key order)
  shares one in-flight request and one cached result.
- **Stale-while-revalidate.** A cached result younger than `staleTime` is
  returned without a request. An older one is returned immediately while a
  fresh one loads in the background (`isValidating` is `true` meanwhile). If
  revalidation fails, the last good `data` is kept and `error` is set.
- **Conditional.** Pass `null` as params, or `{ enabled: false }`, to skip
  fetching until inputs are ready.
- **Reset.** `clearPreflightCache()` drops every cached result, for example
  on sign-out or between tests. `clearPreflightCache(client)` drops one
  client's results.

The cache lives in memory per client, so mainnet and testnet providers never
share results. It uses no browser- or Node-only APIs and works in React
Native.

## Components

- **`CorridorReliabilityCard`** — success rate, latency, and volume for a
  single corridor. Ships with minimal inline default styling (no CSS or
  Tailwind dependency) so it drops into any host app; pass `className` to
  restyle it with your own design system.

More components/hooks (trust scores, network health, liquidity pools) can
follow the same pattern — see `src/hooks/useCorridor.ts` for the shape to
copy.

## License

MIT
