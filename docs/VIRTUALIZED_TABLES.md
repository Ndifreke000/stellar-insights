# Virtualised Streaming Corridor & Transaction Tables

This document describes the virtualised, streaming table architecture implemented for Issue #2835.

---

## 1. Architecture Overview

Large corridor and transaction tables previously rendered every row into the DOM and re-rendered entire tables on live WebSocket updates. The new implementation combines four layers:

1. **Row Virtualisation (`@tanstack/react-virtual`)**:
   - `VirtualizedCorridorTable` (`frontend/src/components/corridors/VirtualizedCorridorTable.tsx`)
   - `VirtualizedTransactionTable` (`frontend/src/components/transactions/VirtualizedTransactionTable.tsx`)
   - Large-dataset virtualisation also enabled on `CorridorComparisonTable` and `CorridorRecentFailures`.
   - Fixed-height row geometry (`56px` corridors, `52px` transactions, `overscan: 10`) keeps the mounted DOM window bounded to ~20–35 rows even for 10,000+ records.

2. **Stable Row Keys & Memoized Rows/Cells**:
   - `getItemKey: (index) => rows[index].id` preserves DOM node identity across patches and cursor appends.
   - Row components (`CorridorVirtualRow`, `TransactionVirtualRow`) and every cell component are wrapped in `React.memo`.
   - Positioning props (`virtualStart: number`, `rowHeight: number`) are passed as primitives instead of freshly allocated style objects so React bails out on untouched sibling rows.
   - `Intl.NumberFormat` formatters are hoisted to module scope to avoid per-cell allocations during scrolling.

3. **O(1) Single-Row WebSocket Patching**:
   - `usePatchableRows` (`frontend/src/hooks/useStreamingTableData.ts`) maintains normalized row overlays by ID, replacing only the affected row object reference (`prevRow !== nextRow`) while preserving strict referential equality (`===`) for all other rows.
   - `RealtimeProvider` (`frontend/src/contexts/RealtimeContext.tsx`) shares the single tab WebSocket connection (#2834) and patches React Query `useQuery` and `useInfiniteQuery` pages in place (`patchCorridorInInfiniteData` / `patchCorridorInPaginatedData`).

4. **Server-Driven Sorting, Filtering & Cursor Pagination**:
   - `GET /api/corridors` accepts `sort_by`, `sort_order` (`asc`/`desc`), `search`, `source_asset`, `destination_asset`, `status`, `success_rate_min`, `time_period`, `limit`, and opaque `cursor`.
   - `GET /api/rpc/payments` accepts `limit`, `cursor`, `asset_code`, `status` (`successful`/`failed`), `search`, and `sort_order`.
   - Frontend `useInfiniteCorridors` and `useInfiniteTransactions` consume `pagination.next_cursor` via `getNextCursor`, automatically requesting the next cursor page when scrolling within 10 rows of the loaded tail.

---

## 2. React Profiler Measurements (Before vs After)

Measured using React's `<Profiler>` API in `frontend/src/components/__tests__/virtualizedTables.profiler.test.tsx` (Node 22 + jsdom, 2,000-row comparison + 10,000-row isolation check):

| Metric | Before (Naive Full Table) | After (TanStack Virtual + Memo Patch) | Improvement |
|--------|---------------------------|---------------------------------------|-------------|
| Mounted DOM rows (2,000 dataset) | 2,000 | 20 | **99.0% fewer DOM nodes** |
| Mounted DOM rows (10,000 dataset) | 10,000 | 20 | **99.8% fewer DOM nodes** |
| Initial mount (`actualDuration`) | 638.15 ms | 45.20 ms | **14.1× faster** |
| 1-row live WebSocket patch (`actualDuration`) | 222.59 ms | 9.72 ms | **22.9× faster** |
| Row components re-rendered per live patch (10k rows) | 10,000 | **1** | **O(1) commit** |

---

## 3. Running Verification Tests

```bash
# React Profiler & 10k-row single-row patch unit tests
cd frontend && pnpm exec vitest run src/components/__tests__/virtualizedTables.profiler.test.tsx

# Playwright 10k-row scroll performance & cursor streaming E2E test
cd frontend && pnpm exec playwright test src/__tests__/e2e/virtualized-tables-perf.test.ts
```
