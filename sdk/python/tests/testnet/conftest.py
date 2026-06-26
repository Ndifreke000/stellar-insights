"""Shared fixtures for testnet integration tests."""
from __future__ import annotations

import os
from typing import AsyncGenerator

import pytest

from stellar_insights import StellarInsights

# Environment variables for testnet configuration
TESTNET_API_URL = os.getenv("TESTNET_API_URL", "https://testnet-api.stellarinsights.io")
TESTNET_SECRET_KEY = os.getenv("TESTNET_SECRET_KEY")


def pytest_configure(config: pytest.Config) -> None:
    """Register the testnet marker."""
    config.addinivalue_line("markers", "testnet: marks tests as testnet integration tests (deselect with '-m \"not testnet\"')")


def pytest_collection_modifyitems(config: pytest.Config, items: list) -> None:
    """Skip testnet tests if credentials are missing."""
    if TESTNET_SECRET_KEY:
        return  # All testnet tests can run

    for item in items:
        if "testnet" in item.keywords:
            item.add_marker(pytest.mark.skip(reason="TESTNET_SECRET_KEY not set"))


@pytest.fixture
async def testnet_client() -> AsyncGenerator[StellarInsights | None, None]:
    """Create a testnet client if credentials are available.

    Yields:
        StellarInsights client configured for testnet, or None if credentials missing.
        The client is async context manager that handles cleanup.

    Usage:
        @pytest.mark.testnet
        async def test_something(testnet_client):
            if testnet_client is None:
                pytest.skip("Testnet credentials not available")
            # use testnet_client
    """
    if not TESTNET_SECRET_KEY:
        yield None
        return

    async with StellarInsights(
        api_key=TESTNET_SECRET_KEY,
        base_url=TESTNET_API_URL,
    ) as client:
        yield client
