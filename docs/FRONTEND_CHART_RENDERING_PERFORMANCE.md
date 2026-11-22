# Frontend: Chart Rendering Performance for Multi-Year Time Series

**Issue:** [#2838](https://github.com/Ndifreke000/stellar-insights/issues/2838)  
**Difficulty:** Hard  
**Estimated size:** L — roughly 1–2 weeks  

---

## Summary

Charts currently receive raw data points from the API. For a multi-year hourly series this is hundreds of thousands of points, each rendered as an SVG node. The browser freezes on these ranges. The fix has three parts: downsample on the server using LTTB before data leaves the wire, switch to Canvas rendering for dense series, and load data progressively as the user zooms.

---

## Problem

| File | Lines | Issue |
|---|---|---|
| `frontend/src/components/charts/` | — | SVG-based charts; no point reduction |
| `frontend/src/hooks/useAdvancedDataVisualization.ts` | 128 | Fetches raw series; passes full arrays to chart components |
| `frontend/src/lib/chart-utils.ts` | 71 | Utility helpers; no downsampling logic |

A 2-year hourly series produces ~17,500 points. SVG renders one DOM node per point. At 5+ years the DOM size causes browser-level jank and eventual OOM on mobile.

---

## Proposed Implementation

### 1. Server-side LTTB downsampling

LTTB (Largest-Triangle-Three-Buckets) is a perceptually lossless downsampling algorithm that preserves visual shape while drastically reducing point count. See the [original paper](https://skemman.is/bitstream/1946/15343/3/SS_MSthesis.pdf).

Add a `resolution` query parameter to time-series endpoints:

```
GET /api/v1/assets/:id/price-history?from=2020-01-01&to=2025-01-01&resolution=1500
```

`resolution` is the target number of output points. The backend samples the raw series down to that count using LTTB before serialising. Choose `resolution` on the client based on viewport width:

```ts
// frontend/src/lib/chart-utils.ts
export function targetResolution(viewportWidth: number): number {
  // 2 points per pixel is the visual limit; leave headroom for zoom
  return Math.min(viewportWidth * 2, 4000);
}
```

The backend returns the downsampled series plus a `sampled: true` flag and the original `total_points` count so the UI can show a "showing N of M points" label.

### 2. Canvas rendering for dense series

Switch from SVG to `<canvas>` when the point count exceeds a threshold (suggested: 2,000 points). Use a lightweight canvas chart library — [`uplot`](https://github.com/leeoniya/uPlot) is the best-fit choice: it renders 1M points in under 40 ms, has a small bundle (~40 KB), and exposes a low-level API that integrates with the existing hook layer.

```ts
// frontend/src/components/charts/TimeSeriesChart.tsx
import uPlot from 'uplot';

const CANVAS_THRESHOLD = 2_000;

export function TimeSeriesChart({ data }: Props) {
  if (data.points.length > CANVAS_THRESHOLD) {
    return <UPlotChart data={data} />;
  }
  return <SvgChart data={data} />;        // existing SVG path for short ranges
}
```

Keep the SVG path for short ranges (day/week views) where tooltip interactivity is richer.

### 3. Progressive loading on zoom

When the user zooms into a sub-range, fetch a fresh downsampled series for that range at a higher resolution:

```ts
// frontend/src/hooks/useAdvancedDataVisualization.ts
function useChartData(assetId: string, range: DateRange) {
  const resolution = targetResolution(window.innerWidth);
  return useQuery({
    queryKey: ['price-history', assetId, range, resolution],
    queryFn: () => fetchPriceHistory(assetId, { ...range, resolution }),
    staleTime: 60_000,
    placeholderData: keepPreviousData,   // show previous zoom level while loading
  });
}
```

This gives the user immediate feedback (previous zoom level) while higher-detail data loads in the background.

### 4. Benchmarks

Add a benchmark script at `frontend/benchmarks/chart-render.ts`:

```ts
// Measures render time for a 10-year hourly series at various resolutions
// Run with: npx tsx benchmarks/chart-render.ts
```

Add a CI performance budget test:

```ts
// frontend/src/__tests__/chart-performance.test.ts
test('10-year series renders within 500 ms', async () => {
  const start = performance.now();
  render(<TimeSeriesChart data={tenYearSeries} />);
  expect(performance.now() - start).toBeLessThan(500);
});
```

---

## Tasks

- [ ] Add `resolution` query parameter and LTTB downsampling to backend time-series endpoints
- [ ] Update `useAdvancedDataVisualization` to pass `resolution` derived from viewport width
- [ ] Integrate `uplot` for Canvas rendering above the 2,000-point threshold
- [ ] Implement progressive loading with `keepPreviousData` on zoom
- [ ] Add benchmark script and CI performance budget test
- [ ] Update docs / README where behaviour changes
- [ ] Add or update tests (see test plan)

---

## Test Plan

**LTTB correctness:** Given a known series, the downsampled output preserves peaks and troughs. Compare the max/min values of the input and output series — they must match.

```ts
test('LTTB preserves global max and min', () => {
  const result = lttb(largeTestSeries, 200);
  expect(Math.max(...result.map(p => p.y))).toBeCloseTo(Math.max(...largeTestSeries.map(p => p.y)));
  expect(Math.min(...result.map(p => p.y))).toBeCloseTo(Math.min(...largeTestSeries.map(p => p.y)));
});
```

**Performance budget:** 10-year hourly series renders in under 500 ms (CI-enforced).

**Additional cases:**
- A series below the 2,000-point threshold uses the SVG path, not Canvas.
- Zooming from 10-year to 1-month range triggers a new fetch with higher resolution.
- The `sampled: true` indicator is shown in the UI when the backend downsamples.

---

## Acceptance Criteria

- [ ] Multi-year ranges render in under 500 ms
- [ ] CI performance budget test is green
- [ ] CI is green (lint, type-check, tests)
- [ ] No new TODOs without a linked issue

---

## Implementation Notes

Write a design note in the PR covering:

- LTTB implementation location (backend Rust vs. a shared Wasm module) and the trade-off.
- How the `resolution` parameter interacts with server-side caching (the resolution value must be part of the cache key).
- Tooltip interaction model on the Canvas path — `uplot` tooltips require a custom cursor hook vs. SVG `onMouseOver`.

**Reference:** [LTTB downsampling algorithm](https://skemman.is/bitstream/1946/15343/3/SS_MSthesis.pdf)

---

## Getting Started

```bash
git clone https://github.com/Ndifreke000/stellar-insights.git
cd stellar-insights

# Backend
cd backend && cp .env.example .env && cargo run
cargo test

# Frontend
cd ../frontend && npm install && npm run dev

# SDKs
cd ../sdk/typescript && npm install && npm test
cd ../python && pip install -e ".[dev]" && pytest

# Contracts
cd ../../contracts && cargo test --workspace
```

Verify the core endpoint:

```bash
curl -s "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
```

---

## Definition of Done

- [ ] Code merged to `main` with tests passing
- [ ] All acceptance criteria checked
- [ ] PR description explains the change, links this issue (`Closes #2838`), and lists anything deferred
- [ ] Docs updated where user-facing or integrator-facing behaviour changed
