/**
 * Retry semantics shared by every client in this SDK (and mirrored in
 * sdk/python/payraider/retry.py):
 *
 * - Retry only on network errors and HTTP 429, 502, 503, 504.
 * - Retry only idempotent methods (GET, HEAD, OPTIONS, PUT, DELETE) unless the
 *   caller marks the request idempotent. POST/PATCH are never retried by default.
 * - At most `maxRetries` retries (default 3) after the first attempt.
 * - Delay is the server's `Retry-After` (delta-seconds or HTTP-date) when present,
 *   otherwise full jitter: random(0, min(maxDelayMs, baseDelayMs * 2^attempt)).
 * - A `Retry-After` longer than `maxDelayMs` is not waited out; the error is
 *   returned to the caller instead.
 */

export const DEFAULT_MAX_RETRIES = 3;
export const DEFAULT_BASE_DELAY_MS = 500;
export const DEFAULT_MAX_DELAY_MS = 30_000;
export const RETRYABLE_STATUSES: ReadonlySet<number> = new Set([429, 502, 503, 504]);
export const IDEMPOTENT_METHODS: ReadonlySet<string> = new Set(["GET", "HEAD", "OPTIONS", "PUT", "DELETE"]);

export function isIdempotentMethod(method: string): boolean {
  return IDEMPOTENT_METHODS.has(method.toUpperCase());
}

/** Full-jitter backoff delay in ms for the given zero-based retry attempt. */
export function fullJitterDelay(
  attempt: number,
  baseDelayMs = DEFAULT_BASE_DELAY_MS,
  maxDelayMs = DEFAULT_MAX_DELAY_MS,
): number {
  const ceiling = Math.min(maxDelayMs, baseDelayMs * 2 ** attempt);
  return Math.floor(Math.random() * ceiling);
}

/**
 * Parse a `Retry-After` header value into milliseconds. Accepts delta-seconds
 * or an HTTP-date. Returns undefined when absent or unparseable.
 */
export function parseRetryAfter(value: string | null | undefined, now = Date.now()): number | undefined {
  if (!value) return undefined;
  const trimmed = value.trim();
  if (/^\d+$/.test(trimmed)) return Number(trimmed) * 1000;
  const date = Date.parse(trimmed);
  if (Number.isNaN(date)) return undefined;
  return Math.max(0, date - now);
}
