# QA: Transaction builder + signature collector (`/[locale]/transactions/builder`)

**Issue:** [#1817](https://github.com/Ndifreke000/stellar-insights/issues/1817)
**Route(s):** `/[locale]/transactions/builder`

## Background

This route is currently a **known placeholder** — `TransactionBuilder.tsx` and
`SignatureCollector.tsx` both render a static "currently undergoing maintenance" message. There's
a dedicated feature-gap issue tracking the real implementation of those two components.

This QA item exists to **re-verify status once that feature work lands** — it is not meant to
test real functionality now, since there isn't any yet.

## Test steps (current placeholder state)

1. Load `/[locale]/transactions/builder` (e.g. `/en/transactions/builder`) with DevTools Console
   open. Confirm no console errors on load.
2. Confirm both `TransactionBuilder` and `SignatureCollector` render their static "currently
   undergoing maintenance" message, with no broken layout or unhandled errors.

## Test steps (once the feature-gap work lands — re-run this doc then)

1. Load the route with DevTools Console and Network tab open. Confirm no console errors on load.
2. Exercise the transaction builder: construct a representative transaction and confirm it builds
   correctly.
3. Exercise the signature collector: confirm it can collect a signature (or simulate the intended
   signing flow) and hands off to the builder correctly.
4. Confirm any API calls made by either component succeed against a real backend (Network tab).

## Acceptance criteria

- [ ] `npm run dev` (or a deployed preview) loads the route with no console errors
- [ ] Current state confirmed: both components show the maintenance placeholder cleanly (no
      crashes, no console errors) — **or**, if the feature-gap work has landed, the builder and
      collector work as intended end-to-end (not just "renders without crashing")
- [ ] Any API calls the page makes succeed against a real backend (verified in Network tab)
- [ ] Any failure found is filed as a follow-up bug with the specific failure, linked below

## Result

_Fill in after running the manual pass: pass/fail, date, browser, whether the feature-gap work had
landed yet, and links to any follow-up issues filed._
