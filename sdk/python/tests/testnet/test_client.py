"""Testnet integration tests for the Python SDK client."""
from __future__ import annotations

import pytest

from stellar_insights import StellarInsights


@pytest.mark.testnet
async def test_client_initialization(testnet_client: StellarInsights | None) -> None:
    """Test that the SDK client initializes successfully on testnet."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    assert testnet_client is not None
    assert testnet_client.anchors is not None
    assert testnet_client.governance is not None
    assert testnet_client.network is not None


@pytest.mark.testnet
async def test_network_info(testnet_client: StellarInsights | None) -> None:
    """Test retrieving network information from testnet."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    result = await testnet_client.network.info()
    assert result is not None
    assert isinstance(result, dict)
    assert "network" in result
    assert "passphrase" in result


@pytest.mark.testnet
async def test_list_anchors(testnet_client: StellarInsights | None) -> None:
    """Test listing anchors from testnet."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    result = await testnet_client.anchors.list(limit=5)
    assert result is not None
    assert "data" in result
    assert "pagination" in result
    assert isinstance(result["data"], list)


@pytest.mark.testnet
async def test_list_prices(testnet_client: StellarInsights | None) -> None:
    """Test retrieving price data from testnet."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    result = await testnet_client.prices.list()
    assert result is not None
    assert isinstance(result, list)


@pytest.mark.testnet
async def test_available_networks(testnet_client: StellarInsights | None) -> None:
    """Test retrieving available networks from testnet."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    result = await testnet_client.network.available()
    assert result is not None
    assert isinstance(result, list)
    assert len(result) > 0


@pytest.mark.testnet
async def test_list_liquidity_pools(testnet_client: StellarInsights | None) -> None:
    """Test listing liquidity pools from testnet."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    result = await testnet_client.liquidity_pools.list(limit=10)
    assert result is not None
    assert "data" in result
    assert "pagination" in result


@pytest.mark.testnet
async def test_list_proposals(testnet_client: StellarInsights | None) -> None:
    """Test listing governance proposals from testnet."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    result = await testnet_client.governance.listProposals(limit=10)
    assert result is not None
    assert "data" in result
    assert "pagination" in result
    assert isinstance(result["data"], list)


@pytest.mark.testnet
async def test_corridors_list(testnet_client: StellarInsights | None) -> None:
    """Test listing corridors from testnet."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    result = await testnet_client.corridors.list(limit=5)
    assert result is not None
    assert "data" in result
    assert "pagination" in result
