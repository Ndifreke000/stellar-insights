"""Typed models for PayRaider API responses.

Preflight, corridor, pagination and error shapes are pydantic v2 models that
mirror docs/openapi.json and are validated when a response is received. The
remaining dataclasses describe other endpoints but are not yet validated; those
resource methods return the decoded JSON as-is.
"""
from __future__ import annotations

import warnings
from dataclasses import dataclass, field
from typing import Any, Dict, Generic, List, Literal, Optional, TypeVar

from pydantic import BaseModel, ConfigDict, Field

T = TypeVar("T")


class ApiModel(BaseModel):
    """Base for response models. Unknown fields are kept (forward compatible)."""

    model_config = ConfigDict(extra="allow", populate_by_name=True)

    def __getitem__(self, key: str) -> Any:
        """Deprecated dict-style access kept for code written against the old dict responses."""
        warnings.warn(
            f"{type(self).__name__}[{key!r}] is deprecated; use attribute access (.{key}) instead",
            DeprecationWarning,
            stacklevel=2,
        )
        try:
            return getattr(self, key)
        except AttributeError:
            raise KeyError(key) from None


# ── Pagination ───────────────────────────────────────────────────────────────


class PageMeta(ApiModel):
    limit: int
    has_next: bool
    has_prev: bool
    total: Optional[int] = None
    next_cursor: Optional[str] = None
    prev_cursor: Optional[str] = None
    offset: Optional[int] = None
    """Deprecated by the API in favour of ``next_cursor``/``prev_cursor``."""
    next_offset: Optional[int] = None
    prev_offset: Optional[int] = None


class PageLinks(ApiModel):
    self_link: str = Field(alias="self")
    next: Optional[str] = None
    prev: Optional[str] = None


class PaginatedResponse(ApiModel, Generic[T]):
    data: List[T]
    pagination: PageMeta
    links: Optional[PageLinks] = None


Pagination = PageMeta  # backwards-compatible name


# ── Corridors ────────────────────────────────────────────────────────────────


class Corridor(ApiModel):
    """One payment corridor (``CorridorResponse`` in the API spec)."""

    id: str
    source_asset: str
    destination_asset: str
    success_rate: float
    total_attempts: int
    successful_payments: int
    failed_payments: int
    average_latency_ms: float
    median_latency_ms: float
    p95_latency_ms: float
    p99_latency_ms: float
    liquidity_depth_usd: float
    liquidity_volume_24h_usd: float
    liquidity_trend: str
    health_score: float
    last_updated: str


CorridorResponse = Corridor


class SuccessRateDataPoint(ApiModel):
    timestamp: str
    success_rate: float
    attempts: int


class LatencyDataPoint(ApiModel):
    latency_bucket_ms: int
    count: int
    percentage: float


class LiquidityDataPoint(ApiModel):
    timestamp: str
    liquidity_usd: float
    volume_24h_usd: float


class CorridorDetail(ApiModel):
    corridor: Corridor
    historical_success_rate: List[SuccessRateDataPoint]
    latency_distribution: List[LatencyDataPoint]
    liquidity_trends: List[LiquidityDataPoint]
    related_corridors: Optional[List[Corridor]] = None


# ── Preflight ────────────────────────────────────────────────────────────────

PreflightDecision = Literal["proceed", "caution", "hold", "unknown"]
CheckStatus = Literal["pass", "warn", "fail"]


class PreflightCheck(ApiModel):
    name: str
    """``success_rate``, ``liquidity``, ``sample_size`` or ``health_score``."""
    status: CheckStatus
    detail: str


class PreflightResponse(ApiModel):
    decision: PreflightDecision
    summary: str
    checks: List[PreflightCheck]
    alternatives: List[Corridor]
    evaluated_at: str
    score: Optional[float] = None
    corridor: Optional[Corridor] = None


# ── Errors ───────────────────────────────────────────────────────────────────


class ApiErrorBody(ApiModel):
    """Error payload. The API sends ``{"error": {"code", "message", ...}}``."""

    code: Optional[str] = None
    message: Optional[str] = None
    request_id: Optional[str] = None
    details: Optional[Dict[str, Any]] = None

    @classmethod
    def from_response_json(cls, raw: Any) -> "ApiErrorBody":
        """Parse any error body leniently; never raises."""
        if not isinstance(raw, dict):
            return cls()
        nested = raw.get("error")
        if isinstance(nested, dict):
            source: Dict[str, Any] = nested
        else:
            # Flat legacy shape: {"error": "CODE", "message": "...", "request_id": "..."}
            source = {**raw, "code": nested if isinstance(nested, str) else raw.get("code")}
        return cls(
            code=_str_or_none(source.get("code")),
            message=_str_or_none(source.get("message")),
            request_id=_str_or_none(source.get("request_id")),
            details=source.get("details") if isinstance(source.get("details"), dict) else None,
        )


def _str_or_none(value: Any) -> Optional[str]:
    return value if isinstance(value, str) else None


# ── Other endpoints (not validated yet) ──────────────────────────────────────


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
