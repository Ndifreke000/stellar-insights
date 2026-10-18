"""Resource classes for each API group."""
from __future__ import annotations

from typing import Any, Callable, Coroutine, List, Optional
from urllib.parse import quote

from .http import HttpClient
from .types import (
    AlertRule,
    Anchor,
    ApiKey,
    AuthTokens,
    ConvertResult,
    Corridor,
    CostEstimateResponse,
    CreateApiKeyResponse,
    LiquidityPool,
    NetworkInfo,
    PaginatedResponse,
    PreflightResponse,
    Price,
    PriceCacheStats,
    PricesBatchResult,
    Proposal,
    Transaction,
    VerifiedAsset,
    Webhook,
)


def _enc(value: str) -> str:
    return quote(value, safe="")


class AnchorsResource:
    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def list(self, *, page: Optional[int] = None, limit: Optional[int] = None) -> PaginatedResponse[Anchor]:
        return await self._http.request("GET", "/api/anchors", params={"page": page, "limit": limit})

    async def get(self, anchor_id: str) -> Anchor:
        return await self._http.request("GET", f"/api/anchors/{_enc(anchor_id)}")

    async def get_by_account(self, account: str) -> Anchor:
        return await self._http.request("GET", f"/api/anchors/account/{_enc(account)}")


class CorridorsResource:
    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def list(self, *, page: Optional[int] = None, limit: Optional[int] = None) -> PaginatedResponse[Corridor]:
        return await self._http.request("GET", "/api/corridors", params={"page": page, "limit": limit})

    async def get(self, source_or_key: str, destination: Optional[str] = None) -> Any:
        """Get one corridor by its key ("SRC->DST"), or by source and destination."""
        key = source_or_key if destination is None else f"{source_or_key}->{destination}"
        return await self._http.request("GET", f"/api/corridors/{_enc(key)}")


class PricesResource:
    """
    The backend has no path-param or no-arg "list all" route - every method
    here takes an explicit asset (or list of assets) and matches
    backend/src/api/price_feed.rs exactly. See the equivalent
    sdk/typescript/src/resources.ts fix for the full rationale.
    """

    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def get(self, asset: str) -> Price:
        return await self._http.request("GET", "/api/prices", params={"asset": asset})

    async def batch(self, assets: List[str]) -> PricesBatchResult:
        return await self._http.request("GET", "/api/prices/batch", params={"assets": ",".join(assets)})

    async def convert_to_usd(self, asset: str, amount: float) -> ConvertResult:
        """The backend only ever converts to USD - there is no arbitrary asset-to-asset conversion."""
        return await self._http.request("GET", "/api/prices/convert", params={"asset": asset, "amount": amount})

    async def cache_stats(self) -> PriceCacheStats:
        return await self._http.request("GET", "/api/prices/cache-stats")


class CostCalculatorResource:
    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def estimate(
        self, source_currency: str, destination_currency: str, source_amount: float
    ) -> CostEstimateResponse:
        return await self._http.request(
            "POST", "/api/cost-calculator/estimate",
            json={
                "source_currency": source_currency,
                "destination_currency": destination_currency,
                "source_amount": source_amount,
            },
        )


class PreflightResource:
    """Pre-payment check for off-ramp and payout apps. Works without an API key."""

    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def check(
        self,
        source_asset: str,
        destination_asset: str,
        *,
        amount_usd: Optional[float] = None,
        min_success_rate: Optional[float] = None,
    ) -> PreflightResponse:
        body = {
            "source_asset": source_asset,
            "destination_asset": destination_asset,
            "amount_usd": amount_usd,
            "min_success_rate": min_success_rate,
        }
        return await self._http.request(
            "POST", "/api/v1/preflight",
            json={key: value for key, value in body.items() if value is not None},
        )

    async def is_safe_to_pay(
        self,
        source_asset: str,
        destination_asset: str,
        *,
        amount_usd: Optional[float] = None,
    ) -> bool:
        """True only when every check passed."""
        result = await self.check(source_asset, destination_asset, amount_usd=amount_usd)
        return result["decision"] == "proceed"


class AlertsResource:
    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def list_rules(self, *, page: Optional[int] = None, limit: Optional[int] = None) -> PaginatedResponse[AlertRule]:
        return await self._http.request("GET", "/api/alerts/rules", params={"page": page, "limit": limit})

    async def create_rule(self, name: str, condition: str, threshold: float) -> AlertRule:
        return await self._http.request(
            "POST", "/api/alerts/rules",
            json={"name": name, "condition": condition, "threshold": threshold},
        )

    async def update_rule(self, rule_id: str, **kwargs: Any) -> AlertRule:
        return await self._http.request("PUT", f"/api/alerts/rules/{_enc(rule_id)}", json=kwargs)

    async def delete_rule(self, rule_id: str) -> None:
        await self._http.request("DELETE", f"/api/alerts/rules/{_enc(rule_id)}")

    async def list_history(self, *, page: Optional[int] = None, limit: Optional[int] = None) -> Any:
        return await self._http.request("GET", "/api/alerts/history", params={"page": page, "limit": limit})


