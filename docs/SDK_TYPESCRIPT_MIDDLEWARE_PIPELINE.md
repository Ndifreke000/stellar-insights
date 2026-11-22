# TypeScript SDK: Consolidate Retry, Dedup and Cancellation into a Middleware Pipeline

**GitHub Issue**: [#2840](https://github.com/Ndifreke000/stellar-insights/issues/2840)  
**Estimated size**: L — 1–2 weeks for someone familiar with the SDK transport layer.  
**Difficulty**: Hard.

---

## Overview

The SDK currently handles retry, deduplication, and cancellation as three independent wrappers that each wrap the same underlying fetch call. When a caller combines them — retry + cancel + dedup — the interactions are undefined: a cancelled request can still be retried, a deduped request can receive a stale `AbortSignal`, and two separate transport paths (`HttpClient` in `http.ts` and `ApiClient` in `api-client.ts`) each implement their own retry loop.

This change replaces that arrangement with a single composable middleware chain:

```
(req: Request, next: Handler) => Promise<Response>
```

Each concern — retry, deduplication, cancellation — becomes one middleware layer. A single transport sits at the bottom of the chain. The public API stays backward-compatible; callers opt into the new pipeline by default and existing call sites require no changes.

---

## Current State

```
sdk/typescript/src/
├── retry_policy.ts            ←  46 lines — constants, fullJitterDelay, parseRetryAfter
├── retry_with_backoff.ts      ← 171 lines — RetryWithBackoff class, duplicate retry loop
├── request_cancellation.ts    ←  73 lines — RequestCancellation class, per-key AbortController map
├── request_deduplication.ts   ← 101 lines — RequestDeduplication class, in-flight promise map
├── http.ts                    ← 170 lines — HttpClient, inline retry loop (second implementation)
└── api-client.ts              ← 488 lines — ApiClient, owns interceptors, calls HttpClient
```

The problems this layout causes:

| Symptom | Root cause |
|---|---|
| Cancelled request is still retried | `RequestCancellation` aborts the outer fetch, but `HttpClient.withRetry` catches the rejection and retries it unconditionally when `isNetworkError` is true |
| Deduped request shares the wrong signal | `RequestDeduplication.execute` calls the fetcher without forwarding any `AbortSignal`; the deduped subscriber gets the result but cannot cancel it |
| Two retry loops diverge | `HttpClient.withRetry` (lines 97–131 of `http.ts`) and `RetryWithBackoff.execute` (lines 44–87 of `retry_with_backoff.ts`) implement the same algorithm independently and are configured separately |
| Interceptors cannot see retry/dedup metadata | `api-client.ts` interceptors run before the retry loop; they cannot observe attempt counts or deduplication hits |

---

## Target Architecture

### Middleware type

```ts
// sdk/typescript/src/middleware/types.ts

export interface MiddlewareRequest {
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: unknown;
  signal?: AbortSignal;
  /** Set by the idempotency middleware; consumed by retry. */
  idempotent?: boolean;
  /** Opaque bag for middleware-to-middleware communication. */
  context: Record<string, unknown>;
}

export interface MiddlewareResponse<T = unknown> {
  status: number;
  headers: Record<string, string>;
  data: T;
  /** Number of attempts made, including the first. */
  attempts: number;
  /** True when this result was served from the dedup in-flight map. */
  deduplicated: boolean;
}

export type NextHandler<T = unknown> = (
  req: MiddlewareRequest,
) => Promise<MiddlewareResponse<T>>;

export type Middleware<T = unknown> = (
  req: MiddlewareRequest,
  next: NextHandler<T>,
) => Promise<MiddlewareResponse<T>>;
```

### Composed chain

Middleware layers execute outermost-first. The recommended stack, ordered from outermost to innermost, is:

```
CancellationMiddleware          ← attaches / propagates AbortSignal
    │
DeduplicationMiddleware         ← short-circuits for in-flight GETs
    │
RetryMiddleware                 ← retries on network errors / 429 / 5xx
    │
FetchTransport                  ← single fetch call
```

```ts
// sdk/typescript/src/middleware/pipeline.ts

import { compose } from './compose.js';
import { cancellationMiddleware } from './cancellation.js';
import { deduplicationMiddleware } from './deduplication.js';
import { retryMiddleware } from './retry.js';
import { fetchTransport } from './transport.js';

export function buildDefaultPipeline(config: PayRaiderConfig): NextHandler {
  return compose(
    cancellationMiddleware(config),
    deduplicationMiddleware(config),
    retryMiddleware(config),
    fetchTransport(config),
  );
}
```

`compose` reduces the array right-to-left so each layer receives the next layer as `next`:

```ts
// sdk/typescript/src/middleware/compose.ts

export function compose<T>(...layers: Middleware<T>[]): NextHandler<T> {
  return layers.reduceRight<NextHandler<T>>(
    (next, middleware) => (req) => middleware(req, next),
    () => { throw new Error('No transport at end of middleware chain'); },
  );
}
```

### Cancellation middleware

Owns a `Map<string, AbortController>`. When a request carries a `requestKey` in `context`, it registers a controller and merges the resulting signal with any caller-supplied signal via `AbortSignal.any`. Cancelling the key aborts the merged signal, which propagates through every downstream layer including the retry sleep.

```ts
// sdk/typescript/src/middleware/cancellation.ts

export function cancellationMiddleware(config: PayRaiderConfig): Middleware {
  const controllers = new Map<string, AbortController>();

  return async (req, next) => {
    const key = req.context.requestKey as string | undefined;
    if (!key) return next(req);

    const existing = controllers.get(key);
    existing?.abort();

    const controller = new AbortController();
    controllers.set(key, controller);

    // Merge caller signal with the cancellation signal.
    const signals = [controller.signal, req.signal].filter(Boolean) as AbortSignal[];
    const mergedSignal = signals.length > 1 ? AbortSignal.any(signals) : signals[0];

    try {
      return await next({ ...req, signal: mergedSignal });
    } finally {
      controllers.delete(key);
    }
  };
}
```

### Deduplication middleware

Maintains an in-flight `Map<string, Promise<MiddlewareResponse>>`. Only deduplicates GET requests (or requests where `context.dedup === true`). The deduplicated subscriber receives the same resolved value; because cancellation is upstream, a subscriber that cancels does not abort the shared in-flight request — it only stops waiting for the result locally.

```ts
// sdk/typescript/src/middleware/deduplication.ts

export function deduplicationMiddleware(_config: PayRaiderConfig): Middleware {
  const inFlight = new Map<string, Promise<MiddlewareResponse>>();

  return async (req, next) => {
    const shouldDedup =
      req.method === 'GET' || req.context.dedup === true;
    if (!shouldDedup) return next(req);

    const key = req.context.dedupKey as string ?? req.url;
    const existing = inFlight.get(key);
    if (existing) {
      const result = await existing;
      return { ...result, deduplicated: true };
    }

    const promise = next(req).finally(() => inFlight.delete(key));
    inFlight.set(key, promise);
    return promise;
  };
}
```

### Retry middleware

Replaces both `HttpClient.withRetry` and `RetryWithBackoff.execute`. Respects the `AbortSignal` already on the request: if the signal is aborted during the backoff sleep, the retry loop exits immediately without issuing another attempt.

```ts
// sdk/typescript/src/middleware/retry.ts

import { fullJitterDelay, isIdempotentMethod, parseRetryAfter, RETRYABLE_STATUSES } from '../retry_policy.js';

export function retryMiddleware(config: PayRaiderConfig): Middleware {
  const maxRetries = config.maxRetries ?? DEFAULT_MAX_RETRIES;
  const baseDelayMs = config.retryDelay ?? DEFAULT_BASE_DELAY_MS;
  const maxDelayMs = config.maxRetryDelay ?? DEFAULT_MAX_DELAY_MS;

  return async (req, next) => {
    const retryable =
      (req.idempotent ?? isIdempotentMethod(req.method)) ||
      req.context.idempotent === true;

    for (let attempt = 0; ; attempt++) {
      try {
        const res = await next({ ...req, context: { ...req.context, attempt } });
        return { ...res, attempts: attempt + 1 };
      } catch (error) {
        const aborted = req.signal?.aborted;
        if (aborted) throw error;                      // propagate cancellation immediately

        const canRetry = retryable && attempt < maxRetries && isRetryableError(error);
        if (!canRetry) throw error;

        const retryAfterMs =
          error instanceof PayRaiderError ? error.retryAfterMs : undefined;
        if (retryAfterMs !== undefined && retryAfterMs > maxDelayMs) throw error;

        const delayMs =
          retryAfterMs ?? fullJitterDelay(attempt, baseDelayMs, maxDelayMs);

        await sleepAbortable(delayMs, req.signal);
      }
    }
  };
}

function sleepAbortable(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) return reject(signal.reason);
    const timer = setTimeout(resolve, ms);
    signal?.addEventListener('abort', () => {
      clearTimeout(timer);
      reject(signal.reason);
    }, { once: true });
  });
}
```

### Fetch transport

The single fetch call. Issues one HTTP request per invocation; all retry/dedup/cancel logic lives upstream.

```ts
// sdk/typescript/src/middleware/transport.ts

export function fetchTransport(config: PayRaiderConfig): NextHandler {
  const timeout = config.timeout ?? DEFAULT_TIMEOUT;

  return async (req) => {
    const timeoutSignal = AbortSignal.timeout(timeout);
    const signals = [timeoutSignal, req.signal].filter(Boolean) as AbortSignal[];
    const signal = signals.length > 1 ? AbortSignal.any(signals) : signals[0];

    const res = await fetch(req.url, {
      method: req.method,
      headers: req.headers,
      body: req.body !== undefined ? JSON.stringify(req.body) : undefined,
      signal,
    });

    if (!res.ok) {
      const retryAfterMs = parseRetryAfter(res.headers.get('Retry-After'));
      return throwApiError(res, retryAfterMs);
    }

    const data = res.status === 204 ? undefined : await res.json();
    return { status: res.status, headers: headersToRecord(res.headers), data, attempts: 1, deduplicated: false };
  };
}
```

---

## Implementation Steps

### 1. Create the `middleware/` directory and type definitions

```bash
mkdir -p sdk/typescript/src/middleware
```

Create `types.ts`, `compose.ts` with the interfaces and `compose` function shown above. No existing files are changed at this step.

### 2. Implement each middleware layer

Create the four files: `cancellation.ts`, `deduplication.ts`, `retry.ts`, and `transport.ts`. Import constants from the existing `retry_policy.ts`; do not duplicate them.

Verify each layer compiles in isolation:

```bash
cd sdk/typescript
npx tsc --noEmit
```

### 3. Create `pipeline.ts` and wire `buildDefaultPipeline`

Compose the four layers in `pipeline.ts`. Add an export from `src/index.ts`:

```ts
// src/index.ts (addition only)
export { buildDefaultPipeline } from './middleware/pipeline.js';
export type { Middleware, MiddlewareRequest, MiddlewareResponse, NextHandler } from './middleware/types.js';
```

### 4. Update `HttpClient` to delegate to the pipeline

Replace `HttpClient.withRetry` with a call to the pipeline. Keep the `request()` signature identical.

```ts
// src/http.ts — updated HttpClient.request()
async request<T>(method: string, path: string, options: RequestOptions = {}): Promise<T> {
  const url = this.buildUrl(path, options.params);
  const headers = this.buildHeaders(options.headers);

  const req: MiddlewareRequest = {
    method,
    url,
    headers,
    body: options.body,
    context: {
      idempotent: options.idempotent,
    },
  };

  const res = await this.pipeline<T>(req);
  return res.data as T;
}
```

Delete the `withRetry` private method and the `sleep` helper from `http.ts`. The retry loop now lives exclusively in `RetryMiddleware`.

### 5. Update `ApiClient` to use the pipeline

`ApiClient` in `api-client.ts` currently calls `HttpClient.request`. That call site is unchanged. Remove the second copy of retry logic inside `ApiClient` if one exists, and move the interceptor chain so it wraps the pipeline rather than sitting inside it.

### 6. Deprecate `RetryWithBackoff` and `RequestCancellation` / `RequestDeduplication` classes

Add `@deprecated` JSDoc to each class pointing to the new middleware API:

```ts
/**
 * @deprecated Use `retryMiddleware` from `./middleware/retry.js` instead.
 * This class will be removed in the next major version.
 */
export class RetryWithBackoff { ... }
```

Do not delete the classes in this PR. Removal is a follow-on breaking-change release.

### 7. Update `sdk/typescript/README.md`

Add a "Middleware Pipeline" section describing the layer order, how to add a custom middleware, and the cancellation / deduplication APIs. Update the existing retry configuration table to reference the new middleware.

### 8. Verify the public API surface is unchanged

```bash
cd sdk/typescript
npm run build
npx attw --pack   # or api-extractor if already configured
```

No public type should be removed or renamed. New exports are additive.

---

## Acceptance Criteria

- [ ] A single `fetchTransport` is the only code that calls `fetch`. No other file issues HTTP requests.
- [ ] `HttpClient.withRetry` and the retry loop inside `RetryWithBackoff.execute` are removed (replaced by `RetryMiddleware`).
- [ ] A cancelled request (`AbortSignal` aborted) is never retried, even when it fails with a retryable status code.
- [ ] A deduped GET resolves to the same data as the in-flight request; the deduped subscriber's cancellation does not abort the shared request.
- [ ] All combination matrix tests pass (see Test Plan below).
- [ ] `npx tsc --noEmit` and `npm run lint` both exit 0.
- [ ] CI is green (lint, type-check, unit tests, integration tests).
- [ ] No new `TODO` comments without a linked issue number.

---

## Test Plan

Add `sdk/typescript/src/middleware/pipeline.test.ts` covering the following matrix. Each combination should be tested as an independent `it` block.

| # | Scenario | Expected outcome |
|---|---|---|
| 1 | Cancel during first attempt | Error propagates; zero retries attempted |
| 2 | Cancel during backoff sleep | Backoff sleep resolves to rejection immediately; no further attempt |
| 3 | Cancel during dedup wait | Subscriber rejects; in-flight request continues to completion |
| 4 | 429 with `Retry-After: 1` on idempotent request | Retried after ~1 s; second attempt succeeds |
| 5 | 429 with `Retry-After` exceeding `maxDelayMs` | Error surfaced immediately without retrying |
| 6 | Dedup of a request that is then retried | Both subscribers receive the successful result from the retry |
| 7 | Non-idempotent POST fails with 503 | Error surfaces after one attempt; no retry |
| 8 | Network error (TypeError) on idempotent GET | Retried up to `maxRetries` times |
| 9 | Dedup key collision across two concurrent callers | Only one fetch is issued; both callers receive the same response object |
| 10 | Custom middleware inserted into the chain | Custom layer executes in position order; `req.context` mutations visible to downstream layers |

Use `vi.fn()` / `jest.fn()` to mock `fetch` and `AbortSignal.timeout`. Assert the number of `fetch` calls, not just the return value.

---

## Design Notes

### Request identity and dedup keys

The deduplication key defaults to the request URL. For requests with query parameters, the URL already encodes them. Callers may override the key by setting `context.dedupKey` on the request.

### Idempotency

`RetryMiddleware` checks `req.idempotent ?? isIdempotentMethod(req.method)`. A caller can mark a POST safe to retry by setting `idempotent: true` in `RequestOptions`; `HttpClient.request` forwards this into `req.idempotent`. This matches the existing `options.idempotent` flag in `http.ts`.

### `AbortSignal.any` availability

`AbortSignal.any` is available in Node.js ≥ 20.3 and all evergreen browsers. For older environments, provide a ponyfill:

```ts
function anySignal(signals: AbortSignal[]): AbortSignal {
  if (typeof AbortSignal.any === 'function') return AbortSignal.any(signals);
  const controller = new AbortController();
  for (const s of signals) {
    if (s.aborted) { controller.abort(s.reason); break; }
    s.addEventListener('abort', () => controller.abort(s.reason), { once: true });
  }
  return controller.signal;
}
```

Place this in `sdk/typescript/src/middleware/signal_utils.ts`.

### Metrics

`RetryWithBackoff.getMetrics()` and `RequestDeduplication.getMetrics()` are part of the public API. The new middleware layers expose equivalent data via the `MiddlewareResponse` fields (`attempts`, `deduplicated`) on each individual response. A global `PipelineMetrics` accumulator can be added as a separate optional middleware layer if aggregate counters are needed; that work is out of scope for this issue.

### Failure modes

| Failure | Behaviour |
|---|---|
| `fetchTransport` throws `DOMException` with `name === 'AbortError'` | `RetryMiddleware` checks `req.signal?.aborted` before deciding to retry; aborted requests are not retried |
| `fetch` rejects with `TypeError` (network error) | `isRetryableError` returns true; retried on idempotent requests |
| Server sends malformed `Retry-After` | `parseRetryAfter` returns `undefined`; fallback to full-jitter delay |
| All retries exhausted | The last error is re-thrown unwrapped; callers receive the original `PayRaiderError` |

---

## Runtime Compatibility

The middleware chain uses `Promise`, `AbortController`, `AbortSignal`, and `fetch`, all of which are required by the existing `HttpClient`. No new browser or Node.js APIs are introduced beyond `AbortSignal.any` (ponyfilled above). The React Native compatibility surface in `src/react_native_compatibility.ts` is unaffected; `fetch` and `AbortController` are available in React Native's Hermes engine.

Verify after the change:

```bash
grep -r "withRetry\|new RetryWithBackoff\|new RequestCancellation\|new RequestDeduplication" src/
```

The only matches should be the deprecated class bodies themselves, not any new call sites.

---

## References

- [`sdk/typescript/src/retry_policy.ts`](../sdk/typescript/src/retry_policy.ts)
- [`sdk/typescript/src/retry_with_backoff.ts`](../sdk/typescript/src/retry_with_backoff.ts)
- [`sdk/typescript/src/request_cancellation.ts`](../sdk/typescript/src/request_cancellation.ts)
- [`sdk/typescript/src/request_deduplication.ts`](../sdk/typescript/src/request_deduplication.ts)
- [`sdk/typescript/src/http.ts`](../sdk/typescript/src/http.ts)
- [`sdk/typescript/src/api-client.ts`](../sdk/typescript/src/api-client.ts)
- [docs/SDK_TYPESCRIPT_SOURCE_LAYOUT.md](./SDK_TYPESCRIPT_SOURCE_LAYOUT.md)
- [MDN: AbortSignal.any()](https://developer.mozilla.org/en-US/docs/Web/API/AbortSignal/any_static)
- [AWS Architecture Blog: Exponential Backoff and Jitter](https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/)
