# payraider

Official Python SDK for the [PayRaider](https://payraider.com) API — real-time payment corridor and anchor analytics for the Stellar network.

## Install

```bash
pip install payraider
```

## Usage

```python
import asyncio
from payraider import PayRaider

async def main():
    async with PayRaider(api_key="sk_...") as client:
        anchors = await client.anchors.list()
        corridor = await client.corridors.get("USDC:issuer", "XLM:native")
        price = await client.prices.get("XLM:native")

asyncio.run(main())
```

Point at testnet or a local backend with `base_url`:

```python
client = PayRaider(api_key="sk_...", base_url="http://localhost:8080")
```

## Check a corridor before paying

Off-ramp and payout apps can ask whether a corridor is healthy enough to pay
on right now. No API key is needed.

```python
async with PayRaider() as client:
    check = await client.preflight.check("USDC", "NGN", amount_usd=2500)
    if check["decision"] == "proceed":
        ...  # send the payment
    else:
        print(check["summary"], check["checks"], check["alternatives"])
```

`decision` is `proceed`, `caution`, `hold` or `unknown` (no recent data for
that corridor).

To check a payout batch, send up to 100 payments in one call. An invalid
payment comes back with an `error` instead of failing the rest:

```python
batch = await client.preflight.check_batch([
    {"source_asset": "USDC", "destination_asset": "NGN", "amount_usd": 2500},
    {"source_asset": "USDC", "destination_asset": "KES", "amount_usd": 900},
])
for item in batch["results"]:
    if item["error"]:
        print(item["index"], item["error"]["code"])
    elif item["result"]["decision"] != "proceed":
        print(item["index"], item["result"]["summary"])
```

## Resources

`anchors`, `corridors`, `prices`, `cost_calculator`, `preflight`, `alerts`, `webhooks`,
`api_keys`, `auth`, `liquidity_pools`, `transactions`, `network`, `ml`,
`governance`, `asset_verification` — see `payraider/resources.py` for
the full method list on each.

## Development

```bash
pip install -e ".[dev]"
pytest                        # unit tests
pytest -m testnet             # integration tests against a real backend (needs credentials)
pytest tests/test_conformance.py  # shared cases in ../conformance, also run by the TypeScript SDK
```

The conformance cases are recorded HTTP exchanges both SDKs must handle the
same way; see [`../conformance`](../conformance/README.md).
