# Python SDK: Sync Client, Streaming Pagination, and Typed Models Generated from OpenAPI

**GitHub Issue**: [#2842](https://github.com/Ndifreke000/stellar-insights/issues/2842)  
**Difficulty**: Hard  
**Estimated size**: L — 1–2 weeks for someone familiar with the stack.

---

## Overview

The Python SDK (`sdk/python/payraider/`) is async-only. All resource methods are `async def`; the HTTP client wraps `httpx.AsyncClient` exclusively. Types in `types.py` are hand-written: a mix of pydantic `BaseModel` subclasses for validated endpoints (corridors, preflight, pagination) and unvalidated `@dataclass` stubs for the rest. Neither the pydantic models nor the dataclasses are generated from `docs/openapi.json`, so they drift from the actual API schema over time.

Data scientists working in Jupyter notebooks, data pipelines, and scripts written in synchronous Python cannot use the SDK without wrapping every call in `asyncio.run(...)`. That friction blocks adoption.

This change delivers three things:

1. **Sync client** — a `SyncPayRaiderClient` that shares all retry, auth, and error-handling logic with the existing async client, without duplicating it.
2. **Generated pydantic models** — replace hand-written types with models generated from `docs/openapi.json`, keeping the existing `ApiModel` base so the public attribute API is unchanged.
3. **Auto-paginating iterators** — `iter_pages()` / `aiter_pages()` on paginated resources, so callers never manage cursors manually.

An optional `pandas` extra adds `.to_dataframe()` helpers to paginated results.

---

## Current State

```
sdk/python/
├── payraider/
│   ├── __init__.py
│   ├── http.py           # 160 lines — async-only HttpClient backed by httpx.AsyncClient
│   ├── resources.py      # 297 lines — async resource classes, all methods are async def
│   ├── retry.py          # retry policy (pure functions, no async dependency)
│   ├── types.py          # 342 lines — hand-written pydantic models + @dataclass stubs
│   └── py.typed
├── tests/
├── pyproject.toml
└── README.md
```

### Problems

| File | Problem |
|---|---|
| `http.py` | `HttpClient._client` is always `httpx.AsyncClient`; no path for synchronous callers |
| `resources.py` | Every method is `async def`; unusable in notebooks without `asyncio.run(...)` |
| `types.py` | Hand-written and partially unvalidated (28 `@dataclass` stubs return raw `Any` from resource methods); drifts from `docs/openapi.json` |

---

## Target Architecture

```
sdk/python/
├── payraider/
│   ├── __init__.py                # exports SyncPayRaiderClient, AsyncPayRaiderClient
│   ├── _transport.py              # NEW: shared sync/async transport abstraction
│   ├── _client_async.py           # renamed from http.py; thin async wrapper
│   ├── _client_sync.py            # NEW: thin sync wrapper
│   ├── resources.py               # unchanged resource logic; dispatches through transport
│   ├── retry.py                   # unchanged (pure functions)
│   ├── types.py                   # GENERATED from openapi.json; do not edit manually
│   ├── _types_base.py             # NEW: ApiModel base + hand-written helpers that generation cannot produce
│   └── py.typed
├── scripts/
│   └── generate_types.py          # NEW: OpenAPI → pydantic generation script
├── tests/
│   ├── test_corridors.py
│   ├── test_pagination.py
│   ├── test_sync_parity.py        # NEW: sync/async parity tests
│   └── ...
├── pyproject.toml                 # adds sync and pandas extras
└── README.md
```

### Sync/Async Transport Layer

The core insight is that retry logic, header construction, and error parsing do not depend on whether the underlying I/O is sync or async. A `Transport` protocol captures the one thing that differs — sending the HTTP request — and both clients implement it.

```
┌──────────────────────────────────────────┐
│            Resource classes              │
│  (corridors, preflight, anchors, …)      │
│  call self._transport.request(...)       │
└───────────────┬──────────────────────────┘
                │
        ┌───────┴────────┐
        │   Transport    │  (Protocol)
        │  .request()    │
        └───────┬────────┘
       ┌────────┴─────────┐
       │                  │
 AsyncTransport      SyncTransport
 (httpx.AsyncClient) (httpx.Client)
       │                  │
 AsyncPayRaiderClient  SyncPayRaiderClient
```

Retry delay is `await asyncio.sleep(...)` for async, `time.sleep(...)` for sync. Everything else — jitter calculation, `Retry-After` parsing, error body parsing — is shared in `retry.py` and `_transport.py`.

### Pydantic Model Generation

`scripts/generate_types.py` reads `docs/openapi.json` and emits `payraider/types.py`. The generator:

- Maps OpenAPI `components/schemas` to `class Name(ApiModel)` declarations.
- Uses `ApiModel` from `_types_base.py` as the base class (preserving `extra="allow"`, `populate_by_name=True`, and the deprecated `__getitem__` shim).
- Emits `Optional[T]` for fields that are not in the schema's `required` list.
- Preserves field aliases where the API uses camelCase keys that differ from the Python attribute name.
- Writes a `# DO NOT EDIT — generated by scripts/generate_types.py` header.

Models that cannot be generated (e.g. `ApiErrorBody.from_response_json` with its custom parsing logic) stay in `_types_base.py` and are re-exported from `types.py`.

### Pagination Iterator Design

Each paginated resource method grows two iterator companions:

- `iter_pages(**kwargs)` — synchronous generator, yields one page (`PaginatedResponse[T]`) at a time.
- `aiter_pages(**kwargs)` — async generator, same contract.

```python
# Sync
for page in client.corridors.iter_pages(limit=50):
    process(page.data)

# Async
async for page in client.corridors.aiter_pages(limit=50):
    await process(page.data)
```

Both iterators follow the cursor returned by `pagination.next_cursor`. They stop when `pagination.has_next` is `False`. The first page request uses any `cursor` / `page` kwarg the caller supplies, allowing resume from a checkpoint.

---

## Implementation Steps

### 1. Split the transport layer out of `http.py`

Create `payraider/_transport.py` with a `Protocol` and shared helpers:

```python
# payraider/_transport.py
from __future__ import annotations

import asyncio
import time
from typing import Any, Dict, Optional, Protocol, runtime_checkable

import httpx

from .retry import (
    full_jitter_delay, is_idempotent_method, parse_retry_after, RETRYABLE_STATUSES,
    DEFAULT_BASE_DELAY, DEFAULT_MAX_DELAY, DEFAULT_MAX_RETRIES,
)
from ._types_base import ApiErrorBody
from .http import PayRaiderError  # re-export; name stays stable


def _build_headers(token: Optional[str]) -> dict[str, str]:
    headers = {"Accept": "application/json", "Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


def _raise_error(response: httpx.Response, retry_after: Optional[float] = None) -> None:
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
```

### 2. Add `SyncTransport` in `_client_sync.py`

```python
# payraider/_client_sync.py
from __future__ import annotations

import time
from typing import Any, Dict, Optional

import httpx

from ._transport import _build_headers, _raise_error
from .retry import (
    full_jitter_delay, is_idempotent_method, parse_retry_after,
    RETRYABLE_STATUSES, DEFAULT_BASE_DELAY, DEFAULT_MAX_DELAY, DEFAULT_MAX_RETRIES,
)

DEFAULT_BASE_URL = "https://api.payraider.io"
DEFAULT_TIMEOUT = 30.0


class SyncHttpClient:
    """Synchronous HTTP client. Drop-in for HttpClient in resource classes."""

    def __init__(
        self,
        *,
        api_key: Optional[str] = None,
        access_token: Optional[str] = None,
        base_url: str = DEFAULT_BASE_URL,
        max_retries: int = DEFAULT_MAX_RETRIES,
        retry_delay: float = DEFAULT_BASE_DELAY,
        max_retry_delay: float = DEFAULT_MAX_DELAY,
        timeout: float = DEFAULT_TIMEOUT,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._max_retries = max_retries
        self._retry_delay = retry_delay
        self._max_retry_delay = max_retry_delay
        self._token: Optional[str] = access_token or api_key
        self._client = httpx.Client(timeout=timeout)

    def set_token(self, token: str) -> None:
        self._token = token

    def request(
        self,
        method: str,
        path: str,
        *,
        params: Optional[Dict[str, Any]] = None,
        json: Optional[Any] = None,
        idempotent: Optional[bool] = None,
    ) -> Any:
        url = f"{self._base_url}{path}"
        clean_params = {k: v for k, v in (params or {}).items() if v is not None}
        retryable = is_idempotent_method(method) if idempotent is None else idempotent
        max_retries = self._max_retries if retryable else 0

        attempt = 0
        while True:
            try:
                response = self._client.request(
                    method, url,
                    headers=_build_headers(self._token),
                    params=clean_params or None,
                    json=json,
                )
            except httpx.TransportError:
                if attempt < max_retries:
                    time.sleep(full_jitter_delay(attempt, self._retry_delay, self._max_retry_delay))
                    attempt += 1
                    continue
                raise

            if response.is_success:
                return None if response.status_code == 204 else response.json()

            retry_after = parse_retry_after(response.headers.get("Retry-After"))
            if (
                response.status_code in RETRYABLE_STATUSES
                and attempt < max_retries
                and (retry_after is None or retry_after <= self._max_retry_delay)
            ):
                delay = retry_after if retry_after is not None else full_jitter_delay(
                    attempt, self._retry_delay, self._max_retry_delay
                )
                time.sleep(delay)
                attempt += 1
                continue

            _raise_error(response, retry_after)

    def close(self) -> None:
        self._client.close()

    def __enter__(self) -> "SyncHttpClient":
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()
```

### 3. Add sync resource wrappers

Resource classes in `resources.py` keep their `async def` methods. Each resource gains a `Sync` subclass (or a thin mixin) that wraps them:

```python
# payraider/resources.py (addition)
from ._client_sync import SyncHttpClient


class SyncCorridorsResource:
    """Synchronous counterpart to CorridorsResource."""

    def __init__(self, http: SyncHttpClient) -> None:
        self._http = http

    def list(
        self,
        *,
        page: Optional[int] = None,
        limit: Optional[int] = None,
        cursor: Optional[str] = None,
    ) -> PaginatedResponse[Corridor]:
        raw = self._http.request(
            "GET", "/api/corridors",
            params={"page": page, "limit": limit, "cursor": cursor},
        )
        return PaginatedResponse[Corridor].model_validate(raw)

    def iter_pages(
        self,
        *,
        limit: Optional[int] = None,
        cursor: Optional[str] = None,
    ):
        """Yield every page of corridors, following next_cursor automatically."""
        while True:
            page = self.list(limit=limit, cursor=cursor)
            yield page
            if not page.pagination.has_next or not page.pagination.next_cursor:
                break
            cursor = page.pagination.next_cursor

    def get(self, source_or_key: str, destination: Optional[str] = None) -> CorridorDetail:
        key = source_or_key if destination is None else f"{source_or_key}->{destination}"
        raw = self._http.request("GET", f"/api/corridors/{_enc(key)}")
        return CorridorDetail.model_validate(raw)
```

The async `CorridorsResource` gains the same `aiter_pages` async generator:

```python
    async def aiter_pages(
        self,
        *,
        limit: Optional[int] = None,
        cursor: Optional[str] = None,
    ):
        while True:
            page = await self.list(limit=limit, cursor=cursor)
            yield page
            if not page.pagination.has_next or not page.pagination.next_cursor:
                break
            cursor = page.pagination.next_cursor
```

### 4. Add top-level client classes

```python
# payraider/__init__.py (additions)
from ._client_sync import SyncHttpClient
from .resources import (
    SyncCorridorsResource, SyncPreflightResource,
    SyncAnchorsResource, SyncPricesResource,
    # … all sync resources
)


class SyncPayRaiderClient:
    """Synchronous client. Use in notebooks, scripts, and data pipelines."""

    def __init__(self, **kwargs: Any) -> None:
        self._http = SyncHttpClient(**kwargs)
        self.corridors = SyncCorridorsResource(self._http)
        self.preflight = SyncPreflightResource(self._http)
        self.anchors = SyncAnchorsResource(self._http)
        self.prices = SyncPricesResource(self._http)
        # … remaining resources

    def close(self) -> None:
        self._http.close()

    def __enter__(self) -> "SyncPayRaiderClient":
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()
```

Usage in a notebook or script:

```python
from payraider import SyncPayRaiderClient

with SyncPayRaiderClient(api_key="pr_live_...") as client:
    # Single page
    page = client.corridors.list(limit=25)
    print(page.data[0].health_score)

    # All pages without cursor management
    corridors = [c for page in client.corridors.iter_pages(limit=100) for c in page.data]

    # Preflight check
    result = client.preflight.check("USDC:GA5ZSE...", "EURC:GAKALK...")
    if result.decision == "proceed":
        submit_payment()
```

The existing async client is unchanged:

```python
from payraider import AsyncPayRaiderClient  # renamed from PayRaiderClient; old name kept as alias

async with AsyncPayRaiderClient(api_key="pr_live_...") as client:
    async for page in client.corridors.aiter_pages(limit=100):
        await process(page.data)
```

### 5. Generate pydantic models from OpenAPI

Add `scripts/generate_types.py`. The script is invoked at development time and in CI — the generated file is committed so users do not need to run it:

```bash
python scripts/generate_types.py docs/openapi.json > sdk/python/payraider/types.py
```

Generated output looks like:

```python
# DO NOT EDIT — generated by scripts/generate_types.py from docs/openapi.json
# To regenerate: python scripts/generate_types.py docs/openapi.json > payraider/types.py
from __future__ import annotations

from typing import List, Literal, Optional
from pydantic import Field
from ._types_base import ApiModel


class Corridor(ApiModel):
    """One payment corridor (CorridorResponse in the API spec)."""

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


class PreflightCheck(ApiModel):
    name: str
    status: Literal["pass", "warn", "fail"]
    detail: str


class PreflightResponse(ApiModel):
    decision: Literal["proceed", "caution", "hold", "unknown"]
    summary: str
    checks: List[PreflightCheck]
    alternatives: List[Corridor]
    evaluated_at: str
    score: Optional[float] = None
    corridor: Optional[Corridor] = None
```

The generator maps OpenAPI `$ref` to Python class names, `nullable: true` to `Optional[T]`, arrays to `List[T]`, and handles `allOf` inheritance. Any schema that the generator cannot map cleanly emits a `TODO` comment and falls back to `Any`, and the CI check catches new unresolved `TODO`s without a linked issue (see acceptance criteria).

### 6. Add pandas helpers (optional extra)

Install with `pip install payraider[pandas]`. The extra adds a `.to_dataframe()` method on `PaginatedResponse[T]` and a `fetch_all_pages()` convenience function:

```python
# payraider/_pandas.py
from __future__ import annotations

from typing import TYPE_CHECKING, Callable, Iterator, TypeVar

if TYPE_CHECKING:
    import pandas as pd

from .types import PaginatedResponse

T = TypeVar("T")


def to_dataframe(page: "PaginatedResponse[T]") -> "pd.DataFrame":
    """Convert one page of results to a pandas DataFrame."""
    try:
        import pandas as pd
    except ImportError as e:
        raise ImportError(
            "Install the pandas extra: pip install payraider[pandas]"
        ) from e
    return pd.DataFrame([item.model_dump() for item in page.data])


def pages_to_dataframe(pages: "Iterator[PaginatedResponse[T]]") -> "pd.DataFrame":
    """Concatenate all pages into one DataFrame."""
    import pandas as pd
    return pd.concat([to_dataframe(p) for p in pages], ignore_index=True)
```

Usage:

```python
from payraider import SyncPayRaiderClient
from payraider._pandas import pages_to_dataframe

with SyncPayRaiderClient(api_key="pr_live_...") as client:
    df = pages_to_dataframe(client.corridors.iter_pages(limit=200))

df[df["health_score"] > 0.9].sort_values("liquidity_depth_usd", ascending=False)
```

`pyproject.toml` optional extras:

```toml
[project.optional-dependencies]
pandas = ["pandas>=1.5"]
dev = [
    "pytest==9.1.1",
    "pytest-asyncio==1.4.0",
    "respx==0.23.1",
    "mypy==2.4.0",
    "pandas>=1.5",
]
```

### 7. Update `pyproject.toml`

Add a `sync` optional extra for users who want to be explicit:

```toml
[project.optional-dependencies]
sync = []          # httpx already ships a sync client; no extra deps needed
pandas = ["pandas>=1.5"]
```

Add mypy overrides for the generated file so strict mode does not flag the `# type: ignore` comments the generator may emit:

```toml
[[tool.mypy.overrides]]
module = "payraider.types"
# Generated file — type: ignore comments are intentional
warn_unused_ignores = false
```

### 8. Update docs and README

Add a note to `sdk/python/README.md` covering both clients, `iter_pages`, and the pandas helper. Update the repository-level `docs/INTEGRATION_GUIDE.md` to reference the sync client for notebook usage.

---

## Acceptance Criteria

- [ ] `SyncPayRaiderClient` is importable from the top-level `payraider` package and works without `asyncio`.
- [ ] Every resource available on `AsyncPayRaiderClient` is available on `SyncPayRaiderClient` with the same method signatures (minus `async`/`await`).
- [ ] `iter_pages()` exists on every paginated sync resource; `aiter_pages()` exists on every paginated async resource.
- [ ] `payraider/types.py` is generated from `docs/openapi.json`; the generation script is in `scripts/generate_types.py`.
- [ ] The generated `types.py` passes `mypy --strict` (with the override for generated `warn_unused_ignores`).
- [ ] `pip install payraider[pandas]` succeeds; `.to_dataframe()` and `pages_to_dataframe()` are importable.
- [ ] `pytest` passes (all existing async tests continue to pass).
- [ ] Sync/async parity tests pass (see test plan below).
- [ ] CI is green: lint, type-check (`mypy --strict`), tests.
- [ ] No new `TODO` comments in generated or hand-written code without a linked issue number.
- [ ] `AsyncPayRaiderClient` is exposed as the new canonical name; `PayRaiderClient` remains as a backwards-compatible alias with a `DeprecationWarning`.

---

## Test Plan

### Sync/async parity tests (`tests/test_sync_parity.py`)

For each resource and method pair, one test fixture runs both the sync and async paths against a `respx` mock and asserts identical return values:

```python
import pytest
import respx
import httpx
from payraider import SyncPayRaiderClient, AsyncPayRaiderClient

CORRIDOR_PAYLOAD = {
    "data": [{"id": "USDC-EURC", "source_asset": "USDC", ...}],
    "pagination": {"limit": 10, "has_next": False, "has_prev": False},
}


@respx.mock
def test_corridors_list_sync():
    respx.get("https://api.payraider.io/api/corridors").mock(
        return_value=httpx.Response(200, json=CORRIDOR_PAYLOAD)
    )
    with SyncPayRaiderClient(api_key="test") as client:
        page = client.corridors.list(limit=10)
    assert page.data[0].id == "USDC-EURC"
    assert page.pagination.has_next is False


@pytest.mark.asyncio
@respx.mock
async def test_corridors_list_async():
    respx.get("https://api.payraider.io/api/corridors").mock(
        return_value=httpx.Response(200, json=CORRIDOR_PAYLOAD)
    )
    async with AsyncPayRaiderClient(api_key="test") as client:
        page = await client.corridors.list(limit=10)
    assert page.data[0].id == "USDC-EURC"
    assert page.pagination.has_next is False
```

### Pagination iterator tests (`tests/test_pagination.py`)

Verify that `iter_pages` / `aiter_pages` stops at the last page and correctly threads `next_cursor`:

```python
@respx.mock
def test_iter_pages_follows_cursor():
    respx.get("https://api.payraider.io/api/corridors", params={"cursor": None}).mock(
        return_value=httpx.Response(200, json={
            "data": [{"id": "A", ...}],
            "pagination": {"limit": 1, "has_next": True, "has_prev": False, "next_cursor": "tok1"},
        })
    )
    respx.get("https://api.payraider.io/api/corridors", params={"cursor": "tok1"}).mock(
        return_value=httpx.Response(200, json={
            "data": [{"id": "B", ...}],
            "pagination": {"limit": 1, "has_next": False, "has_prev": True},
        })
    )
    with SyncPayRaiderClient(api_key="test") as client:
        pages = list(client.corridors.iter_pages(limit=1))
    assert [p.data[0].id for p in pages] == ["A", "B"]
```

### mypy strict check

```bash
mypy --strict sdk/python/payraider/
```

Must produce zero errors. Run as part of the CI `lint` job.

### Generated model regression test

A CI step diffs the committed `types.py` against a fresh generation run. Any diff fails the build, ensuring the committed file stays in sync with `docs/openapi.json`:

```bash
python scripts/generate_types.py docs/openapi.json > /tmp/types_generated.py
diff sdk/python/payraider/types.py /tmp/types_generated.py
```

---

## Design Notes

### Shared transport layer

The key constraint is that retry and error logic must not be duplicated. `_transport.py` holds all logic that is pure (no I/O): header construction, `_raise_error`, the retry decision (`should_retry`), and jitter calculation. Both `SyncHttpClient` and `HttpClient` import from `_transport.py`. The only code that differs between them is the `time.sleep` vs `asyncio.sleep` call inside the retry loop, and the `httpx.Client` vs `httpx.AsyncClient` construction.

Avoid a base class with abstract async methods — Python's `asyncio` does not compose well with inheritance when one branch is sync and the other is async. Protocol-based dispatch is cleaner and easier to type-check with mypy strict.

### Why not `anyio` or `trio`?

`anyio` would let us write one coroutine-based implementation that runs under any async backend, but it does not give us a synchronous API. Data scientists working in notebooks want `result = client.corridors.list()`, not `anyio.from_thread.run_sync(...)`. The explicit sync transport is the right answer for the target audience.

### OpenAPI generator choice

Rather than adopting a heavy third-party generator (openapi-generator, datamodel-codegen), `scripts/generate_types.py` is a ~200-line script that understands exactly the subset of OpenAPI 3.1 that `docs/openapi.json` uses. This keeps the dependency surface small and makes the generation logic auditable. If the spec grows to use features the script cannot handle, the CI diff check surfaces it immediately.

### Pandas helper placement

`_pandas.py` is imported lazily — only when the caller calls `to_dataframe()` or `pages_to_dataframe()`. The `pandas` import inside each function means a user who installs `payraider` without the `[pandas]` extra gets a clear `ImportError` with an install hint rather than an `ImportError` at module import time.

### Backwards compatibility

- `PayRaiderClient` stays as a re-export of `AsyncPayRaiderClient` with a `DeprecationWarning`. It is removed in a future major version.
- `types.py` exports the same names it does today (`Corridor`, `CorridorResponse`, `PreflightResponse`, `PageMeta`, `Pagination`, etc.). Generated names come from the schema `title` or operation `operationId`; the generator normalises them to match existing names where there is a one-to-one correspondence.
- `ApiErrorBody.from_response_json` stays in `_types_base.py` and is re-exported from `types.py` unchanged.

---

## References

- [docs/openapi.json](../docs/openapi.json) — OpenAPI 3.1 source of truth for model generation
- [sdk/python/payraider/http.py](../sdk/python/payraider/http.py) — current async HTTP client (160 lines)
- [sdk/python/payraider/resources.py](../sdk/python/payraider/resources.py) — current async resource classes (297 lines)
- [sdk/python/payraider/types.py](../sdk/python/payraider/types.py) — current hand-written types (342 lines)
- [sdk/python/payraider/retry.py](../sdk/python/payraider/retry.py) — retry policy (shared, unchanged)
- [sdk/python/pyproject.toml](../sdk/python/pyproject.toml) — build config and optional extras
- [docs/API_PAGINATION.md](./API_PAGINATION.md) — pagination contract this SDK must implement
- [docs/INTEGRATION_GUIDE.md](./INTEGRATION_GUIDE.md) — end-user integration docs to update
- [httpx sync client documentation](https://www.python-httpx.org/async/#opening-and-closing-clients)
- [pydantic v2 model generation](https://docs.pydantic.dev/latest/concepts/pydantic_dataclasses/)
