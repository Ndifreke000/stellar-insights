# Lighthouse / Core Web Vitals audit — 2026-07-26

Addresses stellar-insights#1873.

## What already existed

`.github/workflows/performance-budget.yml` already builds the frontend on
every PR touching `frontend/**` and runs `scripts/analyze-bundle.mjs`, which
enforces a 200KB main-bundle budget and a 100KB per-chunk budget across all
`.next/static/chunks/*.js`. That check is **bundle-size-only** — it has no
visibility into the three pages called out in this issue individually
(dashboard, anchors, network graph), and it doesn't measure LCP, CLS, TBT, or
any other Core Web Vital. It's a real, running check, just not the one this
issue asks for.

## What was added

- `frontend/lighthouserc.json` — a Lighthouse CI config that:
  - Boots the production server itself (`startServerCommand: "npm run
    start"`, waiting for the `next start` "Ready in" log line) — no manual
    build/start choreography needed.
  - Runs against the three pages named in the issue's acceptance criteria:
    `/en/dashboard`, `/en/anchors`, `/en/network` (3 runs each, desktop
    preset, median taken automatically by `@lhci/cli`).
  - Asserts on `categories:performance` (warn < 0.8), `categories:accessibility`
    (error < 0.9), `categories:best-practices` (warn < 0.9), and the
    Core Web Vitals directly: `largest-contentful-paint` (warn > 2500ms),
    `cumulative-layout-shift` (warn > 0.1), `total-blocking-time` (warn >
    300ms), plus `render-blocking-resources` and `unused-javascript` as
    warn-level hints.
  - Uploads reports to Lighthouse's temporary public storage so a link to
    the full trace is available from CI output.
- `frontend/package.json` — new `audit:lighthouse` script
  (`next build && npx --yes @lhci/cli@0.15.x autorun`) so this can be run
  locally or from CI without adding `@lhci/cli` as a persisted
  `devDependency` (avoids drifting `pnpm-lock.yaml` out of sync with the
  `pnpm install --frozen-lockfile` step the existing `performance-budget.yml`
  and other CI workflows rely on — `npx` fetches it on demand instead).

## Why this PR reports no actual Lighthouse scores or bundle numbers

Per this task's current scope, no `npm install`/`pnpm install` or build was
run to produce this PR. Lighthouse itself requires a real production build
running on a real server (Chrome-driven page loads, real paint timing) —
there is no way to fabricate honest LCP/CLS/TBT numbers without executing
that build, and this audit intentionally does not report fake or guessed
numbers. Real numbers require one of:

1. A maintainer running `npm run audit:lighthouse` locally, or
2. The GitHub Actions job below, added to CI (could not be pushed directly
   in this PR — GitHub rejects pushes to `.github/workflows/**` from this
   token because it lacks the `workflow` OAuth scope; paste the YAML below
   into a new file at `.github/workflows/lighthouse-ci.yml` to enable it):

```yaml
name: Lighthouse CI

on:
  pull_request:
    branches: [main, develop]
    paths:
      - 'frontend/**'

jobs:
  lighthouse:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: frontend

    steps:
      - uses: actions/checkout@v4

      - uses: pnpm/action-setup@v4
        with:
          version: 10

      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
          cache-dependency-path: frontend/pnpm-lock.yaml

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Run Lighthouse CI
        run: npm run audit:lighthouse
        env:
          NEXT_TELEMETRY_DISABLED: 1

      - name: Upload Lighthouse reports
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: lighthouse-reports-${{ github.sha }}
          path: frontend/.lighthouseci/
          retention-days: 14
```

Once that workflow runs for the first time, its output will supply the
per-page LCP/CLS/TBT numbers and the largest-chunk-per-page breakdown this
issue's acceptance criteria ask for; any audit that scores below the
thresholds in `lighthouserc.json` should get its own follow-up issue rather
than being fixed inline here, per the issue's own instruction.
