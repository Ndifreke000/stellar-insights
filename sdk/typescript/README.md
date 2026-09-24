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
| `client.alerts` / `client.webhooks` | Alerting and webhook subscriptions |
| `client.liquidityPools` | AMM liquidity pool data |
| `client.transactions` | Transaction history and lookups |
| `client.network` | Network-wide health and trust metrics |
| `client.ml` | Predictive/ML-derived signals |
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
