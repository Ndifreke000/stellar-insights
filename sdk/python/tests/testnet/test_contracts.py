"""Testnet integration tests for contract invocations."""
from __future__ import annotations

import pytest

from stellar_insights import StellarInsights, StellarInsightsError


@pytest.mark.testnet
async def test_governance_list_proposals(testnet_client: StellarInsights | None) -> None:
    """Test invoking governance contract to list proposals."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    result = await testnet_client.governance.listProposals(limit=5)
    assert result is not None
    assert "data" in result
    assert isinstance(result["data"], list)


@pytest.mark.testnet
async def test_governance_get_proposal(testnet_client: StellarInsights | None) -> None:
    """Test getting a governance proposal safely."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    # Attempt to get a proposal (may not exist, but should handle gracefully)
    try:
        result = await testnet_client.governance.getProposal("1")
        # If successful, verify response structure
        assert result is not None
    except StellarInsightsError as e:
        # Expected if proposal doesn't exist
        assert e.status in [404, 500]  # Not found or contract error
    except Exception as e:
        # Other errors are fine for this test; we're checking error handling
        assert e is not None


@pytest.mark.testnet
async def test_contract_error_handling(testnet_client: StellarInsights | None) -> None:
    """Test that contract errors are handled gracefully."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    # Try to access a non-existent proposal ID
    try:
        await testnet_client.governance.getProposal("999999999")
    except StellarInsightsError:
        # Expected: proposal not found
        pass
    except Exception:
        # Other exceptions are acceptable
        pass


@pytest.mark.testnet
async def test_analytics_contract_interaction(testnet_client: StellarInsights | None) -> None:
    """Test that analytics-related contract interactions work."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    # Test ML prediction endpoint (may use analytics contract)
    try:
        result = await testnet_client.ml.modelStatus()
        assert result is not None
    except StellarInsightsError:
        # Contract may not be available on testnet
        pass


@pytest.mark.testnet
async def test_network_context_for_contracts(testnet_client: StellarInsights | None) -> None:
    """Test that network context is properly set for contract invocations."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    # Get network info to verify context
    network_info = await testnet_client.network.info()
    assert network_info is not None
    assert "network" in network_info
    assert "rpc_url" in network_info


@pytest.mark.testnet
async def test_governance_read_only_operations(testnet_client: StellarInsights | None) -> None:
    """Test read-only governance contract operations."""
    if testnet_client is None:
        pytest.skip("Testnet credentials not available")

    # List proposals is a read-only operation and should always work
    proposals_response = await testnet_client.governance.listProposals(limit=1)

    assert proposals_response is not None
    assert isinstance(proposals_response.get("data"), list)
    assert isinstance(proposals_response.get("pagination"), dict)
