# QA: Trustlines (`/[locale]/trustlines`) — #1815

Manual browser verification of the trustlines page: network stats, the top-assets
leaderboard, and the per-asset-pair history view.

Setup and reporting conventions: [README](README.md).

## What the code does

[`trustlines/page.tsx`](../../frontend/src/app/%5Blocale%5D/trustlines/page.tsx) is a
client component. On mount it runs one effect that:

1. Fires `fetchTrustlineStats()` and `fetchTrustlineRankings(50)` in parallel.
2. Auto-selects `rankings[0]` and fetches its history.
3. Clears the full-page loading spinner in `finally`.

Clicking a row in the leaderboard calls `handleSelectAsset`, which sets the selection and
fetches that asset's history.

### Network calls

Via [`lib/trustline-api.ts`](../../frontend/src/lib/trustline-api.ts), prefixed with
`NEXT_PUBLIC_API_URL`:

| Trigger | Request |
| --- | --- |
| Mount | `GET {API}/trustlines/stats` |
| Mount | `GET {API}/trustlines/rankings?limit=50` |
| Mount + row click | `GET {API}/trustlines/{asset_code}/{asset_issuer}/history?limit=30` |

Note there is **no `/api` prefix** on these client paths, while the backend handlers are
annotated `/api/trustlines/...` in their utoipa attributes. Worth checking against the
running server (see B1).

### The failure mode that matters most

All three fetch helpers catch their own errors, log via `logger.error`, and **return an
empty fallback** (`{total_assets_tracked: 0, ...}` / `[]`). Nothing propagates to the
component, and the component's own `catch` only logs.

So a completely dead backend renders as: spinner clears, three stat cards showing `0`, an
empty leaderboard, and the "Select an asset from the leaderboard" placeholder. **There is
no error state on this page at all.** A tester who does not have the Network tab open will
record this as "loads fine, just no data."

Keep the Network tab open for every step of this plan. That is the only way this page
tells you it is broken.

## Blockers found by code read

Confirm against the running app before filing.

### B1 — trustlines routes are not mounted

`trustlines::routes()` is referenced only from
[`backend/src/openapi.rs`](../../backend/src/openapi.rs). The app router in
[`backend/src/api/v1/mod.rs`](../../backend/src/api/v1/mod.rs) never merges it.

```bash
curl -i localhost:8080/trustlines/stats
curl -i localhost:8080/api/trustlines/stats
```

Expect 404 from both. If one returns 200, the route is reachable and the real defect is
the prefix mismatch between client and server — note which.

Combined with the swallowed errors above, B1's user-visible symptom is a page of zeros.

### B2 — the issue's premise about a date range picker

The issue asks to "confirm the date range picker and history chart both populate." There is
no date range picker on this page. Neither the page nor
[`TrustlineGrowthChart`](../../frontend/src/components/charts/TrustlineGrowthChart.tsx)
renders one; history is requested at a hard-coded `limit=30` with no user control over the
window, and `limit` is a row count, not a date range.

Two possible readings — resolve with whoever filed #1815 rather than guessing:

- The picker was expected and is missing → file a **missing feature** bug.
- The issue meant the asset selector (the leaderboard rows) → test it as step 4 below and
  amend the issue's wording.

Until that is settled, this plan verifies what exists.

## Test steps

### 1. Page loads

1. Navigate to `/en/trustlines` with Console **and Network** open.
2. The "Syncing Ledger States…" spinner appears, then clears.
3. Header and three stat cards render.

Record how long the spinner is up. It blocks the entire page — there is no partial render —
so a slow `rankings?limit=50` holds the whole route.

### 2. Stats cards carry real data

1. Confirm `GET /trustlines/stats` returned **200** in the Network tab.
2. "Total Trustlines" and "Tracked Assets" show non-zero, plausible values.
3. Cross-check one value against the raw response body.

**Zeros with a 404/500 in the Network tab is a failure, not an empty dataset.** This is
the step B1 fails.

Also note: the "Growth Trend" card is hard-coded to the literal string "Positive" with the
caption "Based on rolling 30-day average". It is not computed from any response. If that is
not intended as a placeholder, it is a separate bug — a stat card that always reads
"Positive" is worse than no card.

### 3. Leaderboard populates

1. `GET /trustlines/rankings?limit=50` returned 200.
2. Rows render with asset code, truncated issuer, and trustline count.
3. The list scrolls inside its fixed `h-[600px]` container without breaking page layout.
4. The first row is auto-selected and highlighted.

### 4. Per-asset-pair history

1. On load, `GET /trustlines/{code}/{issuer}/history?limit=30` fired for the top asset.
2. The chart renders with plotted points — not an empty axis frame.
3. Click a different row. Confirm:
   - a new history request fires with **that row's** code and issuer, URL-encoded;
   - the chart re-renders with different data;
   - the asset header (code, issuer, total supply) updates to match;
   - the selection highlight moves.
4. Click several rows quickly. `handleSelectAsset` has no request-ordering guard, so a slow
   earlier response can land after a fast later one and leave the chart showing the wrong
   asset. Check whether the rendered chart matches the highlighted row after rapid clicks —
   if it does not, that is a real race worth filing.
5. Pick an asset whose issuer contains characters needing encoding, if one exists, and
   confirm the URL is encoded correctly.

### 5. Holder distribution bar

1. Authorized / unauthorized counts render.
2. The two-segment bar's widths look proportional to those counts.
3. **Find an asset with `total_trustlines === 0`** if the dataset has one. The widths are
   computed as `authorized / total * 100` with no zero guard, so a zero total yields `NaN%`.
   Check what the browser does with it — a collapsed or full-width bar is a bug.
4. The caption below switches correctly between "Includes trustlines pending
   authorization or revoked" and "All trustlines are fully authorized".

### 6. Empty and edge states

| Case | How to produce | Expect |
| --- | --- | --- |
| Empty rankings | Backend returns `[]` | Placeholder "Select an asset from the leaderboard" renders; no crash |
| Empty history | Asset with no snapshots | Chart renders an empty state, not a broken axis or a thrown error |
| Backend down | Stop the backend, reload | **Currently:** page of zeros, errors only in console. Confirm and file — a user-visible error state is missing |

### 7. Console and network audit

- Zero console errors on the happy path. `logger.error` output during the backend-down case
  is expected; note the exact text.
- Every trustlines request returns 200.
- Layout holds at ~375px width — this page uses fixed heights and truncation widths
  (`w-24`, `h-[600px]`) that are worth a mobile-width pass.

## Acceptance criteria

- [ ] `/en/trustlines` loads with no console errors
- [ ] All three trustlines endpoints return 200 (B1 resolved)
- [ ] Stat cards show real values from the response, not zero fallbacks
- [ ] Leaderboard populates and scrolls
- [ ] History chart populates for the auto-selected asset
- [ ] Selecting a different asset refetches and re-renders the chart correctly
- [ ] Date range picker question resolved with the issue author (present-and-working, or bug filed for missing)
- [ ] Backend-down case renders something meaningful, or a bug is filed for the missing error state
- [ ] Any failure has a follow-up bug filed and linked from #1815

## Results

| Step | Result | Notes / bug link |
| --- | --- | --- |
| 1. Page loads | | |
| 2. Stats cards | | |
| 3. Leaderboard | | |
| 4. History chart + reselect | | |
| 4.4 Rapid-click ordering | | |
| 5. Distribution bar | | |
| 6. Empty / down states | | |
| 7. Console / network clean | | |

Tester: &nbsp; Date: &nbsp; Frontend SHA: &nbsp; Backend SHA:
