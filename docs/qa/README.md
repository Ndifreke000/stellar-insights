# Manual QA: Post-Stabilization Browser Verification

This session's stabilization pass got the backend, contracts, and frontend build/typecheck/lint
all green, but per the project's own guidance ("Type checking and test suites verify code
correctness, not feature correctness"), nothing in the routes below was actually clicked through
in a running browser — there wasn't a way to do that non-interactively in that pass.

Each doc in this folder is a manual test plan for one GitHub issue. Run through the steps against
`npm run dev` (or a deployed preview) with a real backend, check the browser console and Network
tab as you go, and record the outcome at the bottom of the doc. If something's broken, file a
follow-up bug with the specific failure and link it from the doc.

| Issue | Route | Doc |
|---|---|---|
| [#1820](https://github.com/Ndifreke000/stellar-insights/issues/1820) | `/alerts` | [1820-alerts.md](1820-alerts.md) |
| [#1819](https://github.com/Ndifreke000/stellar-insights/issues/1819) | `/[locale]/network` | [1819-network-graph.md](1819-network-graph.md) |
| [#1818](https://github.com/Ndifreke000/stellar-insights/issues/1818) | `/[locale]/calculator` | [1818-calculator.md](1818-calculator.md) |
| [#1817](https://github.com/Ndifreke000/stellar-insights/issues/1817) | `/[locale]/transactions/builder` | [1817-transaction-builder.md](1817-transaction-builder.md) |

## Common setup

1. Start a real backend (not a mock) — the acceptance criteria for every route require API calls
   to succeed against a live backend, checked in the Network tab.
2. `npm run dev` in the frontend, or point at a deployed preview URL.
3. Open the browser DevTools Console and Network tab before navigating to the route.
4. Work through the route's doc top to bottom, checking off each acceptance criterion.
5. Record the result (pass/fail + notes) in the "Result" section of the doc, and if you find a
   break, open a follow-up issue and link it there.
