# @stellar-insights/react

React hooks and components for [Stellar Insights](https://github.com/Ndifreke000/stellar-insights), built on top of [`@stellar-insights/sdk`](../typescript).

## Install

```bash
npm install @stellar-insights/react @stellar-insights/sdk
```

## Quickstart

```tsx
import { StellarInsightsProvider, CorridorReliabilityCard } from "@stellar-insights/react";

export function App() {
  return (
    <StellarInsightsProvider network="mainnet">
      <CorridorReliabilityCard source="USDC" destination="BRL" />
    </StellarInsightsProvider>
  );
}
```

`StellarInsightsProvider` also accepts a pre-built client (useful if your
app already constructs one, e.g. with a custom `baseUrl` or auth token):

```tsx
import { StellarInsights } from "@stellar-insights/sdk";
import { StellarInsightsProvider } from "@stellar-insights/react";

const client = new StellarInsights({ baseUrl: "https://your-backend.example.com" });

<StellarInsightsProvider client={client}>...</StellarInsightsProvider>;
```

## Hooks

```tsx
import { useCorridor, useCorridors } from "@stellar-insights/react";

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
under a `<StellarInsightsProvider>`.

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
