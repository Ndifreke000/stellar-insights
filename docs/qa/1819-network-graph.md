# QA: Network graph (`/[locale]/network`)

**Issue:** [#1819](https://github.com/Ndifreke000/payraider/issues/1819)
**Route(s):** `/[locale]/network`

## Background

This session's stabilization pass got the backend, contracts, and frontend build/typecheck/lint
all green, but nothing on this route was clicked through in a running browser. This doc is the
manual verification pass that was missing.

Two fixes to specifically re-verify:
- A duplicate `NetworkGraph` dynamic-import block was removed — confirm only **one** graph
  instance renders, not two.
- `/api/network-graph` now prefers `NEXT_PUBLIC_API_URL` over the older
  `NEXT_PUBLIC_BACKEND_URL` — confirm the request actually goes to the intended backend and
  returns real data.

## Test steps

1. Load `/[locale]/network` (e.g. `/en/network`) with DevTools Console and Network tab open.
   Confirm no console errors on load.
2. **Single graph instance**: visually confirm exactly one network graph renders on the page —
   not two overlapping/duplicate graphs (the regression this session's fix addressed).
3. **Interactivity**:
   - Pan the graph (click-drag) and confirm the viewport moves.
   - Zoom in/out (scroll or zoom controls) and confirm the graph scales.
   - Click a node and confirm it responds (selection highlight, detail panel, tooltip — whatever
     the intended interaction is) rather than being inert.
4. **Data source**: in the Network tab, find the request to `/api/network-graph`. Confirm it
   succeeds (2xx) and returns non-empty, real data — not a fallback/empty state. Confirm it's
   hitting the URL derived from `NEXT_PUBLIC_API_URL`, not a stale `NEXT_PUBLIC_BACKEND_URL`
   value.
5. Re-check the Console tab for hydration warnings or errors surfaced during interaction.

## Acceptance criteria

- [ ] `npm run dev` (or a deployed preview) loads `/[locale]/network` with no console errors
- [ ] Exactly one `NetworkGraph` renders (no duplicate from the old dynamic-import block)
- [ ] Graph is interactive: pan, zoom, and node click all work
- [ ] `/api/network-graph` returns real data from the correct backend (`NEXT_PUBLIC_API_URL`),
      verified in the Network tab
- [ ] Any failure found is filed as a follow-up bug with the specific failure, linked below

## Result

_Fill in after running the manual pass: pass/fail, date, browser, and links to any follow-up
issues filed._
