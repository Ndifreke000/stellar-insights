# stellar-insights

Official Python SDK for the [Stellar Insights](https://stellar-insights.com) API — real-time payment corridor and anchor analytics for the Stellar network.

## Install

```bash
pip install stellar-insights
```

## Usage

```python
import asyncio
from stellar_insights import StellarInsights

async def main():
    async with StellarInsights(api_key="sk_...") as client:
        anchors = await client.anchors.list()
        corridor = await client.corridors.get("USDC:issuer", "native")
        price = await client.prices.get("XLM:native")

asyncio.run(main())
```

Point at testnet or a local backend with `base_url`:

```python
client = StellarInsights(api_key="sk_...", base_url="http://localhost:8080")
```

## Resources

`anchors`, `corridors`, `prices`, `cost_calculator`, `alerts`, `webhooks`,
`api_keys`, `auth`, `liquidity_pools`, `transactions`, `network`, `ml`,
`governance`, `asset_verification` — see `stellar_insights/resources.py` for
the full method list on each.

## Development

```bash
pip install -e ".[dev]"
pytest                        # unit tests
pytest -m testnet             # integration tests against a real backend (needs credentials)
```
