"""PayRaider Python SDK."""
from __future__ import annotations

from typing import Optional

from .http import HttpClient, PayRaiderError
from .resources import (
    AlertsResource,
    AnchorsResource,
    ApiKeysResource,
    AssetVerificationResource,
    AuthResource,
    CorridorsResource,
    CostCalculatorResource,
    GovernanceResource,
    LiquidityPoolsResource,
    MlResource,
    NetworkResource,
    PricesResource,
    TransactionsResource,
    WebhooksResource,
)


class PayRaider:
    """Async client for the PayRaider API.

    Usage::

        async with PayRaider(api_key="sk_...") as client:
            anchors = await client.anchors.list()
    """

    def __init__(
        self,
        *,
        api_key: Optional[str] = None,
        access_token: Optional[str] = None,
        base_url: str = "https://api.payraider.io",
        max_retries: int = 3,
        retry_delay: float = 0.5,
        timeout: float = 30.0,
    ) -> None:
        self._http = HttpClient(
            api_key=api_key,
            access_token=access_token,
            base_url=base_url,
            max_retries=max_retries,
            retry_delay=retry_delay,
            timeout=timeout,
        )
        self.anchors = AnchorsResource(self._http)
        self.corridors = CorridorsResource(self._http)
        self.prices = PricesResource(self._http)
        self.cost_calculator = CostCalculatorResource(self._http)
        self.alerts = AlertsResource(self._http)
        self.webhooks = WebhooksResource(self._http)
        self.api_keys = ApiKeysResource(self._http)
        self.auth = AuthResource(self._http, self._http.set_token)
        self.liquidity_pools = LiquidityPoolsResource(self._http)
        self.transactions = TransactionsResource(self._http)
        self.network = NetworkResource(self._http)
        self.ml = MlResource(self._http)
        self.governance = GovernanceResource(self._http)
        self.asset_verification = AssetVerificationResource(self._http)

    async def aclose(self) -> None:
        await self._http.aclose()

    async def __aenter__(self) -> "PayRaider":
        return self

    async def __aexit__(self, *_: object) -> None:
        await self.aclose()


__all__ = ["PayRaider", "PayRaiderError"]
