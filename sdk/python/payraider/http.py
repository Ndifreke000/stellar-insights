"""HTTP client with auth, retry, and rate-limit handling."""
from __future__ import annotations

import asyncio
from typing import Any, Dict, NoReturn, Optional

import httpx

from .retry import (
    DEFAULT_BASE_DELAY,
    DEFAULT_MAX_DELAY,
    DEFAULT_MAX_RETRIES,
    RETRYABLE_STATUSES,
    full_jitter_delay,
    is_idempotent_method,
    parse_retry_after,
)
from .types import ApiErrorBody

DEFAULT_BASE_URL = "https://api.payraider.io"
DEFAULT_RETRY_DELAY = DEFAULT_BASE_DELAY
DEFAULT_TIMEOUT = 30.0  # seconds


class PayRaiderError(Exception):
    """Raised when the API returns an error response."""

    def __init__(
        self,
        status: int,
        code: str,
        message: str,
        request_id: Optional[str] = None,
        *,
        details: Optional[Dict[str, Any]] = None,
        retry_after: Optional[float] = None,
    ) -> None:
        super().__init__(message)
        self.status = status
        self.code = code
        self.request_id = request_id
        self.details = details
        #: Parsed ``Retry-After`` in seconds, when the server sent one.
        self.retry_after = retry_after

    def __repr__(self) -> str:
        return f"PayRaiderError(status={self.status}, code={self.code!r}, message={str(self)!r})"


class HttpClient:
    def __init__(
        self,
        *,
        api_key: Optional[str] = None,
        access_token: Optional[str] = None,
        base_url: str = DEFAULT_BASE_URL,
        max_retries: int = DEFAULT_MAX_RETRIES,
        retry_delay: float = DEFAULT_RETRY_DELAY,
        max_retry_delay: float = DEFAULT_MAX_DELAY,
        timeout: float = DEFAULT_TIMEOUT,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._max_retries = max_retries
        self._retry_delay = retry_delay
        self._max_retry_delay = max_retry_delay
        self._token: Optional[str] = access_token or api_key
        self._client = httpx.AsyncClient(timeout=timeout)

    def set_token(self, token: str) -> None:
        self._token = token

    def _headers(self) -> dict[str, str]:
        headers = {"Accept": "application/json", "Content-Type": "application/json"}
        if self._token:
            headers["Authorization"] = f"Bearer {self._token}"
        return headers

    async def request(
        self,
        method: str,
        path: str,
        *,
        params: Optional[Dict[str, Any]] = None,
        json: Optional[Any] = None,
        idempotent: Optional[bool] = None,
    ) -> Any:
        """Send a request and return the decoded JSON body.

        ``idempotent=True`` allows retries for a method that is not idempotent by
        default (e.g. a read-only POST). See ``payraider.retry`` for the policy.
        """
        url = f"{self._base_url}{path}"
        # Strip None values from params
        clean_params = {k: v for k, v in (params or {}).items() if v is not None}
        retryable = is_idempotent_method(method) if idempotent is None else idempotent
        max_retries = self._max_retries if retryable else 0

        attempt = 0
        while True:
            try:
                response = await self._client.request(
                    method,
                    url,
                    headers=self._headers(),
                    params=clean_params or None,
                    json=json,
                )
            except httpx.TransportError:
                if attempt < max_retries:
                    await asyncio.sleep(full_jitter_delay(attempt, self._retry_delay, self._max_retry_delay))
                    attempt += 1
                    continue
                raise

            if response.is_success:
                if response.status_code == 204:
                    return None
                return response.json()

            retry_after = parse_retry_after(response.headers.get("Retry-After"))
            if (
                response.status_code in RETRYABLE_STATUSES
                and attempt < max_retries
                and (retry_after is None or retry_after <= self._max_retry_delay)
            ):
                delay = (
                    retry_after
                    if retry_after is not None
                    else full_jitter_delay(attempt, self._retry_delay, self._max_retry_delay)
                )
                await asyncio.sleep(delay)
                attempt += 1
                continue

            self._raise_error(response, retry_after)

    @staticmethod
    def _raise_error(response: httpx.Response, retry_after: Optional[float] = None) -> NoReturn:
        try:
            raw = response.json()
        except ValueError:
            raw = None
        body = ApiErrorBody.from_response_json(raw)
        raise PayRaiderError(
            status=response.status_code,
            code=body.code or "UNKNOWN_ERROR",
            message=body.message or response.reason_phrase or "Unknown error",
            request_id=body.request_id,
            details=body.details,
            retry_after=retry_after,
        )

    async def aclose(self) -> None:
        await self._client.aclose()

    async def __aenter__(self) -> "HttpClient":
        return self

    async def __aexit__(self, *_: Any) -> None:
        await self.aclose()
