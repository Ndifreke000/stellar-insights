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
```
