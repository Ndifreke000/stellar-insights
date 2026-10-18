# @payraider/sdk

Official TypeScript SDK for the [PayRaider](https://github.com/Ndifreke000/payraider) API — payment corridor reliability, liquidity, trustline, and network-health data for the Stellar network.

## Install

```bash
npm install @payraider/sdk
```

## Quickstart

```ts
import { createClient } from "@payraider/sdk";

const client = createClient("mainnet"); // or "testnet"

const corridors = await client.corridors.list();
const usdcBrl = await client.corridors.get("USDC", "BRL");
```

### Custom configuration

`createClient` covers the common case of pointing at PayRaider' own
hosted API on mainnet or testnet. To talk to a self-hosted backend, or to
set an API key, construct `PayRaider` directly:

```ts
import { PayRaider } from "@payraider/sdk";

const client = new PayRaider({
  baseUrl: "https://your-backend.example.com",
  apiKey: process.env.PAYRAIDER_API_KEY,
});
```

### Check a corridor before paying

Off-ramp and payout apps can ask whether a corridor is healthy enough to pay
on right now. No API key is needed.

```ts
const check = await client.preflight.check({
  source_asset: "USDC",
  destination_asset: "NGN",
  amount_usd: 2500,
});

if (check.decision === "proceed") {
  // send the payment
} else {
  console.warn(check.summary, check.checks, check.alternatives);
}
```

`decision` is `proceed`, `caution`, `hold` or `unknown` (no recent data for
that corridor). `client.preflight.isSafeToPay(req)` returns `true` only for
`proceed`.

### Retries

Requests are retried automatically:

| | |
| --- | --- |
| Retried on | network errors, HTTP 429, 502, 503, 504 |
| Not retried | other 4xx, 500, and any non-idempotent call (POST, PATCH) |
| Idempotent methods | GET, HEAD, OPTIONS, PUT, DELETE — plus preflight and cost estimates, which are read-only POSTs |
| Max retries | 3 after the first attempt |
| Delay | the server's `Retry-After` (seconds or HTTP-date) when sent, otherwise full jitter: random(0, min(cap, base × 2^attempt)) |
| Long `Retry-After` | longer than the cap: no retry, the error is raised with the value attached |

The TypeScript and Python SDKs behave the same way.

```ts
import { PayRaider, PayRaiderError } from "@payraider/sdk";

const client = new PayRaider({
  maxRetries: 3,        // set to 0 to turn retries off
  retryDelay: 500,      // base delay in ms
  maxRetryDelay: 30_000 // cap on any single wait in ms
});

try {
  await client.corridors.list();
} catch (e) {
  if (e instanceof PayRaiderError && e.status === 429) {
    console.log("retry after", e.retryAfterMs, "ms");
  }
}
```

### Types generated from the API spec

`ApiSchemas`, `ApiPaths` and `ApiOperations` are generated from
[`docs/openapi.json`](../../docs/openapi.json), so they follow the API exactly:

```ts
import type { ApiSchemas } from "@payraider/sdk";

type Corridor = ApiSchemas["CorridorResponse"];
type Preflight = ApiSchemas["PreflightResponse"];
```

After the spec changes, run `npm run generate:types` and commit
`src/types/generated.ts`. `npm run check:types` exits non-zero when the
committed file is out of date.

### Authentication

```ts
const { access_token } = await client.auth.login({ email, password });
// subsequent requests on this client are now authenticated
```

### Real-time updates

```ts
client.subscribe((event) => {
  console.log("network event:", event);
});

// later
client.disconnect();
```

## What's included

The client exposes one resource per API surface — each is a thin,
type-safe wrapper over the REST API:

| Resource | Purpose |
| --- | --- |
| `client.corridors` | Payment corridor reliability & liquidity depth |
| `client.anchors` | Anchor directory and health |
| `client.prices` | Asset price feeds |
| `client.costCalculator` | Transaction cost estimation |
| `client.preflight` | Pre-payment corridor check (proceed / caution / hold) |
| `client.alerts` / `client.webhooks` | Alerting and webhook subscriptions |
| `client.liquidityPools` | AMM liquidity pool data |
| `client.transactions` | Transaction history and lookups |
| `client.network` | Network-wide health and trust metrics |
| `client.governance` | Soroban governance proposals and votes |
| `client.assetVerification` | Asset authenticity verification |
| `client.auth`, `client.apiKeys` | Authentication and API key management |

Built-in cross-cutting behavior: automatic retry with backoff, request
deduplication, request cancellation, and pagination helpers — see
[`CHANGELOG.md`](../CHANGELOG.md) and [`COMPATIBILITY.md`](../COMPATIBILITY.md)
for version history and compatibility guarantees.

## Framework helpers

Building a React app? See
[`@payraider/react`](../react) for hooks and pre-built components on
top of this SDK.

## License

MIT
