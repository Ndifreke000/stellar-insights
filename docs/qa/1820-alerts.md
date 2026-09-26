# QA: Alerts (`/alerts`)

**Issue:** [#1820](https://github.com/Ndifreke000/payraider/issues/1820)
**Route(s):** `/alerts`

## Background

This session's stabilization pass got the backend, contracts, and frontend build/typecheck/lint
all green, but nothing on this route was clicked through in a running browser. This doc is the
manual verification pass that was missing.

Of particular interest: the `/history/{id}/snooze` route's axum path-param syntax was fixed this
session. This is the first real check that the button wired to it actually works end-to-end.

## Test steps

1. Load `/alerts` with DevTools Console and Network tab open. Confirm no console errors on load
   and no failed (4xx/5xx) requests in the Network tab.
2. **Alert rules list**
   - Confirm existing alert rules render with their configured conditions/thresholds.
   - **Create a new rule**: open the create-rule flow, fill in required fields, submit. Confirm
     the request succeeds in the Network tab and the new rule appears in the list without a
     manual refresh.
   - **Edit an existing rule**: change a field (e.g. threshold or condition), save. Confirm the
     update request succeeds and the list reflects the new value.
3. **Alert history list**
   - Confirm history entries render (timestamp, rule, status).
   - **Mark an entry read**: click the "read" action. Confirm the request succeeds and the UI
     updates the entry's status.
   - **Dismiss an entry**: click "dismiss". Confirm the request succeeds and the entry is
     removed/updated as expected.
   - **Snooze an entry**: click "snooze" — this is the one that hits `/history/{id}/snooze`.
     Confirm in the Network tab that the request goes out with the correct `{id}` in the path,
     returns a success status, and the UI reflects the snoozed state (not just an optimistic
     update that silently fails).
4. Re-check the Console tab for any errors or warnings surfaced during the above interactions.

## Acceptance criteria

- [ ] `npm run dev` (or a deployed preview) loads `/alerts` with no console errors
- [ ] Create rule works end-to-end (request succeeds, list updates)
- [ ] Edit rule works end-to-end (request succeeds, list reflects change)
- [ ] Mark-read / dismiss / snooze all work end-to-end, including the `/history/{id}/snooze` call
- [ ] All API calls succeed against a real backend (verified in Network tab)
- [ ] Any failure found is filed as a follow-up bug with the specific failure, linked below

## Result

_Fill in after running the manual pass: pass/fail, date, browser, and links to any follow-up
issues filed._
