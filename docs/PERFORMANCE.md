# Performance Standards

Performance budgets and targets for the PayRaider frontend.

## Budgets

| Metric | Budget | Enforcement |
|---|---|---|
| Main bundle (gzipped) | ≤ 200 KB | `scripts/analyze-bundle.mjs` + CI |
| Per-asset raw size | ≤ 500 KB | webpack `performance.hints` |
| Per-entrypoint raw size | ≤ 500 KB | webpack `performance.hints` |
| Largest Contentful Paint (LCP) | ≤ 2.5 s | Lighthouse / manual |
| First Input Delay (FID) | ≤ 100 ms | Lighthouse / manual |
| Cumulative Layout Shift (CLS) | ≤ 0.1 | Lighthouse / manual |
| Time to Interactive (TTI) | ≤ 3 s | Lighthouse / manual |
| Lighthouse Performance score | ≥ 90 | Manual / CI (optional) |

## CI enforcement

The [performance-budget workflow](../.github/workflows/performance-budget.yml) runs on every PR that touches `frontend/`. It:

1. Builds the Next.js app (`pnpm build`)
2. Runs `scripts/analyze-bundle.mjs` — fails if the main bundle exceeds 200 KB
3. webpack `performance.hints = 'error'` in CI — fails the build if any asset or entrypoint exceeds 500 KB raw

## Checking budgets locally

```bash
# Full build + bundle size report
cd frontend
npm run analyze

# Build only (webpack will warn on oversized assets)
npm run build
```

## Webpack budget config

`next.config.ts` sets `config.performance` in the webpack callback:

```ts
config.performance = {
  hints: process.env.CI ? 'error' : 'warning',
  maxAssetSize: 500 * 1024,
  maxEntrypointSize: 500 * 1024,
};
```

Locally this prints warnings; in CI it causes a hard build failure.

## Lighthouse

Run against a production build:

```bash
cd frontend
npm run build && npm start &
npx lighthouse http://localhost:3000 --output=json --output-path=lighthouse-report.json
```

Target scores: Performance ≥ 90, Accessibility ≥ 90, Best Practices ≥ 90, SEO ≥ 90.

## Keeping bundles small

- Heavy components (charts, graphs, animations) are lazy-loaded via `src/components/dynamic-imports.ts`.
- `optimizePackageImports` in `next.config.ts` enables tree-shaking for large libraries.
- Run `ANALYZE=true npm run build` to open the bundle visualiser and identify regressions.
- Prefer `lucide-react` named imports over barrel imports.
