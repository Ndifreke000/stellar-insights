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
    if check.decision == "proceed":
        ...  # send the payment
    else:
        print(check.summary)
        for c in check.checks:
            print(c.name, c.status, c.detail)
        for alt in check.alternatives:
            print(alt.id, alt.health_score)
```

`decision` is `proceed`, `caution`, `hold` or `unknown` (no recent data for
that corridor). `client.preflight.is_safe_to_pay(...)` returns `True` only for
`proceed`.

## Typed responses

Preflight and corridor methods return [pydantic v2](https://docs.pydantic.dev/)
models instead of dicts, so a typo is a type-checker error rather than a
`KeyError` at runtime. The package ships `py.typed`.

| Method | Returns |
| --- | --- |
| `preflight.check(...)` | `PreflightResponse` |
| `corridors.list(...)` | `PaginatedResponse[Corridor]` |
| `corridors.get(...)` | `CorridorDetail` |

```python
from payraider import PayRaider, Corridor, PaginatedResponse

async with PayRaider() as client:
    page: PaginatedResponse[Corridor] = await client.corridors.list(limit=50)
    for corridor in page.data:
        print(corridor.id, corridor.success_rate)
    if page.pagination.next_cursor:
        page = await client.corridors.list(cursor=page.pagination.next_cursor)

    detail = await client.corridors.get("USDC", "NGN")
    print(detail.corridor.health_score, len(detail.historical_success_rate))
```

Fields the API adds later are kept on the model, not dropped. Dict access
such as `check["decision"]` still works but raises a `DeprecationWarning`. Use
`.model_dump()` when you need a plain dict.

Other resources return the decoded JSON (`dict` / `list`) for now.

## Errors

Every non-2xx response raises `PayRaiderError`:

```python
from payraider import PayRaiderError

try:
    await client.corridors.get("USDC", "XYZ")
except PayRaiderError as e:
    print(e.status, e.code, str(e), e.request_id, e.details, e.retry_after)
```

## Retries

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

```python
client = PayRaider(
    max_retries=3,         # set to 0 to turn retries off
    retry_delay=0.5,       # base delay in seconds
    max_retry_delay=30.0,  # cap on any single wait in seconds
)
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
mypy                          # strict type check (config in pyproject.toml)
```