class WebhooksResource:
    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def list(self) -> List[Webhook]:
        return await self._http.request("GET", "/api/webhooks")

    async def create(self, url: str, events: List[str]) -> Webhook:
        return await self._http.request("POST", "/api/webhooks", json={"url": url, "events": events})

    async def get(self, webhook_id: str) -> Webhook:
        return await self._http.request("GET", f"/api/webhooks/{_enc(webhook_id)}")

    async def delete(self, webhook_id: str) -> None:
        await self._http.request("DELETE", f"/api/webhooks/{_enc(webhook_id)}")

    async def test(self, webhook_id: str) -> Any:
        return await self._http.request("POST", f"/api/webhooks/{_enc(webhook_id)}/test")


class ApiKeysResource:
    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def list(self) -> List[ApiKey]:
        return await self._http.request("GET", "/api/api-keys")

    async def create(self, name: str) -> CreateApiKeyResponse:
        return await self._http.request("POST", "/api/api-keys", json={"name": name})

    async def get(self, key_id: str) -> ApiKey:
        return await self._http.request("GET", f"/api/api-keys/{_enc(key_id)}")

    async def rotate(self, key_id: str) -> CreateApiKeyResponse:
        return await self._http.request("POST", f"/api/api-keys/{_enc(key_id)}/rotate")

    async def revoke(self, key_id: str) -> None:
        await self._http.request("DELETE", f"/api/api-keys/{_enc(key_id)}")


class AuthResource:
    def __init__(self, http: HttpClient, set_token: Callable[[str], None]) -> None:
        self._http = http
        self._set_token = set_token

    async def login(self, email: str, password: str) -> AuthTokens:
        tokens = await self._http.request("POST", "/api/auth/login", json={"email": email, "password": password})
        self._set_token(tokens["access_token"])
        return tokens

    async def refresh(self, refresh_token: str) -> AuthTokens:
        tokens = await self._http.request("POST", "/api/auth/refresh", json={"refresh_token": refresh_token})
        self._set_token(tokens["access_token"])
        return tokens

    async def logout(self) -> None:
        await self._http.request("POST", "/api/auth/logout")


class LiquidityPoolsResource:
    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def list(self, *, page: Optional[int] = None, limit: Optional[int] = None) -> PaginatedResponse[LiquidityPool]:
        return await self._http.request("GET", "/api/liquidity-pools", params={"page": page, "limit": limit})

    async def get(self, pool_id: str) -> LiquidityPool:
        return await self._http.request("GET", f"/api/liquidity-pools/{_enc(pool_id)}")


class TransactionsResource:
    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def create(self, envelope_xdr: str) -> Transaction:
        return await self._http.request("POST", "/api/transactions", json={"envelope_xdr": envelope_xdr})

    async def get(self, tx_id: str) -> Transaction:
        return await self._http.request("GET", f"/api/transactions/{_enc(tx_id)}")

    async def submit(self, tx_id: str) -> Transaction:
        return await self._http.request("POST", f"/api/transactions/{_enc(tx_id)}/submit")


class NetworkResource:
    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def info(self) -> NetworkInfo:
        return await self._http.request("GET", "/api/network/info")

    async def available(self) -> List[NetworkInfo]:
        return await self._http.request("GET", "/api/network/available")


class GovernanceResource:
    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def list_proposals(self, *, page: Optional[int] = None, limit: Optional[int] = None) -> PaginatedResponse[Proposal]:
        return await self._http.request("GET", "/api/governance/proposals", params={"page": page, "limit": limit})

    async def create_proposal(self, title: str, description: str) -> Proposal:
        return await self._http.request("POST", "/api/governance/proposals", json={"title": title, "description": description})

    async def get_proposal(self, proposal_id: str) -> Proposal:
        return await self._http.request("GET", f"/api/governance/proposals/{_enc(proposal_id)}")

    async def vote(self, proposal_id: str, support: bool) -> Any:
        return await self._http.request("POST", f"/api/governance/proposals/{_enc(proposal_id)}/vote", json={"support": support})


class AssetVerificationResource:
    def __init__(self, http: HttpClient) -> None:
        self._http = http

    async def verify(self, asset_code: str, asset_issuer: str) -> VerifiedAsset:
        return await self._http.request("GET", f"/api/assets/verify/{_enc(asset_code)}/{_enc(asset_issuer)}")

    async def get(self, asset_code: str, asset_issuer: str) -> VerifiedAsset:
        return await self._http.request(
            "GET", f"/api/assets/{_enc(asset_code)}/{_enc(asset_issuer)}/verification"
        )

    async def list(self, *, page: Optional[int] = None, limit: Optional[int] = None) -> PaginatedResponse[VerifiedAsset]:
        return await self._http.request("GET", "/api/assets/verified", params={"page": page, "limit": limit})
