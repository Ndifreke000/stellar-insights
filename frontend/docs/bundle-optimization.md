# Frontend Bundle Size Optimization

## Overview

This document describes the bundle optimization strategy for the Stellar Insights frontend.

## Architecture

### Bundle Analyzer

The `@next/bundle-analyzer` package is configured in `next.config.ts` and activated with:

```bash
ANALYZE=true npm run build
```

This generates interactive treemaps in `.next/analyze/` showing the size of every module in each chunk.

### Bundle Budget Check

Run `node scripts/bundle-budget-check.mjs` after a build to verify all chunks stay within budget:

| Budget | Limit | Notes |
|--------|-------|-------|
| Per-asset | 500 KB (raw) | ~200 KB gzipped |
| Total initial load | 600 KB (raw) | ~200 KB gzipped |
| vendor chunk | 300 KB (raw) | Third-party libraries |
| charts chunk | 200 KB (raw) | recharts, d3-force-3d |
| animation chunk | 100 KB (raw) | framer-motion |

In CI, budget violations cause a non-zero exit code.

### Code Splitting Strategy

**Route-level lazy loading** (`src/lib/lazy-route.tsx`):
- Heavy page components are loaded via `dynamic()` with SSR disabled
- Loading skeletons are shown during chunk download
- Three helpers: `lazyRoute()`, `lazyChart()`, `lazyExport()`

**Webpack splitChunks** (in `next.config.ts`):
- `vendor` — all `node_modules` code
- `charts` — recharts, d3-force-3d, react-force-graph-2d
- `animation` — framer-motion
- `common` — shared modules used by 2+ chunks

**optimizePackageImports** (in `next.config.ts`):
- Tree-shaking for recharts, framer-motion, @stellar/stellar-sdk, d3-force-3d, react-force-graph-2d

### Image Optimization

- `next/image` with WebP and AVIF formats
- Responsive `deviceSizes` and `imageSizes` configured
- 30-day cache TTL for optimized images
- Remote patterns restricted to `*.stellar.org`

### Lighthouse CI

`.lighthouserc.js` enforces:
- Performance score ≥ 90
- LCP ≤ 2500ms
- CLS ≤ 0.1
- TBT ≤ 300ms
- FCP ≤ 1800ms
- Total resource size ≤ 500KB

Run with: `npm run audit:lighthouse`

## Verification

```bash
# Build and analyze
npm run build
npm run analyze

# Check bundle budgets
node scripts/bundle-budget-check.mjs

# Run Lighthouse CI
npm run audit:lighthouse
```
