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
