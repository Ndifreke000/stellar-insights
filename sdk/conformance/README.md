# SDK conformance cases

One set of recorded HTTP exchanges that the TypeScript and Python SDKs must
both pass, so the two cannot drift apart unnoticed.

Each file in [`cases/`](cases) is one case:

```json
{
  "name": "prices_get_sends_asset_as_query",
  "description": "What the case pins down, in one sentence.",
  "client": { "api_key": "optional" },
  "operation": "prices.get",
  "input": { "asset": "XLM:native" },
  "request": {
    "method": "GET",
    "path": "/api/prices",
    "query": { "asset": "XLM:native" },
    "body": null,
    "headers": { "authorization": "optional, lower-case names" }
  },
  "response": { "status": 200, "body": { "...": "recorded backend response" } },
  "expect": { "result": { "...": "what the SDK method returns" } }
}
```

A runner builds a client (base URL `https://conformance.test`, no retries,
plus anything in `client`), calls `operation` with `input`, and checks:

- the one request sent matches `request`: method, percent-encoded `path`,
  decoded `query` (all values strings, compared exactly), JSON `body` (`null`
  means no body) and every listed header;
- the SDK returned `expect.result` (deep equal; a 204 is `null`), or raised
  its API error with `expect.error`'s `status`, `code` and `message`.

No network is used: the runners intercept the HTTP call and replay
`response`.

## Operations

`operation` is a neutral name; each runner maps it onto its SDK's own call
signature in one table, so the differences between the SDKs are explicit.

| Operation | `input` | TypeScript | Python |
| --- | --- | --- | --- |
| `preflight.check` | `source_asset`, `destination_asset`, `amount_usd?`, `min_success_rate?` | `preflight.check(input)` | `preflight.check(src, dst, amount_usd=, min_success_rate=)` |
| `preflight.check_batch` | `items` | `preflight.checkBatch(items)` | `preflight.check_batch(items)` |
| `prices.get` | `asset` | `prices.get(asset)` | `prices.get(asset)` |
| `prices.batch` | `assets` | `prices.batch(assets)` | `prices.batch(assets)` |
| `anchors.list` | `page?`, `limit?` | `anchors.list({ page, limit })` | `anchors.list(page=, limit=)` |
| `anchors.get` | `id` | `anchors.get(id)` | `anchors.get(id)` |
| `corridors.get` | `source`, `destination?` | `corridors.get(source, destination)` | `corridors.get(source, destination)` |
| `webhooks.delete` | `id` | `webhooks.delete(id)` | `webhooks.delete(id)` |
| `network.info` | none | `network.info()` | `network.info()` |

## Run

```bash
cd sdk/typescript && npm test -- conformance   # src/conformance.test.ts
cd sdk/python && pytest tests/test_conformance.py
```

## Add a case

1. Record the exchange against a running backend, or copy one from an
   existing case, and save it as `cases/<behaviour>.json`. Name it after the
   behaviour (`anchors_get_raises_not_found`), not the implementation.
2. If it uses a new `operation`, add a row to the table above and to the
   operation table in both runners.
3. Run both runners. A case that only one SDK passes is a real difference:
   fix the SDK, do not loosen the case.
