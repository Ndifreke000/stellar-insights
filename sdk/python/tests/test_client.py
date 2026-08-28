"""Tests for the Stellar Insights Python SDK."""
import pytest
import httpx
import respx

from stellar_insights import StellarInsights, StellarInsightsError

BASE = "https://api.stellarinsights.io"


@pytest.fixture
def client():
    return StellarInsights(api_key="test-key", max_retries=0)


@respx.mock
async def test_sends_bearer_token(client):
    route = respx.get(f"{BASE}/api/anchors").mock(
        return_value=httpx.Response(200, json={"data": [], "pagination": {}})
    )
    await client.anchors.list()
    assert route.called
    assert route.calls[0].request.headers["Authorization"] == "Bearer test-key"


@respx.mock
async def test_pagination_params(client):
    route = respx.get(f"{BASE}/api/anchors").mock(
        return_value=httpx.Response(200, json={"data": [], "pagination": {}})
    )
    await client.anchors.list(page=2, limit=10)
    url = str(route.calls[0].request.url)
    assert "page=2" in url
    assert "limit=10" in url


@respx.mock
async def test_raises_on_401(client):
    respx.get(f"{BASE}/api/anchors").mock(
        return_value=httpx.Response(401, json={"error": "UNAUTHORIZED", "message": "Invalid API key"})
    )
    with pytest.raises(StellarInsightsError) as exc_info:
        await client.anchors.list()
    assert exc_info.value.status == 401
    assert exc_info.value.code == "UNAUTHORIZED"


@respx.mock
async def test_raises_on_404(client):
    respx.get(f"{BASE}/api/anchors/missing").mock(
        return_value=httpx.Response(404, json={"error": "NOT_FOUND", "message": "Anchor not found"})
    )
    with pytest.raises(StellarInsightsError) as exc_info:
        await client.anchors.get("missing")
    assert exc_info.value.status == 404


@respx.mock
async def test_retries_on_429():
    client = StellarInsights(api_key="test-key", max_retries=2, retry_delay=0)
    route = respx.post(f"{BASE}/api/cost-calculator/estimate").mock(
        side_effect=[
            httpx.Response(429, json={"error": "RATE_LIMITED", "message": "Too many requests"}),
            httpx.Response(429, json={"error": "RATE_LIMITED", "message": "Too many requests"}),
            httpx.Response(200, json={"routes": []}),
        ]
    )
    result = await client.cost_calculator.estimate("USD:GXXX", "EUR:GYYY", 100)
    assert route.call_count == 3
    assert result == {"routes": []}


@respx.mock
async def test_post_body(client):
    route = respx.post(f"{BASE}/api/cost-calculator/estimate").mock(
        return_value=httpx.Response(200, json={"routes": []})
    )
    await client.cost_calculator.estimate("USD:GXXX", "EUR:GYYY", 500.0)
    import json
    body = json.loads(route.calls[0].request.content)
    assert body["amount"] == 500.0
    assert body["source_asset"] == "USD:GXXX"


@respx.mock
async def test_context_manager():
    async with StellarInsights(api_key="test-key") as client:
        respx.get(f"{BASE}/api/network").mock(
            return_value=httpx.Response(200, json={"network": "testnet", "passphrase": "x", "horizon_url": "y", "rpc_url": "z"})
        )
        result = await client.network.info()
        assert result["network"] == "testnet"


@respx.mock
async def test_prices_get_uses_asset_query_param(client):
    route = respx.get(f"{BASE}/api/prices").mock(
        return_value=httpx.Response(200, json={"asset": "XLM:native", "price_usd": 0.17, "stale": False, "timestamp": "t"})
    )
    result = await client.prices.get("XLM:native")
    assert route.called
    assert route.calls[0].request.url.params["asset"] == "XLM:native"
    assert result["price_usd"] == 0.17


@respx.mock
async def test_prices_batch_joins_assets(client):
    route = respx.get(f"{BASE}/api/prices/batch").mock(
        return_value=httpx.Response(200, json={"prices": {"XLM:native": 0.17}, "stale": False, "timestamp": "t"})
    )
    await client.prices.batch(["XLM:native", "USDC:issuer"])
    assert route.calls[0].request.url.params["assets"] == "XLM:native,USDC:issuer"


@respx.mock
async def test_prices_convert_to_usd(client):
    route = respx.get(f"{BASE}/api/prices/convert").mock(
        return_value=httpx.Response(200, json={"asset": "XLM:native", "amount": 100.0, "amount_usd": 17.0, "price_usd": 0.17, "timestamp": "t"})
    )
    result = await client.prices.convert_to_usd("XLM:native", 100.0)
    params = route.calls[0].request.url.params
    assert params["asset"] == "XLM:native"
    assert params["amount"] == "100.0"
    assert result["amount_usd"] == 17.0
