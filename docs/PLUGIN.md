# PayRaider plugin: check a corridor before you pay

PayRaider gives off-ramp and payout applications one question to ask before
money moves: **is this corridor healthy enough to pay on right now?**

The answer is a single decision with the reasons behind it:

| Decision | Meaning | What an app should do |
| --- | --- | --- |
| `proceed` | Every check passed | Send the payment |
| `caution` | At least one check is marginal | Send with care, or use an alternative corridor |
| `hold` | At least one check failed | Do not pay on this corridor now |
| `unknown` | No recent payments observed | No recommendation; apply your own policy |

The check is public and read-only. It needs no API key, so you can integrate
and evaluate it before paying for anything; the anonymous rate-limit tier
applies. An API key raises the limit.

There are four ways to use it. Pick the one that fits your stack.

## 1. REST

```bash
curl -s -X POST "$PAYRAIDER_BASE_URL/api/v1/preflight" \
  -H 'content-type: application/json' \
  -d '{"source_asset":"USDC","destination_asset":"NGN","amount_usd":2500}'
```

`GET /api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500`
is equivalent.

### Request

| Field | Required | Meaning |
| --- | --- | --- |
| `source_asset` | yes | Asset the payment is sent in: a code (`USDC`) or `CODE:ISSUER` |
| `destination_asset` | yes | Asset the recipient receives: a code (`NGN`) or `CODE:ISSUER` |
| `amount_usd` | no | Payment size in USD. Omit to skip the liquidity check |
| `min_success_rate` | no | Minimum success rate in percent. Default 95 |
| `max_p95_latency_ms` | no | Maximum p95 latency in milliseconds. Default 5000 |

Asset codes match case-insensitively. When several issuers serve the same code
pair and you give only codes, the healthiest corridor is evaluated; give
`CODE:ISSUER` to pin one.

### Response

For a 100,000 USD payment on a corridor with 166,600 USD of observed
liquidity:

```json
{
  "decision": "hold",
  "summary": "Failed on: liquidity. Do not pay on this corridor right now.",
  "score": 83.0,
  "corridor": { "id": "USDC:G...->NGN:G...", "success_rate": 100.0, "p95_latency_ms": 1500.0, "liquidity_depth_usd": 166600.0, "...": "..." },
  "checks": [
    { "name": "success_rate", "status": "pass", "detail": "100.0% of recent payments succeeded (minimum 95.0%)" },
    { "name": "liquidity", "status": "fail", "detail": "payment is 60.0% of $166600 observed liquidity" },
    { "name": "latency", "status": "pass", "detail": "p95 settlement latency 1500 ms (maximum 5000 ms)" },
    { "name": "sample_size", "status": "pass", "detail": "33 recent payments observed (at least 20 for confidence)" },
    { "name": "health_score", "status": "pass", "detail": "corridor health score 83.0 of 100" }
  ],
  "alternatives": [],
  "evaluated_at": "2026-01-01T00:00:00+00:00"
}
```

`corridor` and `score` are `null`, and `checks` is empty, when the decision is
`unknown`. `alternatives` lists up to three healthier corridors to the same
destination asset, best first.

Invalid input (a missing asset, a non-positive amount, a threshold out of
range) returns `400`.

### How the decision is made

| Check | Pass | Warn | Fail |
| --- | --- | --- | --- |
| `success_rate` | at or above the minimum | within 10 points below it | lower |
| `liquidity` | payment is at most 10% of observed liquidity | at most 50% | more, or no liquidity |
| `latency` | p95 at or below the maximum | at most twice the maximum | higher |
| `sample_size` | at least 20 recent payments | fewer | never fails |
| `health_score` | 80 or above | 60 or above | lower |

Any `fail` gives `hold`. Otherwise any `warn` gives `caution`. Otherwise
`proceed`.

## 2. SDK

TypeScript ([`sdk/typescript`](../sdk/typescript)):

