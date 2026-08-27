# QA: Calculator (`/[locale]/calculator`)

**Issue:** [#1818](https://github.com/Ndifreke000/stellar-insights/issues/1818)
**Route(s):** `/[locale]/calculator`

## Background

This session's stabilization pass got the backend, contracts, and frontend build/typecheck/lint
all green, but nothing on this route was clicked through in a running browser. This doc is the
manual verification pass that was missing.

The specific fix to re-verify: a duplicate `CostCalculator` import (one static, one `dynamic()`
with `ssr: false`) was resolved. Confirm the page now renders exactly one calculator widget and
the dynamic import doesn't produce a hydration mismatch.

## Test steps

1. Load `/[locale]/calculator` (e.g. `/en/calculator`) with DevTools Console and Network tab open.
2. **Single widget**: visually confirm exactly one fee/cost calculator widget renders — not two
   overlapping instances (the regression this session's fix addressed).
3. **Hydration check**: watch the Console during initial load specifically for a React hydration
   mismatch warning (e.g. "Hydration failed because the initial UI does not match..."). There
   should be none.
4. **Functional check**: exercise the calculator itself — enter representative inputs and confirm
   it computes and displays a result correctly.
5. If the calculator makes any API calls (e.g. for live fee/price data), confirm they succeed in
   the Network tab.

## Acceptance criteria

- [ ] `npm run dev` (or a deployed preview) loads `/[locale]/calculator` with no console errors
- [ ] Exactly one calculator widget renders (no duplicate from the old static+dynamic import)
- [ ] No hydration mismatch warning in the console
- [ ] Calculator computes correct results for representative inputs
- [ ] Any API calls the page makes succeed against a real backend (verified in Network tab)
- [ ] Any failure found is filed as a follow-up bug with the specific failure, linked below

## Result

_Fill in after running the manual pass: pass/fail, date, browser, and links to any follow-up
issues filed._
