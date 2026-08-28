"""Typed dataclasses for Stellar Insights API responses."""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Generic, List, Optional, TypeVar

T = TypeVar("T")


@dataclass
class Pagination:
    page: int
    limit: int
    total: int
    pages: int


@dataclass
class PaginatedResponse(Generic[T]):
    data: List[T]
    pagination: Pagination


@dataclass
class AnchorMetrics:
    total_transactions: int
    success_rate: float
    avg_transaction_time_ms: float
    total_volume_usd: float


@dataclass
class Anchor:
    id: str
    name: str
    account: str
    status: str
    metrics: AnchorMetrics
    domain: Optional[str] = None


@dataclass
class Corridor:
    source: str
    destination: str
    volume_usd: float
    success_rate: float
    avg_latency_ms: float


# Mirrors the backend's actual PriceResponse/PricesResponse/ConvertResponse
# (backend/src/api/price_feed.rs) - the previous shape (change_24h, rate,
# from_asset/to_asset) never matched what the API returns.
@dataclass
class Price:
    asset: str
    price_usd: float
    stale: bool
    timestamp: str


@dataclass
class PricesBatchResult:
    prices: dict[str, float]
    stale: bool
    timestamp: str


@dataclass
class ConvertResult:
    asset: str
    amount: float
    amount_usd: float
    price_usd: float
    timestamp: str


@dataclass
class PriceCacheStats:
    total_cached: int
    fresh_cached: int
    timestamp: str


@dataclass
class RouteFees:
    network_fee: float
    bridge_fee: float
    liquidity_fee: float


@dataclass
class PaymentRoute:
    path: List[str]
    total_cost: float
    exchange_rate: float
    fees: RouteFees
    estimated_time_ms: int


@dataclass
class CostEstimateResponse:
    routes: List[PaymentRoute]


@dataclass
class AlertRule:
    id: str
    name: str
    condition: str
    threshold: float
    enabled: bool
    created_at: str


@dataclass
class Webhook:
    id: str
    url: str
    events: List[str]
    active: bool
    created_at: str


@dataclass
class ApiKey:
    id: str
    name: str
    prefix: str
    created_at: str
    last_used_at: Optional[str] = None


@dataclass
class CreateApiKeyResponse(ApiKey):
    key: str = ""  # only returned on creation


@dataclass
class AuthTokens:
    access_token: str
    refresh_token: str
    expires_in: int


@dataclass
class LiquidityPool:
    id: str
    assets: List[str]
    total_value_usd: float
    volume_24h_usd: float
    fee_rate: float


@dataclass
class Transaction:
    id: str
    status: str
    created_at: str
    hash: Optional[str] = None


@dataclass
class NetworkInfo:
    network: str
    passphrase: str
    horizon_url: str
    rpc_url: str


@dataclass
class PaymentPrediction:
    success_probability: float
    risk_score: float
    factors: dict[str, float] = field(default_factory=dict)


@dataclass
class Proposal:
    id: str
    title: str
    description: str
    status: str
    votes_for: int
    votes_against: int
    created_at: str


@dataclass
class VerifiedAsset:
    asset_code: str
    asset_issuer: str
    verified: bool
    verification_date: Optional[str] = None
