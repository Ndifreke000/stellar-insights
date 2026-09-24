# QA: Send payment (`/[locale]/send-payment`) — #1816

Manual browser verification of the SEP-31 cross-border payment flow: anchor selection,
quote, payment submission, and transaction status.

Setup and reporting conventions: [README](README.md).

## What the code does

[`send-payment/page.tsx`](../../frontend/src/app/%5Blocale%5D/send-payment/page.tsx) is a
thin header wrapper. All behavior lives in
[`Sep31PaymentFlow`](../../frontend/src/components/Sep31PaymentFlow.tsx), loaded via
`next/dynamic` with `ssr: false` — so a client-side throw shows as an empty region under
the header, not a server error page.

The form uses `react-hook-form` + a zod resolver (`sep31PaymentFlowSchema` in
[`frontend/src/lib/schemas.ts`](../../frontend/src/lib/schemas.ts)) with `mode: "onChange"`.
Submission is gated on `isValid && isDirty`.

### Network calls

All go through [`services/sep31.ts`](../../frontend/src/services/sep31.ts), which prefixes
`NEXT_PUBLIC_API_URL` via [`fetchWithRetry`](../../frontend/src/services/fetchWithRetry.ts).

| Trigger | Request |
| --- | --- |
| Load anchors | `GET /api/sep31/anchors` |
| Anchor selected | `GET /api/sep31/info?...` |
| Get quote | `POST /api/sep31/quote` |
| Send payment | `POST /api/sep31/transactions` |
| Refresh transactions | `GET /api/sep31/transactions?...` |

`fetchWithRetry` retries **once** on HTTP 503, honoring `Retry-After` (capped at 5s). On
any other non-2xx it throws `Sep31Error` carrying `message` / `error` from the body, else
`statusText`. Expect exactly two requests for a 503, one for anything else.

## Blockers found by code read

Confirm these against the running app before filing — they come from reading the source,
not from a browser.

### B1 — `Sep31PaymentFlow` will not compile

[`Sep31PaymentFlow.tsx:86`](../../frontend/src/components/Sep31PaymentFlow.tsx#L86):

```ts
const transferServer = selectedAnchor?.transfer_server || customTransferServer.trim();
```

`transferServer` is already bound at line 67 (`watch("transferServer")`), and
`customTransferServer` is not declared anywhere in the file. That is a redeclaration of a
block-scoped binding plus an unresolved identifier — both hard errors.

If the stabilization pass reported typecheck green, that is worth reconciling before
anything else: either this line landed after the check, or the component is being excluded
from it. Either way, **step 1 below is the whole test until this is resolved** — a module
that fails to evaluate means the dynamic import rejects and the page renders header-only.

### B2 — SEP-31 backend routes are not mounted

`sep31_proxy::routes()` is referenced only from
[`backend/src/openapi.rs`](../../backend/src/openapi.rs). The router assembled in
[`backend/src/api/v1/mod.rs`](../../backend/src/api/v1/mod.rs) never merges it, so
`/api/sep31/*` should 404 against a running backend even once B1 is fixed.

Sanity check without the browser:

```bash
curl -i localhost:8080/api/sep31/anchors
```

A 404 confirms B2. A 200 means the route is reachable by some path this read missed —
note that on the issue and continue with the plan.

## Test steps

### 1. Page loads

1. Navigate to `/en/send-payment` with devtools open.
2. Header ("SEP-31 // Cross-border", "Send payment") renders.
3. The flow below the header renders form controls — not blank space.

**Expected (blocker):** with B1 unfixed, the header renders and the region below it is
empty, with a module-evaluation error in the console. Capture the exact console text; it
is the bug report.

Do not proceed past this step until the form renders. Everything below assumes it does.

### 2. Anchors load

1. On mount / on "Refresh", `GET /api/sep31/anchors` fires.
2. The anchor select populates from the response.

**Expected (blocker):** 404 per B2. The catch sets the error banner from `e.message`.
Check what actually renders — a 404 body that is not JSON yields `statusText`, so the
banner may read "Not Found", which is technically a string but tells the user nothing.
That is worth its own bug even after B2 is fixed.

### 3. Anchor info

1. Select an anchor.
2. `GET /api/sep31/info` fires; asset dropdowns populate from the response.
3. Selecting a different anchor refetches and **replaces** the asset lists — confirm stale
   assets from the previous anchor are gone, not merged.

### 4. Quote

1. Enter an amount, source asset, destination asset.
2. Click the quote action. `POST /api/sep31/quote` fires.
3. Quote details render (rate, fee, expiry as returned).
4. With an empty amount, the code short-circuits to the error "Enter amount and select an
   anchor" without a network call — confirm no request appears in the Network tab.

### 5. Submit payment

1. Fill in receiver id and any anchor-required fields.
2. Confirm the submit button is disabled until the form is both valid and dirty.
3. Submit. `POST /api/sep31/transactions` fires.
4. A success message renders with the returned transaction id.
5. Refresh transactions — the new transaction appears with a status.

### 6. Error states

The issue calls out three specifically. Each needs to render something a user can act on,
not a raw error object or `[object Object]`.

| Case | How to produce | Expect |
| --- | --- | --- |
| Insufficient balance | Amount exceeding the funded testnet balance | Anchor/backend error surfaced as readable text naming the balance problem |
| No trustline | Destination asset the account has no trustline for | Readable text naming the missing trustline, ideally the asset code |
| Rejected signature | Start the flow, reject the signing prompt in the wallet | Flow returns to an editable state; no spinner left running; message says the signature was declined |
| Backend 503 | Stop the backend mid-flow, or force a 503 | Exactly two attempts ~1s apart, then a readable error |
| Malformed anchor URL | Type a non-URL into the transfer server field | Zod validation message inline on the field, no request fired |

For each, screenshot the rendered message. `Sep31Error` preserves `status` and `data`, so
check whether the UI uses them or falls back to `statusText`.

### 7. Console and network audit

- Zero console errors and zero unhandled rejections across the whole session.
- No request returns 4xx/5xx except the ones deliberately provoked in step 6.
- No secret (JWT, wallet key) appears in a query string — the JWT should travel in a body
  or header. Check the Network tab request URLs directly.

## Acceptance criteria

- [ ] `/en/send-payment` loads with no console errors
- [ ] The SEP-31 form renders (B1 resolved)
- [ ] Anchors and anchor info load from a real backend (B2 resolved)
- [ ] A quote returns and renders
- [ ] A payment submits and appears in the transaction list with a status
- [ ] Insufficient balance renders a meaningful message
- [ ] No trustline renders a meaningful message
- [ ] Rejected signature renders a meaningful message and leaves the form usable
- [ ] Every API call the page makes returns 2xx (Network tab evidence attached)
- [ ] Any failure has a follow-up bug filed and linked from #1816

## Results

| Step | Result | Notes / bug link |
| --- | --- | --- |
| 1. Page loads | | |
| 2. Anchors load | | |
| 3. Anchor info | | |
| 4. Quote | | |
| 5. Submit payment | | |
| 6a. Insufficient balance | | |
| 6b. No trustline | | |
| 6c. Rejected signature | | |
| 6d. 503 retry | | |
| 7. Console / network clean | | |

Tester: &nbsp; Date: &nbsp; Frontend SHA: &nbsp; Backend SHA:
