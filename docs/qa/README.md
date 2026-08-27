# Manual QA Test Plans

Test plans for the browser-verification issues opened after the stabilization pass
(#1813–#1816). That pass got backend, contracts, and frontend build/typecheck/lint green,
but per the project's own guidance:

> Type checking and test suites verify code correctness, not feature correctness.

Nothing in these flows was clicked through in a running browser. These documents exist so
that verification is reproducible: same environment, same steps, same expected results,
regardless of who runs it.

## Test plans

| Issue | Route | Plan |
| --- | --- | --- |
| #1813 | `/[locale]/sep6` | [SEP-6 deposit & withdraw](1813-sep6.md) |
| #1814 | `/[locale]/sep10-demo` | [SEP-10 auth demo](1814-sep10-demo.md) |
| #1815 | `/[locale]/trustlines` | [Trustlines](1815-trustlines.md) |
| #1816 | `/[locale]/send-payment` | [Send payment (SEP-31)](1816-send-payment.md) |

## Read this first: known blockers

A code read done while writing these plans turned up defects that will stop several of
these flows before a tester gets to the interesting parts. Each is written up in full in
its own plan, but they are collected here because a tester who hits them cold will assume
their environment is broken.

| Blocker | Affects | Detail |
| --- | --- | --- |
| SEP-10, SEP-31, SEP-24, and trustlines routers are defined but never merged into the axum app | #1813, #1814, #1815, #1816 | `sep10::routes()`, `sep31_proxy::routes()`, `sep24_proxy::routes()`, and `trustlines::routes()` are referenced only from [`backend/src/openapi.rs`](../../backend/src/openapi.rs). The app router assembled in [`backend/src/api/v1/mod.rs`](../../backend/src/api/v1/mod.rs) never merges them, so every one of those paths 404s on a running server. |
| No SEP-6 backend routes exist at all | #1813 | [`backend/src/api/`](../../backend/src/api/) contains `sep24_proxy.rs` and `sep31_proxy.rs` but no SEP-6 proxy. The frontend calls `/api/sep6/*` unconditionally. |
| `Sep31PaymentFlow` has a duplicate binding and an undeclared identifier | #1816 | [`frontend/src/components/Sep31PaymentFlow.tsx:86`](../../frontend/src/components/Sep31PaymentFlow.tsx#L86) redeclares `transferServer` (already bound at line 67) and reads `customTransferServer`, which is never declared in the file. |
| Trustlines has no date range picker | #1815 | The issue asks to confirm "the date range picker and history chart both populate." Neither [the page](../../frontend/src/app/%5Blocale%5D/trustlines/page.tsx) nor [`TrustlineGrowthChart`](../../frontend/src/components/charts/TrustlineGrowthChart.tsx) contains one; history is fetched at a fixed `limit=30`. |

None of these were verified in a browser — they come from reading the code — so confirm
each against the running app before filing. Where a plan says "expected (blocker)", that
is the behavior predicted by the code read, not an observed result.

## Environment setup

All four plans assume the same setup.

### 1. Backend

Follow [`docs/testnet-quickstart.md`](../testnet-quickstart.md) to fund a testnet account
and populate `backend/.env`. Then:

```bash
cd backend
cargo run
```

The server needs a reachable Postgres (see `backend/.env.example`) — an empty or
unmigrated database will surface as empty result sets rather than errors, which is easy to
misread as a frontend bug.

Confirm the backend is up before touching the frontend:

```bash
curl -s localhost:8080/health
```

### 2. Frontend

`NEXT_PUBLIC_API_URL` is **required**. [`frontend/src/config.ts`](../../frontend/src/config.ts)
throws at module load if it is unset, so a missing value fails the page hard rather than
falling back to a default:

```bash
cd frontend
export NEXT_PUBLIC_API_URL=http://localhost:8080
npm run dev
```

### 3. Browser

- Chrome or Firefox with devtools open on **Console** and **Network** for the whole session.
- Preserve log / persist network across navigations — several of these flows redirect.
- A Stellar wallet extension (Freighter is the best-supported) on **testnet**, funded via
  Friendbot. Required for #1814 and #1816; #1813 and #1815 need no wallet.

### Locale

`[locale]` is a route segment. Run each plan at least once at `/en/...`. If the project's
supported locales include an RTL or long-string locale, a second pass there is worth it —
these pages use fixed-width utility classes (`w-24`, `min-w-[200px]`) that can clip
translated labels.

## How to record a result

Each plan ends with a results table. Copy it into the issue, fill in Pass / Fail / Blocked
per step, and attach:

- A screenshot of the Console tab (including the clean case — "no errors" is a claim that
  needs evidence).
- The Network tab filtered to the relevant API calls, with status codes visible.
- For any failure: the request URL, status, response body, and what the UI rendered
  instead.

## Filing follow-up bugs

One bug per distinct failure, not one per plan. Include:

1. Route and exact steps to reproduce, starting from a fresh page load.
2. Observed vs. expected.
3. The failing network request (method, URL, status, response body).
4. Console output, verbatim.
5. Backend commit SHA and frontend commit SHA.
6. A link back to the QA issue (#1813–#1816).

Then link the new bug from the QA issue and leave the QA issue open until every step in
its plan passes.
