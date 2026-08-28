# Responsive audit — dashboard / analytics / chart-heavy pages (#1870)

Audit date: 2026-07-26  
Breakpoints checked (static layout review + class inspection): **375px**, **768px**, **1024px**, **1440px**  
Pages: dashboard, corridors, anchors, network graph, calculator  

This issue asks for an audit and **follow-up bugs rather than inline fixes**. Findings below are ready to file as separate issues.

---

## Summary

| Page | Charts reflow? | Tables at 375px | Verdict |
|------|----------------|-----------------|---------|
| Dashboard | Mostly yes (`ResponsiveContainer` + `overflow-x-auto` on asset tables) | Horizontal scroll inside table containers | Pass with follow-ups |
| Corridors | Card grid `grid-cols-1 md:2 lg:3`; charts use `w-full` | N/A (cards) | Pass |
| Anchors | Sparkline charts in cards/table | Table `hidden lg:block overflow-x-auto`; cards on smaller viewports | Pass |
| Network graph | Canvas fills parent; parent uses fixed min heights | N/A | **Follow-up** |
| Calculator | Form stacks `flex-col md:flex-row` | N/A | Pass |

---

## Follow-up bugs to file

### BUG-1870-1 — Network Topology page cramped / overflow risk under 768px
- **Where:** `frontend/src/app/[locale]/network/page.tsx`
- **What:** Page uses `p-8`, header `text-4xl`, graph region `min-h-[500px]` + `h-[calc(100vh-320px)]`, and a dense glass stats strip. At **375px** the padding + min-height combination likely forces vertical overflow and squeezes the force-graph viewport; the stats strip does not collapse to a single column below `md`.
- **Expected:** Reduce padding on small screens (`p-4 sm:p-8`), lower `min-h` on mobile, stack stats `flex-col` under `sm`.

### BUG-1870-2 — LiquidityDepthCard / SettlementSpeedCard omit ResponsiveContainer sizing props
- **Where:** `frontend/src/components/dashboard/LiquidityDepthCard.tsx`, `SettlementSpeedCard.tsx`
- **What:** Sibling charts (`LiquidityChart`, `SettlementSpeedChart`) pass `width="100%" height="100%"`, but these card variants use bare `<ResponsiveContainer>` inside a styled height box. Recharts can fail to size correctly on first paint / narrow widths, causing clipped or zero-width plots at **375px–768px**.
- **Expected:** Pass explicit `width="100%" height="100%"` (or numeric height) like the other dashboard charts.

### BUG-1870-3 — CorridorComparisonTable pagination controls wrap awkwardly at 375px
- **Where:** `frontend/src/components/CorridorComparisonTable.tsx` (`DataTablePagination`)
- **What:** Table body correctly uses `overflow-x-auto`. Pagination mixes jump-to input + page-size select in a wrapping flex row; at **375px** controls remain usable but can push below the fold without sticky context, and the table + pagination do not share a single scroll region.
- **Expected:** Stack pagination actions full-width on `xs`, keep table scroll isolated (already mostly OK).

### BUG-1870-4 — Dashboard metric/chart grid density at 375px
- **Where:** `frontend/src/app/[locale]/dashboard/page.tsx` and `frontend/src/components/dashboard/*`
- **What:** Multiple chart cards stack correctly, but combined with navbar `ml-20` sidebar offset the content column is ~295px at 375px. Charts with `height={300}` (corridor-charts) remain readable; tooltips from Recharts may still overflow the viewport horizontally (library default).
- **Expected:** Constrain tooltip wrappers / use `allowEscapeViewBox={false}` on high-traffic charts; verify under real device QA.

---

## Confirmed good patterns (no bug)

1. **Anchors:** desktop table scrolls inside `overflow-x-auto`; mobile/tablet uses `AnchorCards` — matches “reflow to cards or scroll within container”.
2. **TopAssetsTable / TopAssetsCard:** `overflow-x-auto` on the table wrapper, not the page.
3. **Corridors list:** responsive card grid; no fixed-width SVG tables.
4. **Calculator:** single-column form on small screens.

---

## Out of scope for #1870

Do **not** fix the bugs above in the #1870 PR/commit — file them separately (or open issues from this doc) so this audit remains reviewable and closable on its own.
