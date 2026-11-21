# SDK Conformance Suite: Shared Cases for TypeScript, Python and MCP Server

**GitHub Issue**: [#2841](https://github.com/Ndifreke000/stellar-insights/issues/2841)  
**Estimated size**: L — roughly one to two weeks for someone familiar with the SDK stack.

---

## Overview

Three separate SDK implementations call the same Stellar Insights API:

| SDK | Language | Test runner |
|---|---|---|
| `sdk/typescript` | TypeScript / Node.js | vitest (`src/conformance.test.ts`) |
| `sdk/python` | Python 3 | pytest (`tests/test_conformance.py`) |
| `sdk/mcp-server` | TypeScript / Node.js | vitest (`tests/`) |

A shared set of JSON case files in `sdk/conformance/cases/` already drives the TypeScript and Python runners. The MCP server has no conformance runner at all, and the existing 13 cases cover only the happy-path wire format. Error responses, retry behaviour, pagination edge cases, rate-limit back-off, and auth-token refresh are untested cross-SDK, so the three implementations can and do drift apart without any CI signal.

This issue expands the case set, adds a conformance runner to the MCP server, and wires all three runners into a CI matrix so a single case failure blocks every SDK at once.

---

## Current State

```
sdk/conformance/
├── README.md                                   ← format spec + operations table
└── cases/                                      ← 13 JSON cases, happy-path only
    ├── anchors_get_raises_not_found.json
    ├── anchors_list_sends_pagination_params.json
    ├── corridors_get_percent_encodes_the_key.json
    ├── network_info_returns_body.json
    ├── preflight_check_batch_raises_on_empty_batch.json
    ├── preflight_check_batch_returns_per_item_results.json
    ├── preflight_check_omits_unset_fields.json
    ├── preflight_check_raises_api_error_on_400.json
    ├── preflight_check_returns_decision.json
    ├── preflight_check_sends_api_key_as_bearer.json
    ├── prices_batch_joins_assets_with_commas.json
    ├── prices_get_sends_asset_as_query.json
    └── webhooks_delete_returns_nothing_on_204.json

sdk/typescript/src/conformance.test.ts          ← 127 lines, runs all 13 cases
sdk/python/tests/test_conformance.py            ← runs all 13 cases
sdk/mcp-server/tests/                           ← no conformance runner
```

**What is missing:**

- Cases for retry semantics (transient 5xx, exponential back-off, `maxRetries` cap).
- Cases for pagination (auto-fetching subsequent pages, stopping at the last page).
- Cases for rate-limit handling (429 + `Retry-After`, back-off, eventual success).
- Cases for auth token refresh (401 → re-fetch token → retry original request).
- A conformance runner for the MCP server.
- A CI matrix that runs all three runners together and fails the build if any one of them does not pass all cases.

---

## Target State

```
sdk/conformance/
├── README.md                                   ← updated: new operations + extended format
└── cases/                                      ← 13 existing + ~20 new cases
    ├── ...existing cases...
    ├── anchors_list_paginates_to_next_page.json
    ├── anchors_list_stops_at_last_page.json
    ├── preflight_check_retries_on_503.json
    ├── preflight_check_gives_up_after_max_retries.json
    ├── preflight_check_backs_off_on_429.json
    ├── preflight_check_respects_retry_after_header.json
    ├── prices_get_refreshes_token_on_401.json
    └── ...

sdk/typescript/src/conformance.test.ts          ← extended: multi-response sequences
sdk/python/tests/test_conformance.py            ← extended: multi-response sequences
sdk/mcp-server/tests/conformance.test.ts        ← new: MCP tool → SDK call → HTTP chain

.github/workflows/sdk-conformance.yml           ← new: CI matrix across all three SDKs
```

The case format gains an optional `responses` array (a sequence of mocked HTTP responses) alongside the existing single `response` field, to drive retry and auth-refresh scenarios that require multiple round trips.

---

## Case Format Extension

The existing format is preserved as-is. New cases that need a sequence of responses replace the top-level `response` object with a `responses` array. Runners consume the array in order; any attempt beyond the last entry is an error.

**Existing single-response format (unchanged):**

```json
{
  "name": "preflight_check_returns_decision",
  "description": "A healthy corridor returns a proceed decision.",
  "operation": "preflight.check",
  "input": {
    "source_asset": "USDC",
    "destination_asset": "NGN",
    "amount_usd": 2500
  },
  "request": {
    "method": "POST",
    "path": "/api/v1/preflight",
    "query": {},
    "body": { "source_asset": "USDC", "destination_asset": "NGN", "amount_usd": 2500 }
  },
  "response": { "status": 200, "body": { "decision": "proceed" } },
  "expect": { "result": { "decision": "proceed" } }
}
```

**Extended multi-response format (new cases only):**

```json
{
  "name": "preflight_check_retries_on_503",
  "description": "A transient 503 is retried; the second attempt succeeds.",
  "operation": "preflight.check",
  "input": {
    "source_asset": "USDC",
    "destination_asset": "NGN",
    "amount_usd": 2500
  },
  "request": {
    "method": "POST",
    "path": "/api/v1/preflight",
    "query": {},
    "body": { "source_asset": "USDC", "destination_asset": "NGN", "amount_usd": 2500 }
  },
  "responses": [
    { "status": 503, "body": { "error": "SERVICE_UNAVAILABLE", "message": "try again" } },
    { "status": 200, "body": { "decision": "proceed" } }
  ],
  "expect": {
    "result": { "decision": "proceed" },
    "requests_sent": 2
  }
}
```

**Error case with `expect.error`:**

```json
{
  "name": "preflight_check_gives_up_after_max_retries",
  "description": "After exhausting retries the SDK raises its API error.",
  "operation": "preflight.check",
  "input": { "source_asset": "USDC", "destination_asset": "NGN", "amount_usd": 2500 },
  "request": {
    "method": "POST",
    "path": "/api/v1/preflight",
    "query": {},
    "body": { "source_asset": "USDC", "destination_asset": "NGN", "amount_usd": 2500 }
  },
  "responses": [
    { "status": 503, "body": { "error": "SERVICE_UNAVAILABLE", "message": "try again" } },
    { "status": 503, "body": { "error": "SERVICE_UNAVAILABLE", "message": "try again" } },
    { "status": 503, "body": { "error": "SERVICE_UNAVAILABLE", "message": "try again" } }
  ],
  "expect": {
    "error": { "status": 503, "code": "SERVICE_UNAVAILABLE", "message": "try again" },
    "requests_sent": 3
  }
}
```

**Rate-limit case with `Retry-After` header:**

```json
{
  "name": "preflight_check_respects_retry_after_header",
  "description": "A 429 with Retry-After causes the SDK to wait before retrying.",
  "operation": "preflight.check",
  "input": { "source_asset": "USDC", "destination_asset": "NGN", "amount_usd": 100 },
  "request": {
    "method": "POST",
    "path": "/api/v1/preflight",
    "query": {},
    "body": { "source_asset": "USDC", "destination_asset": "NGN", "amount_usd": 100 }
  },
  "responses": [
    {
      "status": 429,
      "headers": { "retry-after": "1" },
      "body": { "error": "RATE_LIMITED", "message": "slow down" }
    },
    { "status": 200, "body": { "decision": "proceed" } }
  ],
  "expect": {
    "result": { "decision": "proceed" },
    "requests_sent": 2
  }
}
```

---

## Implementation Steps

### 1. Extend the case format in `sdk/conformance/README.md`

Add a section documenting `responses` and `expect.requests_sent`. Update the operations table to include the new `token.refresh` pseudo-operation used by auth-refresh cases. Keep the existing single-`response` format valid — runners treat a top-level `response` as a one-element `responses` array internally.

### 2. Write the new JSON cases

Add one file per new behaviour to `sdk/conformance/cases/`. The full list of cases to add:

| File | Behaviour pinned |
|---|---|
| `anchors_list_paginates_to_next_page.json` | SDK sends `page=2` after a first-page response |
| `anchors_list_stops_at_last_page.json` | SDK does not fetch page 3 when `pages == 2` |
| `preflight_check_retries_on_503.json` | 503 → retry → 200 |
| `preflight_check_retries_on_502.json` | 502 → retry → 200 |
| `preflight_check_gives_up_after_max_retries.json` | Three 503s → raises API error |
| `preflight_check_does_not_retry_on_400.json` | 400 is not retried |
| `preflight_check_does_not_retry_on_404.json` | 404 is not retried |
| `preflight_check_backs_off_on_429.json` | 429 → retry → 200 |
| `preflight_check_respects_retry_after_header.json` | 429 + `Retry-After: 1` → back-off |
| `prices_get_refreshes_token_on_401.json` | 401 → token refresh → retry → 200 |
| `prices_get_raises_on_repeated_401.json` | 401 after refresh → raises auth error |

Name files after the behaviour, not the implementation detail.

### 3. Update the TypeScript runner

The runner in `sdk/typescript/src/conformance.test.ts` replays a single `response`. Extend it to support `responses` as a queue:

```typescript
function replaySequence(tc: ConformanceCase): Sent[] {
  const queue = tc.responses ?? [tc.response!];
  const sent: Sent[] = [];
  vi.stubGlobal("fetch", async (input: string, init: RequestInit = {}) => {
    const response = queue[sent.length] ?? queue[queue.length - 1];
    const url = new URL(input);
    const headers: Record<string, string> = {};
    for (const [k, v] of Object.entries((init.headers ?? {}) as Record<string, string>)) {
      headers[k.toLowerCase()] = v;
    }
    sent.push({
      method: init.method ?? "GET",
      path: url.pathname,
      query: Object.fromEntries(url.searchParams),
      body: typeof init.body === "string" ? JSON.parse(init.body) : null,
      headers,
    });
    const body = response.status === 204 ? null : JSON.stringify(response.body);
    const responseHeaders: Record<string, string> = {
      "content-type": "application/json",
      ...(response.headers ?? {}),
    };
    return new Response(body, { status: response.status, headers: responseHeaders });
  });
  return sent;
}
```

After the call, assert `sent.length === (tc.expect.requests_sent ?? 1)` when `requests_sent` is present.

Update the `ConformanceCase` interface:

```typescript
interface ConformanceCase {
  name: string;
  description: string;
  client?: { api_key?: string };
  operation: string;
  input: Record<string, unknown>;
  request: {
    method: string;
    path: string;
    query: Record<string, string>;
    body: unknown;
    headers?: Record<string, string>;
  };
  // Single-response (existing) or multi-response sequence (new).
  response?: { status: number; headers?: Record<string, string>; body: unknown };
  responses?: Array<{ status: number; headers?: Record<string, string>; body: unknown }>;
  expect: {
    result?: unknown;
    error?: { status: number; code: string; message: string };
    requests_sent?: number;
  };
}
```

### 4. Update the Python runner

The runner in `sdk/python/tests/test_conformance.py` has a single `replay` callback. Replace it with a queue-based side effect:

```python
@pytest.mark.parametrize("case", CASES, ids=[c["name"] for c in CASES])
async def test_conformance(case: Dict[str, Any]):
    name = case["name"]
    call = OPERATIONS.get(case["operation"])
    assert call is not None, f'{name}: operation "{case["operation"]}" has no mapping in OPERATIONS'

    queue = case.get("responses", [case["response"]])
    sent: list[httpx.Request] = []

    def replay(request: httpx.Request) -> httpx.Response:
        resp = queue[len(sent)] if len(sent) < len(queue) else queue[-1]
        sent.append(request)
        if resp["status"] == 204:
            return httpx.Response(204)
        return httpx.Response(
            resp["status"],
            json=resp.get("body"),
            headers=resp.get("headers", {}),
        )

    client = PayRaider(
        base_url=BASE_URL,
        api_key=case.get("client", {}).get("api_key"),
        max_retries=len(queue) - 1,  # allow retries up to the sequence length
    )
    result: Any = None
    error: Any = None
    with respx.mock(assert_all_called=False) as mock:
        mock.route().mock(side_effect=replay)
        try:
            result = await call(client, case["input"])
        except PayRaiderError as err:
            error = err
        finally:
            await client.aclose()

    expected = case["expect"]
    if "requests_sent" in expected:
        assert len(sent) == expected["requests_sent"], f"{name}: requests_sent"

    if "error" in expected:
        assert error is not None, f"{name}: expected a PayRaiderError, got result {result!r}"
        got = {"status": error.status, "code": error.code, "message": str(error)}
        assert got == expected["error"], f"{name}: error"
    else:
        assert error is None, f"{name}: unexpected error {error!r}"
        assert result == expected["result"], f"{name}: result"
```

### 5. Add the MCP server conformance runner

Create `sdk/mcp-server/tests/conformance.test.ts`. The MCP server wraps the TypeScript SDK, so a conformance case exercises the full chain: MCP tool call → SDK method → HTTP intercept.

Map each neutral operation name to the tool name exposed by the MCP server:

```typescript
// sdk/mcp-server/tests/conformance.test.ts
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it, vi } from "vitest";
import { READ_ONLY_TOOLS } from "../src/tools.js";

const CASES_DIR = fileURLToPath(
  new URL("../../../conformance/cases", import.meta.url)
);

/** Neutral operation -> MCP tool name + argument mapping. */
const TOOL_MAPPINGS: Record<
  string,
  (input: Record<string, unknown>) => { tool: string; args: Record<string, unknown> }
> = {
  "preflight.check": (i) => ({
    tool: "preflight_payment",
    args: {
      source_asset: i.source_asset,
      destination_asset: i.destination_asset,
      ...(i.amount_usd !== undefined && { amount_usd: i.amount_usd }),
      ...(i.min_success_rate !== undefined && { min_success_rate: i.min_success_rate }),
    },
  }),
  "prices.get": (i) => ({ tool: "get_prices", args: { assets: [i.asset] } }),
  "prices.batch": (i) => ({ tool: "get_prices", args: { assets: i.assets } }),
  "anchors.list": (i) => ({
    tool: "list_anchors",
    args: { page: i.page, limit: i.limit },
  }),
  "anchors.get": (i) => ({ tool: "get_anchor", args: { id: i.id } }),
  "corridors.get": (i) => ({
    tool: "get_corridor",
    args: { source: i.source, destination: i.destination },
  }),
  "network.info": () => ({ tool: "network_info", args: {} }),
};

// Load only the cases whose operation the MCP server exposes.
const cases = readdirSync(CASES_DIR)
  .filter((f) => f.endsWith(".json"))
  .sort()
  .map((f) => JSON.parse(readFileSync(join(CASES_DIR, f), "utf8")))
  .filter((tc) => tc.operation in TOOL_MAPPINGS);

// ... (replay helper and test loop follow the same pattern as conformance.test.ts)
```

Ops that the MCP server intentionally does not expose (such as `webhooks.delete`) are skipped — the filter on line `tc.operation in TOOL_MAPPINGS` ensures only relevant cases run.

### 6. Add the CI matrix workflow

Create `.github/workflows/sdk-conformance.yml`:

```yaml
# Shared conformance suite — all three SDKs must pass all cases (#2841).
name: SDK conformance

on:
  push:
    branches: [main]
    paths:
      - "sdk/conformance/**"
      - "sdk/typescript/**"
      - "sdk/python/**"
      - "sdk/mcp-server/**"
      - ".github/workflows/sdk-conformance.yml"
  pull_request:
    paths:
      - "sdk/conformance/**"
      - "sdk/typescript/**"
      - "sdk/python/**"
      - "sdk/mcp-server/**"
      - ".github/workflows/sdk-conformance.yml"

jobs:
  conformance:
    name: conformance / ${{ matrix.sdk }}
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false   # run all three; show all failures at once
      matrix:
        sdk:
          - typescript
          - python
          - mcp-server
    steps:
      - uses: actions/checkout@v4

      - name: Set up Node.js
        if: matrix.sdk == 'typescript' || matrix.sdk == 'mcp-server'
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm
          cache-dependency-path: sdk/${{ matrix.sdk }}/package-lock.json

      - name: Set up Python
        if: matrix.sdk == 'python'
        uses: actions/setup-python@v5
        with:
          python-version: "3.12"

      - name: Install dependencies (Node)
        if: matrix.sdk == 'typescript' || matrix.sdk == 'mcp-server'
        run: npm ci
        working-directory: sdk/${{ matrix.sdk }}

      - name: Install dependencies (Python)
        if: matrix.sdk == 'python'
        run: pip install -e ".[dev]"
        working-directory: sdk/python

      - name: Run conformance (TypeScript)
        if: matrix.sdk == 'typescript'
        run: npm test -- conformance
        working-directory: sdk/typescript

      - name: Run conformance (Python)
        if: matrix.sdk == 'python'
        run: pytest tests/test_conformance.py -v
        working-directory: sdk/python

      - name: Run conformance (MCP server)
        if: matrix.sdk == 'mcp-server'
        run: npm test -- conformance
        working-directory: sdk/mcp-server
```

`fail-fast: false` ensures all three SDKs report their failures in the same run rather than stopping at the first. A PR is blocked if any matrix leg fails.

### 7. Update `sdk/conformance/README.md`

- Add a row for `mcp-server` to the **Run** section.
- Document the `responses` array field and `expect.requests_sent`.
- Update the operations table to include the MCP server column.
- Add a **Case categories** section listing: happy-path, error, retry, pagination, rate-limit, auth-refresh.

### 8. Update the top-level `docs/README.md`

Add a row for `SDK_CONFORMANCE_SUITE.md` in the documentation index.

---

## Acceptance Criteria

- [ ] All 13 existing cases continue to pass in TypeScript, Python, and (after this change) MCP server.
- [ ] All new cases (error, retry, pagination, rate-limit, auth-refresh) pass in all three runners.
- [ ] The CI matrix (`sdk-conformance.yml`) is green on the main branch.
- [ ] A PR that breaks one case in one SDK fails CI even if the other two SDKs pass.
- [ ] `sdk/conformance/README.md` reflects the extended format and all three runners.
- [ ] No new TODOs without a linked issue.

---

## Test Plan

Each new case file is its own test. The full plan is:

| Category | Cases | Runners |
|---|---|---|
| Happy path | All 13 existing | TypeScript, Python, MCP server |
| Pagination | `anchors_list_paginates_to_next_page`, `anchors_list_stops_at_last_page` | TypeScript, Python |
| Retries | `preflight_check_retries_on_503`, `preflight_check_retries_on_502`, `preflight_check_gives_up_after_max_retries`, `preflight_check_does_not_retry_on_400`, `preflight_check_does_not_retry_on_404` | TypeScript, Python |
| Rate limits | `preflight_check_backs_off_on_429`, `preflight_check_respects_retry_after_header` | TypeScript, Python |
| Auth refresh | `prices_get_refreshes_token_on_401`, `prices_get_raises_on_repeated_401` | TypeScript, Python |

MCP server cases cover only the operations the server exposes (happy-path and error categories). Retry, rate-limit, and auth-refresh behaviour is tested at the SDK layer, not duplicated in the MCP layer.

**All SDKs must pass all applicable cases before this issue is closed.**

---

## Design Notes

**Why JSON cases, not test code?**  
Test code in one language cannot run in another. JSON cases are language-neutral and straightforward to review in pull requests — a reviewer sees the HTTP exchange directly, without decoding test framework idioms.

**Why `responses` (plural) instead of a richer DSL?**  
A simple ordered array is enough to express every scenario needed here. A DSL with conditionals or state machines would be harder to read and harder to implement consistently across three runners. If a scenario genuinely cannot be expressed as a fixed sequence of responses, it belongs in SDK-specific unit tests, not in the shared conformance suite.

**Single-response cases are not changed.**  
Runners treat a top-level `response` field as equivalent to `responses: [response]`. All existing case files are valid without modification.

**MCP server coverage is narrower by design.**  
The MCP server exposes a read-only subset of the API (no `webhooks.delete`, no write operations). The conformance runner for the MCP server skips cases whose operation is not in the tool mapping table. This is explicit: if a case is skipped because the tool does not exist, that is correct. If a case is skipped because the mapping is missing, that is a bug.

**`fail-fast: false` in CI.**  
Running all three legs to completion on every PR makes it possible to see which SDKs are broken and which are not. A single-SDK fix in a PR should show the other two legs green, confirming the fix is scoped correctly.

**Retry count in the Python runner.**  
The Python runner sets `max_retries` equal to `len(queue) - 1` so that multi-step cases can complete their full sequence. Single-response cases continue to run with `max_retries=0` (one-element queue → `max_retries=0`), matching the existing behaviour.

---

## References

- [`sdk/conformance/README.md`](../sdk/conformance/README.md) — case format, operations table, run instructions
- [`sdk/typescript/src/conformance.test.ts`](../sdk/typescript/src/conformance.test.ts) — TypeScript runner (127 lines)
- [`sdk/python/tests/test_conformance.py`](../sdk/python/tests/test_conformance.py) — Python runner
- [`sdk/mcp-server/tests/`](../sdk/mcp-server/tests/) — MCP server test directory (no conformance runner yet)
- [`sdk/conformance/cases/`](../sdk/conformance/cases/) — 13 existing case files
- [`.github/workflows/sdk-release.yml`](../.github/workflows/sdk-release.yml) — existing SDK CI for reference
- [SDK_TYPESCRIPT_SOURCE_LAYOUT.md](./SDK_TYPESCRIPT_SOURCE_LAYOUT.md) — related SDK structural change