```ts
import { PayRaider } from "@payraider/sdk";

const client = new PayRaider({ baseUrl: process.env.PAYRAIDER_BASE_URL });

const check = await client.preflight.check({
  source_asset: "USDC",
  destination_asset: "NGN",
  amount_usd: 2500,
});

if (check.decision !== "proceed") {
  throw new Error(`Payout blocked: ${check.summary}`);
}
```

Python ([`sdk/python`](../sdk/python)):

```python
import os

from payraider import PayRaider

async with PayRaider(base_url=os.environ["PAYRAIDER_BASE_URL"]) as client:
    check = await client.preflight.check("USDC", "NGN", amount_usd=2500)
    if check["decision"] != "proceed":
        raise RuntimeError(f"Payout blocked: {check['summary']}")
```

## 3. MCP server (for AI agents and hosted use)

[`sdk/mcp-server`](../sdk/mcp-server) exposes the check as the
`preflight_payment` tool, alongside read-only corridor, anchor, price and
asset-verification tools. It runs over stdio for a local client, or over
streamable HTTP as a hosted service that several apps can share:

```bash
# from the sdk/ directory
docker build -f mcp-server/Dockerfile -t payraider-mcp .
docker run --rm -p 3333:3333 \
  -e PAYRAIDER_BASE_URL=https://your-backend.example.com \
  -e PAYRAIDER_MCP_AUTH_TOKEN=$(openssl rand -hex 32) \
  payraider-mcp
```

Clients connect to `POST https://your-host/mcp` with
`Authorization: Bearer <token>`. `GET /healthz` is the liveness probe. See the
[server README](../sdk/mcp-server/README.md) for every option.

## 4. Claude plugin

```
/plugin marketplace add Ndifreke000/stellar-insights
/plugin install payraider@payraider
```

Then `/preflight USDC NGN 2500`, or ask in plain language. See the
[plugin README](../plugins/payraider/README.md).

## Getting an API key

Without a key every caller shares the anonymous per-IP limit. A key gives an
app its own bucket (200 requests a minute by default; per-key limits live in
`api_keys_rate_limit_config`). Keys belong to a Stellar wallet:

1. `POST /api/sep10/auth` with `{"account": "G..."}` returns a `transaction`
   (the challenge).
2. Sign that string with the wallet's key, either with Freighter's
   `signMessage` (SEP-53) or as a raw Ed25519 signature, and send
   `POST /api/sep10/verify` with `{"transaction": "<challenge>", "signature": "<base64>"}`.
   The response holds a session `token`.
3. `POST /api/api-keys` with `Authorization: Bearer <token>` and
   `{"name": "my-offramp"}`. The response's `plain_key` (`si_live_...`) is
   shown once; store it.
4. Send `Authorization: Bearer si_live_...` on API calls, or set
   `PAYRAIDER_API_KEY` for the SDKs and the MCP server.

`GET /api/api-keys`, `POST /api/api-keys/{id}/rotate` and
`DELETE /api/api-keys/{id}` manage the wallet's keys with the same session
token. A revoked key stops being honoured within a minute.

Wallet sign-in needs Redis (`REDIS_URL`): challenges and sessions are stored
there, and without it sign-in is refused rather than allowed.

## Deploying it yourself

1. **Run the backend.** The plugin reads from a PayRaider backend; see
   [testnet quickstart](testnet-quickstart.md). Note its public URL.
2. **Run the MCP server** with `PAYRAIDER_BASE_URL` set to that URL, using the
   Docker image, `docker compose` or the Fly.io config in `sdk/mcp-server`.
3. **Set `PAYRAIDER_MCP_AUTH_TOKEN`** if the server is reachable from the
   internet, and give that token to the apps that should use it.
4. **Point apps at it**: REST and SDK callers use the backend URL directly;
   MCP clients use `https://your-host/mcp`.

## What it does not do

- It does not move funds, sign transactions or hold keys.
- It does not guarantee a payment will succeed. It reports what recent
  payments on the corridor looked like at the time of the call.
- Corridor metrics are built from the Stellar payment stream, which contains
  successful payments only, so the success-rate check reflects that source.
- A corridor with no recent payments returns `unknown`, not a guess.
