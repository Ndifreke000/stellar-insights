"""HTTP client with auth, retry, and rate-limit handling."""
from __future__ import annotations

import asyncio
import random
import time
from typing import Any, Optional

import httpx

DEFAULT_BASE_URL = "https://api.payraider.io"
DEFAULT_MAX_RETRIES = 3
DEFAULT_RETRY_DELAY = 0.5  # seconds
DEFAULT_TIMEOUT = 30.0  # seconds

RETRYABLE_STATUSES = {429, 500, 502, 503, 504}


class PayRaiderError(Exception):
    """Raised when the API returns an error response."""

    def __init__(self, status: int, code: str, message: str, request_id: Optional[str] = None) -> None:
        super().__init__(message)
        self.status = status
        self.code = code
        self.request_id = request_id

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
        timeout: float = DEFAULT_TIMEOUT,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._max_retries = max_retries
        self._retry_delay = retry_delay
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
        params: Optional[dict[str, Any]] = None,
        json: Optional[Any] = None,
    ) -> Any:
        url = f"{self._base_url}{path}"
        # Strip None values from params
        clean_params = {k: v for k, v in (params or {}).items() if v is not None}

        for attempt in range(self._max_retries + 1):
            response = await self._client.request(
                method,
                url,
                headers=self._headers(),
                params=clean_params or None,
                json=json,
            )

            if response.status_code in RETRYABLE_STATUSES and attempt < self._max_retries:
                delay = self._backoff(response, attempt)
                await asyncio.sleep(delay)
                continue

            if not response.is_success:
                self._raise_error(response)

            if response.status_code == 204:
                return None
            return response.json()

        # Should not reach here, but raise last error
        self._raise_error(response)  # type: ignore[possibly-undefined]

    def _backoff(self, response: httpx.Response, attempt: int) -> float:
        retry_after = response.headers.get("Retry-After")
        if retry_after and retry_after.isdigit():
            return float(retry_after)
        return self._retry_delay * (2**attempt) + random.uniform(0, 0.1)

    def _raise_error(self, response: httpx.Response) -> None:
        try:
            body = response.json()
        except Exception:
            body = {}
        raise PayRaiderError(
            status=response.status_code,
            code=body.get("error", "UNKNOWN_ERROR"),
            message=body.get("message", response.reason_phrase or "Unknown error"),
            request_id=body.get("request_id"),
        )

    async def aclose(self) -> None:
        await self._client.aclose()

    async def __aenter__(self) -> "HttpClient":
        return self

    async def __aexit__(self, *_: Any) -> None:
        await self.aclose()
