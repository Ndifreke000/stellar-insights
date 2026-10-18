"""Retry semantics shared with the TypeScript SDK (sdk/typescript/src/retry_policy.ts).

- Retry only on network errors and HTTP 429, 502, 503, 504.
- Retry only idempotent methods (GET, HEAD, OPTIONS, PUT, DELETE) unless the
  caller marks the request idempotent. POST/PATCH are never retried by default.
- At most ``max_retries`` retries (default 3) after the first attempt.
- Delay is the server's ``Retry-After`` (delta-seconds or HTTP-date) when present,
  otherwise full jitter: ``uniform(0, min(max_delay, base_delay * 2**attempt))``.
- A ``Retry-After`` longer than ``max_delay`` is not waited out; the error is
  raised to the caller instead.
"""
from __future__ import annotations

import random
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from typing import Optional

DEFAULT_MAX_RETRIES = 3
DEFAULT_BASE_DELAY = 0.5  # seconds
DEFAULT_MAX_DELAY = 30.0  # seconds
RETRYABLE_STATUSES = frozenset({429, 502, 503, 504})
IDEMPOTENT_METHODS = frozenset({"GET", "HEAD", "OPTIONS", "PUT", "DELETE"})


def is_idempotent_method(method: str) -> bool:
    return method.upper() in IDEMPOTENT_METHODS


def full_jitter_delay(
    attempt: int,
    base_delay: float = DEFAULT_BASE_DELAY,
    max_delay: float = DEFAULT_MAX_DELAY,
) -> float:
    """Full-jitter backoff delay in seconds for the given zero-based retry attempt."""
    return random.uniform(0, min(max_delay, base_delay * (2**attempt)))


def parse_retry_after(value: Optional[str], now: Optional[datetime] = None) -> Optional[float]:
    """Parse a ``Retry-After`` header into seconds; None when absent or unparseable."""
    if not value:
        return None
    value = value.strip()
    if value.isdigit():
        return float(value)
    try:
        when = parsedate_to_datetime(value)
    except (TypeError, ValueError):
        return None
    if when.tzinfo is None:
        when = when.replace(tzinfo=timezone.utc)
    now = now or datetime.now(timezone.utc)
    return max(0.0, (when - now).total_seconds())
